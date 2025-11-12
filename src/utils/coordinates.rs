use core::ops::Add;
use core::ops::Sub;
use core::cmp::Ordering;

use adaptive_bezier::Vector2;
use ordered_float::NotNan;
use ordered_float::FloatIsNan;
use geo::algorithm::HaversineDistance as _;
use geo::algorithm::HaversineIntermediate as _;
use geo::algorithm::HaversineBearing as _;
use geo::algorithm::Centroid as _;
use angular_units::Deg;
use angular_units::Angle as _;
use geo::polygon;


use crate::geometry::Collection;
use crate::progress::ProgressObserver;
use crate::geometry::GDALGeometryWrapper;
use crate::geometry::Point;
use crate::utils::edge::Edge;
use crate::errors::CommandError;
use super::extent::Extent;
use crate::progress::WatchableIterator as _;
use crate::utils::world_shape::WorldShape;

#[derive(Hash,Eq,PartialEq,Clone,Debug)]
pub(crate) struct Coordinates {
    x: NotNan<f64>,
    y: NotNan<f64>
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

    // normalizing creates a point on a unit circle in the same direction from 0,0. This function is used by bezierify operations for curves.
    // I don't think I need to worry about WorldShape for this, since it will still make nice beziers even if we don't. However, what if I
    // end up using it in a different algorithm?
    // FUTURE: Do I need to worry about WorldShape with this one?
    pub(crate) fn normalized(&self) -> Self {
        let length = self.abs();//(self.x * self.x + self.y * self.y).sqrt();
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

    // FUTURE: Do I need to worry about WorldShape with this?
    pub(crate) fn abs(&self) -> f64 {
        // -- the absolute value for a point is the distance from 0, just as the absolute value of an integer is it's distance from 0.
        self.x.hypot(self.y.into_inner())
        // (x.hypot(y) = (x.powi(2) + y.powi(2)).sqrt();
    }

    // This is used to find a perpendicular for a normalized coordinate in bezier curves. I don't think I need to worry about WorldShape
    // for this.
    // FUTURE: Do I need to worry about WorldShape with this?
    pub(crate) fn perpendicular(&self, negate_second: bool) -> Self {
        if negate_second {
            Self::new(self.y,-self.x)
        } else {
            Self::new(-self.y,self.x)
        }
    }

    pub(crate) fn spherical_distance(&self, other: &Self) -> f64 {
        let this: geo::Point = self.into();
        let other: geo::Point = other.into();
        this.haversine_distance(&other)
    }

    pub(crate) fn distance(&self, other: &Self) -> f64 {
        (other.x.into_inner() - self.x.into_inner()).hypot(other.y.into_inner() - self.y.into_inner())
        // (x.hypot(y) = (x.powi(2) + y.powi(2)).sqrt();
        // (other.x - self.x).hypot(other.y - self.y) = ((other.x - self.x).powi(2) + (other.y - self.y).powi(2)).sqrt() 
    }

    pub(crate) fn shaped_distance(&self, other: &Self, shape: &WorldShape) -> f64 {
        match shape {
            WorldShape::Cylinder => self.distance(other),
            WorldShape::Sphere => self.spherical_distance(other)
        }
    }

    pub(crate) fn spherical_middle_point_between(&self, other: &Self) -> Result<Self,CommandError> {
        let this: geo::Point = self.into();
        let other: geo::Point = other.into();
        let result = this.haversine_intermediate(&other,0.5);
        Ok(result.try_into()?)
    }

    pub(crate) fn middle_point_between(&self, other: &Self) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }        
    }

    pub(crate) fn shaped_middle_point_between(&self, other: &Self, shape: &WorldShape) -> Result<Self,CommandError> {
        match shape {
            WorldShape::Cylinder => Ok(self.middle_point_between(other)),
            WorldShape::Sphere => self.spherical_middle_point_between(other)
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

    pub(crate) fn spherical_circumcenter(points: (&Self,&Self,&Self)) -> Result<Self,CommandError> {

        // FUTURE: The code after this calculates an actual spherical circumcenter, but it doesn't work. See there. For now,
        // I think that a simple planar centroid might make more sense anyway, since the tiles aren't going to be big enough to notice weirdness,
        // and I think it will look better than a circumcenter anyway.
        let (a,b,c) = points;
        // sort the points clockwise. There are only three points, which means there are only two orders: a,b,c and a,c,b. 
        // It doesn't matter where you start, so b,c,a is the same as a,b,c and c,b,a is the same as a,b,c. Try it out and tell me
        // if I'm wrong. That means I only have to compare the a and b ordering as clockwise around c, and if it is not, then swap a and b.
        // if you can get a different order out of that. 
        let (a,b,c) = match Self::order_clockwise(a, b, c) {
            Ordering::Less | Ordering::Equal => (a,b,c), // ordering is correct
            Ordering::Greater => (b,a,c), // ordering is incorrect, so swap to re-order.
        };
        let polygon = polygon![
            (x: a.x.into_inner(), y: a.y.into_inner()),
            (x: b.x.into_inner(), y: b.y.into_inner()),
            (x: c.x.into_inner(), y: c.y.into_inner()),
            (x: a.x.into_inner(), y: a.y.into_inner()),
        ];
        let centroid = polygon.centroid().expect("Why wouldn't a triangle have a centroid?");
        Ok(Self::try_from((centroid.x(),centroid.y()))?)



        /*
        // FUTURE: The following produces tiles that spread all over the place. I think I'm getting some antipodal values from
        // this. I tried sorting the points in clockwise and counterclockwise order before calculation, as suggested in the 
        // source, but that didn't fix it. Maybe there's something else going on.

        // https://web.archive.org/web/20171023010630/http://mathforum.org/library/drmath/view/68373.html
        macro_rules! to_cartesian {
            ($point: ident) => {{
                let lon_r = $point.x.to_radians();
                let lat_r = $point.y.to_radians();
                //x = cos(lon)*cos(lat)
                let x = lon_r.cos() * lat_r.cos();
                //y = sin(lon)*cos(lat)
                let y = lon_r.sin() * lat_r.cos();
                //z = sin(lat)                
                let z = lat_r.sin();
                (x,y,z)
            }};
        }

        let (a,b,c) = points;
        let (x1,y1,z1) = to_cartesian!(a);
        let (x2,y2,z2) = to_cartesian!(b);
        let (x3,y3,z3) = to_cartesian!(c);

        // cross-product
        //let n = (B-A) * (C-A);
        //let n = (x2-x1, y2-y1, z2-z1) * (x3-x1, y3-y1, z3-z1);
        let (xn,yn,zn) = (((y2-y1)*(z3-z1))-((z2-z1)*(y3-y1)),
                          ((z2-z1)*(x3-x1))-((x2-x1)*(z3-z1)),
                          ((x2-x1)*(y3-y1))-((y2-y1)*(x3-x1)));

        // radius of n, which is needed for lat/lon
        let r = (xn.powi(2) + yn.powi(2) + zn.powi(2)).sqrt();

        let lat = (zn/r).asin().to_degrees();
        let lon = yn.atan2(xn).to_degrees();

        Ok(Self::try_from((lon,lat))?)
        */

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

    pub(crate) fn shaped_circumcenter(points: (&Self,&Self,&Self), shape: &WorldShape) -> Result<Self,CommandError> {
        match shape {
            WorldShape::Cylinder => Ok(Self::circumcenter(points)),
            WorldShape::Sphere => Self::spherical_circumcenter(points)
        }
    }

    // FUTURE: I believe that despite the distortion, the order of points by angle will still be the same on a sphere. Maybe have to revisit this someday?
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
            Edge::Southeast => (NotNan::try_from(extents.east())?,NotNan::try_from(extents.south())?),
            Edge::South => (self.x,NotNan::try_from(extents.south())?),
            Edge::Southwest => (NotNan::try_from(extents.west())?,NotNan::try_from(extents.south())?),
            Edge::West => (NotNan::try_from(extents.west())?,self.y),
            Edge::Northwest => (NotNan::try_from(extents.west())?,NotNan::try_from(extents.north())?),
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

        let west = NotNan::new(extent.west())?;
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
                    (Location::InExtent, Location::ToWest) => Some(previous.interpolate_at_longitude(&next, extent.west())?),

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

    pub(crate) fn shaped_bearing(&self, neighbor_site: &Self, world_shape: &WorldShape) -> Deg<f64> {
        match world_shape {
            WorldShape::Cylinder => self.bearing(neighbor_site),
            WorldShape::Sphere => self.spherical_bearing(neighbor_site)
        }
    }

    fn spherical_bearing(&self, neighbor_site: &Self) -> Deg<f64> {
        let this: geo::Point = self.into();
        let other: geo::Point = neighbor_site.into();
        // TODO: Need to test whether its in the same ranges, with the name "bearing" it should be
        Deg(this.haversine_bearing(other))
    }

    fn bearing(&self, neighbor_site: &Self) -> Deg<f64> {
        // needs to be clockwise, from the north, with a value from 0..360

        // the result below is counter clockwise from the east, but also if it's in the south it's negative.
        let counter_clockwise_from_east = Deg(((neighbor_site.y-self.y).atan2(neighbor_site.x.into_inner()-self.x.into_inner()).to_degrees()).round());
        // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
        let clockwise_from_north = Deg(450.0) - counter_clockwise_from_east; 

        // And, the Deg structure allows me to normalize it
        clockwise_from_north.normalize()

    }
    
    pub(crate) const fn x(&self) -> NotNan<f64> {
        self.x
    }
    
    pub(crate) const fn y(&self) -> NotNan<f64> {
        self.y
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

impl From<&Coordinates> for geo_types::Point {
    fn from(value: &Coordinates) -> Self {
        Self::new(value.x.into_inner(),value.y.into_inner())
    }
}

impl TryFrom<geo_types::Point> for Coordinates {

    type Error = FloatIsNan;

    fn try_from(value: geo_types::Point) -> Result<Self, Self::Error> {
        Ok(Self::new(NotNan::new(value.0.x)?,NotNan::new(value.0.y)?))
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

        let clipped = Coordinates::clip_point_vec_across_antimeridian(line, &Extent::from_height_width_south_west(
            180.0,
            360.0,
            -90.0,
            -180.0,
        )).unwrap();

        assert_eq!(clipped,vec![
            vec![Coordinates::new(NotNan::try_from(178.1579399076034).unwrap(), NotNan::try_from(4.993378037130952).unwrap()),
                 Coordinates::new(NotNan::try_from(180.0).unwrap(), NotNan::try_from(5.2479985491665895).unwrap())],
            vec![Coordinates::new(NotNan::try_from(-180.0).unwrap(), NotNan::try_from(5.2479985491665895).unwrap()),
                 Coordinates::new(NotNan::try_from(-179.03189170475136).unwrap(), NotNan::try_from(5.381816241032141).unwrap())]
        ])

    }

}