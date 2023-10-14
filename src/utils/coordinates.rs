use core::ops::Add;
use core::ops::Sub;
use core::cmp::Ordering;

use adaptive_bezier::Vector2;
use ordered_float::NotNan;
use ordered_float::FloatIsNan;

use crate::geometry::Collection;
use crate::progress::ProgressObserver;
use crate::geometry::GDALGeometryWrapper;
use crate::geometry::Point;
use crate::utils::edge::Edge;
use crate::errors::CommandError;
use super::extent::Extent;
use crate::progress::WatchableIterator;

#[derive(Hash,Eq,PartialEq,Clone,Debug)]
pub(crate) struct Coordinates {
    pub(crate) x: NotNan<f64>,
    pub(crate) y: NotNan<f64>
}

impl Coordinates {

    pub(crate) fn to_tuple(&self) -> (f64,f64) {
        (*self.x,*self.y)
    }

    pub(crate) fn to_vector_2(&self) -> Vector2 {
        Vector2::new(*self.x,*self.y)
    }

    pub(crate) const fn new(x: NotNan<f64>, y: NotNan<f64>) -> Self {
        Self { x, y }
    }

    pub(crate) fn subtract(&self, other: &Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub(crate) fn normalized(&self) -> Self {
        let length = (self.x * self.x + self.y * self.y).sqrt();
        if length == 0.0 {
            Self::new(NotNan::from(0), NotNan::from(0))
        } else {
            Self::new(self.x / length, self.y / length)
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub(crate) fn multiply(&self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    pub(crate) fn abs(&self) -> f64 {
        // -- the absolute value for a point is the distance from 0, just as the absolute value of an integer is it's distance from 0.
        self.x.hypot(self.y.into_inner())
        // (x.hypot(y) = (x.powi(2) + y.powi(2)).sqrt();
    }

    pub(crate) fn perpendicular(&self, negate_second: bool) -> Self {
        if negate_second {
            Self::new(self.y,-self.x)
        } else {
            Self::new(-self.y,self.x)
        }
    }

    pub(crate) fn distance(&self, other: &Self) -> f64 {
        // FUTURE: Is there some way to improve this by using the hypot function? 
        (other.x.into_inner() - self.x.into_inner()).hypot(other.y.into_inner() - self.y.into_inner())
        // (x.hypot(y) = (x.powi(2) + y.powi(2)).sqrt();
        // (other.x - self.x).hypot(other.y - self.y) = ((other.x - self.x).powi(2) + (other.y - self.y).powi(2)).sqrt() 
    }

    pub(crate) fn middle_point_between(&self, other: &Self) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }

    }

    pub(crate) fn interpolate_at_longitude(&self, other: &Self, longitude: f64) -> Result<Self,CommandError> {
        /*
        (y - y0)/(x - x0) = (y1 - y0)/(x1 - x0)
        (y - y0) = ((y1 - y0)/(x1 - x0))*(x - x0)
        y = ((y1 - y0)/(x1 - x0))*(x - x0) + y0        
         */
        let longitude = NotNan::try_from(longitude)?;
        let y = self.y + ((longitude - self.x)*((other.y - self.y)/(other.x - self.x)));
        Ok(Self {
            x: longitude,
            y
        })
    }

    pub(crate) fn create_geometry(&self) -> Result<Point,CommandError> {
        Point::new(self.x.into(), self.y.into())
    }

    pub(crate) fn circumcenter(points: (&Self,&Self,&Self)) -> Self {
        // Finding the Circumcenter: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2

        let (a,b,c) = points;
        let d = (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) * 2.0;
        let d_recip = d.recip();
        let (ax2,ay2,bx2,by2,cx2,cy2) = ((a.x*a.x),(a.y*a.y),(b.x*b.x),(b.y*b.y),(c.x*c.x),(c.y*c.y));
        let (ax2_ay2,bx2_by2,cx2_cy2) = (ax2+ay2,bx2+by2,cx2+cy2);
        let ux = ((ax2_ay2)*(b.y - c.y) + (bx2_by2)*(c.y - a.y) + (cx2_cy2)*(a.y - b.y)) * d_recip;
        let uy = ((ax2_ay2)*(c.x - b.x) + (bx2_by2)*(a.x - c.x) + (cx2_cy2)*(b.x - a.x)) * d_recip;

        (ux,uy).into()

    }

    pub(crate) fn order_clockwise(a: &Self, b: &Self, center: &Self) -> Ordering
    {

        let a_run = a.x - center.x;
        let b_run = b.x - center.x;

        // yes, is_sign_positive does weird things if we have a -0, but I don't think that's possible with simple subtraction
        // and it would just sort them one way or the other, which I feel is probably the right way anyway.
        match (a_run.is_sign_positive(),b_run.is_sign_positive()) {
            (true, false) => {
                // a is in the east, b is in the west. a is closer to north and less than b.
                Ordering::Less
            },
            (false, true) => {
                // a is in the west, b is in the east, a is further from north and greater than b.
                Ordering::Greater
            },
            (east, _) => { // both are in the same east-west half
                let a_rise = a.y - center.y;
                let b_rise = b.y - center.y;

                match (a_rise.is_sign_positive(),b_rise.is_sign_positive()) {
                    (true, false) => {
                        // a is in the north and b is in the south
                        if east {
                            // a is in northeast and b is in southeast
                            Ordering::Less
                        } else {
                            // a is in northwest and b is in southwest
                            Ordering::Greater
                        }
                    },
                    (false, true) => {
                        // a is in the south and b is in the north, a is further from north
                        if east {
                            // a is in the southeast and b is in the northeast
                            Ordering::Greater
                        } else {
                            // a is in southwest and b is in northwest
                            Ordering::Less
                        }
                    },
                    (_, _) => {
                        // both are in the same quadrant. Compare the cross-product.
                        // NOTE: I originally compared the slope, but the stackoverflow accepted solution used something like the following formula 
                        // and called it a cross-product. Assuming that is the same, it's the same thing as comparing the slope. Why?
                        // (Yes, I know a mathematician might not need this, but I'll explain my reasoning to future me)
                        // A slope is a fraction. To compare two fractions, you have to do the same thing that you do when adding fractions:
                        //   A/B cmp C/D = (A*D)/(B*D) cmp (C*B)/(B*D)
                        // For a comparison, we can then remove the denominators:
                        //   (A*D) cmp (B*D) 
                        // and that's the same thing the solution called a cross-product. 
                        // So, in order to avoid a divide by 0, I'm going to use that instead of slope.
                        match ((a_run) * (b_rise)).cmp(&((b_run) * (a_rise))) {
                            Ordering::Equal => {
                                // The slopes are the same, compare the distance from center. The shorter distance should be closer to the beginning.
                                let a_distance = (a_run) * (a_run) + (a_rise) * (a_rise);
                                let b_distance = (b_run) * (b_run) + (b_rise) * (b_rise);
                                a_distance.cmp(&b_distance)
                            },
                            slope_compare => {
                                // both are in the same quadrant now, but the slopes are not the same, we can just return the result of slope comparison:
                                // in the northeast quadrant, a lower positive slope means it is closer to east and further away.
                                // in the southeast quadrant, a lower negative slope means it is closer to south and further away.
                                // in the southwest quadrant, a lower positive slope means it is closer to west and further away.
                                // in the northwest quadrant, a lower negative slope means it is closer to north and further away from the start.
                                slope_compare
                            }
                        }

                    },
                }


            },
        }

    }

    pub(crate) const fn to_ordered_tuple(&self) -> (NotNan<f64>, NotNan<f64>) {
        (self.x,self.y)
    }

    pub(crate) fn semi_random_toggle(&self) -> bool {
        // Sometimes I want to do something based on the point, such as switch the direction of a curve, in 
        // a way that looks random, but is reproducible. 
        // The easiest way I can think of is basically to base it off of whether the integral part of a value is even.
        self.x.rem_euclid(2.0) < 1.0
    }

    pub(crate) fn to_edge(&self, extents: &Extent, edge: &Edge) -> Result<Self,CommandError> {
        let (x,y) = match edge {
            Edge::North => (self.x,NotNan::try_from(extents.north())?),
            Edge::Northeast => (NotNan::try_from(extents.east())?,NotNan::try_from(extents.north())?),
            Edge::East => (NotNan::try_from(extents.east())?,self.y),
            Edge::Southeast => (NotNan::try_from(extents.east())?,NotNan::try_from(extents.south)?),
            Edge::South => (self.x,NotNan::try_from(extents.south)?),
            Edge::Southwest => (NotNan::try_from(extents.west)?,NotNan::try_from(extents.south)?),
            Edge::West => (NotNan::try_from(extents.west)?,self.y),
            Edge::Northwest => (NotNan::try_from(extents.west)?,NotNan::try_from(extents.north())?),
        };
        Ok(Self {
            x,
            y
        })
    }

    pub(crate) fn longitude_across_antimeridian<Float>(source_x: Float, relative_x: &Float) -> Float 
    where Float: PartialOrd + Sub<f64, Output = Float> + Add<f64, Output = Float> {
        if &source_x > relative_x {
            // it's across to the west, on the far east longitudes, so shift it around to the west
            source_x - 360.0
        } else {
            // otherwise it's across to the east, so shift it around to the east
            source_x + 360.0
        }
    }

    pub(crate) fn across_antimeridian(&self, relative_to: &Self) -> Self {
        Self {
            x: Self::longitude_across_antimeridian(self.x, &relative_to.x),
            y: self.y
        }
    }

    pub(crate) fn clip_point_vec_across_antimeridian(line: Vec<Self>, extent: &Extent) -> Result<Vec<Vec<Self>>,CommandError> {

        #[derive(PartialEq,Debug)]
        pub(crate) enum Location {
            ToWest,
            InExtent,
            ToEast,
        }

        let west = NotNan::new(extent.west)?;
        let east = NotNan::new(extent.east())?;

        let fix_point = |point: &Self, location: &Location| {
            match location {
                Location::ToWest => Self {
                    x: point.x + 360.0,
                    y: point.y
                },
                Location::InExtent => point.clone(),
                Location::ToEast => Self {
                    x: point.x - 360.0,
                    y: point.y
                },
            }
        };


        let get_location = |point: &Self| {
            if point.x < west {
                Location::ToWest
            } else if point.x > east {
                Location::ToEast
            } else {
                Location::InExtent
            }

        };


        let mut result = Vec::new();
        let mut segment = Vec::new();

        let mut line = line.into_iter();
        if let Some(mut previous) = line.next() {
            let mut previous_location = get_location(&previous);
            assert_eq!(previous_location,Location::InExtent);
            segment.push(fix_point(&previous,&previous_location));
            for next in line {
                let next_location = get_location(&next);
                let mid_point = match (&previous_location,&next_location) {
                    (Location::ToWest, Location::InExtent) |
                    (Location::InExtent, Location::ToWest) => Some(previous.interpolate_at_longitude(&next, extent.west)?),

                    (Location::InExtent, Location::ToEast) |
                    (Location::ToEast, Location::InExtent) => Some(previous.interpolate_at_longitude(&next, extent.east())?),

                    (Location::ToWest, Location::ToEast) |
                    (Location::ToEast, Location::ToWest) => panic!("Points should all be anchored on one side."),

                    (Location::ToWest, Location::ToWest) |
                    (Location::InExtent, Location::InExtent) |
                    (Location::ToEast, Location::ToEast) => None, // no split here
                };
                if let Some(mid_point) = mid_point {
                    // it's time to cut it
                    segment.push(fix_point(&mid_point,&previous_location));
                    result.push(segment);
                    segment = Vec::new();
                    segment.push(fix_point(&mid_point,&next_location));
                }
                segment.push(fix_point(&next,&next_location));
                previous = next;
                previous_location = next_location;
            }
        }
        result.push(segment);

        Ok(result)
    }

}

impl TryFrom<(f64,f64,f64)> for Coordinates {

    type Error = FloatIsNan;

    fn try_from(value: (f64,f64,f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: NotNan::new(value.0)?,
            y: NotNan::new(value.1)?
        })
    }
}

impl From<(NotNan<f64>,NotNan<f64>)> for Coordinates {

    fn from(value: (NotNan<f64>,NotNan<f64>)) -> Self {
        Self {
            x: value.0,
            y: value.1
        }
    }
}

impl TryFrom<(f64,f64)> for Coordinates {

    type Error = FloatIsNan;

    fn try_from(value: (f64,f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: value.0.try_into()?,
            y: value.1.try_into()?
        })
    }
}

impl Sub for &Coordinates {
    type Output = Coordinates;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add for &Coordinates {
    type Output = Coordinates;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

pub(crate) trait ToGeometryCollection<Geometry: GDALGeometryWrapper> {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Collection<Geometry>,CommandError>;
}

// FUTURE: Implement traits so I can just use collect? But then I can't use progress observer.
impl<Iter: Iterator<Item=Result<Point,CommandError>>> ToGeometryCollection<Point> for Iter {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Collection<Point>,CommandError> {
        let mut result = Collection::new()?;
        for geometry in self.watch(progress,"Collecting points.","Points collected.") {
            result.push_item(geometry?)?;
        }
        Ok(result)
    }


}

#[cfg(test)]
mod test {

    use super::Coordinates;
    use super::Extent;
    use ordered_float::NotNan;

    #[test]
    fn test_clip_point_vec_across_antimeridian() {

        let line = vec![
            Coordinates::new(NotNan::try_from(178.1579399076034).unwrap(), NotNan::try_from(4.993378037130952).unwrap()),
            Coordinates::new(NotNan::try_from(-179.03189170475136).unwrap()+360.0, NotNan::try_from(5.381816241032141).unwrap()),
        ];

        let clipped = Coordinates::clip_point_vec_across_antimeridian(line, &Extent {
            height: 180.0,
            width: 360.0,
            south: -90.0,
            west: -180.0,
        }).unwrap();

        assert_eq!(clipped,vec![
            vec![Coordinates::new(NotNan::try_from(178.1579399076034).unwrap(), NotNan::try_from(4.993378037130952).unwrap()),
                 Coordinates::new(NotNan::try_from(180.0).unwrap(), NotNan::try_from(5.2479985491665895).unwrap())],
            vec![Coordinates::new(NotNan::try_from(-180.0).unwrap(), NotNan::try_from(5.2479985491665895).unwrap()),
                 Coordinates::new(NotNan::try_from(-179.03189170475136).unwrap(), NotNan::try_from(5.381816241032141).unwrap())]
        ])



    }
}