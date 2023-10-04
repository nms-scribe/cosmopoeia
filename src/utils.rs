use core::hash::Hash;
use core::str::FromStr;
use core::fmt::Display;
use core::cmp::Ordering; 
use core::ops::Sub;
use core::ops::Add;


use ordered_float::NotNan;
use ordered_float::FloatIsNan;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use rand_distr::uniform::SampleUniform;
use serde::Deserialize;
use serde::Serialize;
use adaptive_bezier::Vector2;
use angular_units::Deg;


use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::commands::RandomSeedArg;
use crate::geometry::LinearRing;
use crate::geometry::Polygon;
use crate::geometry::Point as GeoPoint;
use crate::geometry::Collection;
use crate::geometry::GDALGeometryWrapper;
use crate::geometry::VariantArealGeometry;
use crate::impl_simple_serde_tagged_enum;

pub(crate) fn random_number_generator(arg: &RandomSeedArg) -> StdRng {
    let seed = if let Some(seed) = arg.seed {
        seed
    } else {
        let mut seeder = StdRng::from_entropy();
        let seed = seeder.gen::<u64>();
        println!("Using random seed {seed}");
        seed
    };
    StdRng::seed_from_u64(seed)
}

pub(crate) trait RandomNth<ItemType> {

    fn choose<Random: Rng>(&mut self, rng: &mut Random) -> Option<ItemType>;

}

impl<ItemType, IteratorType: Iterator<Item=ItemType>> RandomNth<ItemType> for IteratorType {

    fn choose<Random: Rng>(&mut self, rng: &mut Random) -> Option<ItemType> {

        // FUTURE: I really wish size_hint was a trait that iterators could implement, so I could require it to exist for this to work.
        if let Some(len) = self.size_hint().1 {
            self.nth(rng.gen_range(0..len))
        } else {
            None
        }
    }
}



pub(crate) trait RandomIndex<ItemType> {

    fn choose<Random: Rng>(&self, rng: &mut Random) -> &ItemType;

    fn choose_index<Random: Rng>(&self, rng: &mut Random) -> usize;

    fn choose_biased_index<Random: Rng>(&self, rng: &mut Random, min: usize, max: usize, ex: i32) -> usize;

/*


    * biased(min,max,ex):
  -- generates a random number between min and max the leans towards the beginning
  * (min + ((max - min) * random(0..1).pow(ex))).round()
 */    
}

impl<ItemType> RandomIndex<ItemType> for [ItemType] {

    fn choose<Random: Rng>(&self, rng: &mut Random) -> &ItemType  {
        &self[rng.gen_range(0..self.len())] 
    }

    fn choose_index<Random: Rng>(&self, rng: &mut Random) -> usize {
        rng.gen_range(0..self.len())
    }

    fn choose_biased_index<Random: Rng>(&self, rng: &mut Random, min: usize, max: usize, ex: i32) -> usize {
        min + ((max - min) * rng.gen_range::<f64,_>(0.0..1.0).powi(ex).floor() as usize).clamp(0,self.len()-1)
    }

}

impl<ItemType> RandomIndex<ItemType> for Vec<ItemType> {
    fn choose<Random: Rng>(&self, rng: &mut Random) -> &ItemType  {
        &self[rng.gen_range(0..self.len())] 
    }

    fn choose_index<Random: Rng>(&self, rng: &mut Random) -> usize {
        rng.gen_range(0..self.len())
    }

    fn choose_biased_index<Random: Rng>(&self, rng: &mut Random, min: usize, max: usize, ex: i32) -> usize {
        min + ((max - min) * rng.gen_range::<f64,_>(0.0..1.0).powi(ex).floor() as usize).clamp(0,self.len()-1)
    }

}

impl<ItemType, const N: usize> RandomIndex<ItemType> for [ItemType; N] {
    fn choose<Random: Rng>(&self, rng: &mut Random) -> &ItemType  {
        &self[rng.gen_range(0..self.len())] 
    }

    fn choose_index<Random: Rng>(&self, rng: &mut Random) -> usize {
        rng.gen_range(0..self.len())
    }

    fn choose_biased_index<Random: Rng>(&self, rng: &mut Random, min: usize, max: usize, ex: i32) -> usize {
        min + ((max - min) * rng.gen_range::<f64,_>(0.0..1.0).powi(ex).floor() as usize).clamp(0,self.len()-1)
    }

}


#[derive(Clone)]
pub(crate) struct Extent {
    pub(crate) height: f64,
    pub(crate) width: f64,
    pub(crate) south: f64,
    pub(crate) west: f64,
}

impl Extent {

    pub(crate) fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        let width = east - west;
        let height = north - south;
        Self { 
            height, 
            width, 
            south, 
            west 
        }
    }

    pub(crate) const fn new_with_dimensions(west: f64, south: f64, width: f64, height: f64) -> Self {
        Self {
            height,
            width,
            south,
            west,
        }
    }

    pub(crate) fn contains(&self,point: &Point) -> bool {
        let x = point.x.into_inner();
        let y = point.y.into_inner();
        (x >= self.west) &&
           (x <= (self.west + self.width)) &&
           (y >= self.south) &&
           (y <= (self.south + self.height))

    }

    pub(crate) fn is_extent_on_edge(&self, extent: &Self) -> Result<Option<Edge>,CommandError> {
        let north = extent.north();
        let east = extent.east();
        let mut edge: Option<Edge> = None;
        for (x,y) in [(extent.west,extent.south),(extent.west,north),(east,north),(east,extent.south)] {
            if let Some(point_edge) = self.is_tuple_on_edge(x,y) {
                if let Some(previous_edge) = edge {
                    edge = Some(point_edge.combine_with(previous_edge)?);
                } else {
                    edge = Some(point_edge)
                }
            } // else keep previous edge
        }
        Ok(edge)
    }

    pub(crate) fn is_tuple_on_edge(&self, x: f64, y: f64) -> Option<Edge> {
        let x_order = if x <= self.west {
            Ordering::Less
        } else if x >= (self.east()) {
            Ordering::Greater
        } else {
            Ordering::Equal
        };

        let y_order = if y <= self.south {
            Ordering::Less
        } else if y >= (self.north()) {
            Ordering::Greater
        } else {
            Ordering::Equal
        };

        match (x_order,y_order) {
            (Ordering::Less, Ordering::Less) => Some(Edge::Southwest),
            (Ordering::Less, Ordering::Equal) => Some(Edge::West),
            (Ordering::Less, Ordering::Greater) => Some(Edge::Northwest),
            (Ordering::Equal, Ordering::Less) => Some(Edge::South),
            (Ordering::Equal, Ordering::Equal) => None,
            (Ordering::Equal, Ordering::Greater) => Some(Edge::North),
            (Ordering::Greater, Ordering::Less) => Some(Edge::Southeast),
            (Ordering::Greater, Ordering::Equal) => Some(Edge::East),
            (Ordering::Greater, Ordering::Greater) => Some(Edge::Northeast),
        }
    }

    pub(crate) fn is_off_edge(&self, point: &Point) -> Option<Edge> {
        let (x,y) = point.to_tuple();
        self.is_tuple_on_edge(x, y)

        
    }

    pub(crate) fn create_polygon(&self) -> Result<Polygon,CommandError> {
        let vertices = vec![
            (self.west,self.south),
            (self.west,self.south+self.height),
            (self.west+self.width,self.south+self.height),
            (self.west+self.width,self.south),
            (self.west,self.south),
        ];
        let ring = LinearRing::from_vertices(vertices)?;
        Polygon::from_rings([ring])
    }

    pub(crate) fn create_boundary_geometry(&self) -> Result<VariantArealGeometry, CommandError> {
        let north = self.north();
        let east = self.east();
        let west = self.west;
        let south = self.south;
        let mut border_points = Vec::new();
        border_points.push((west,south));
        for y in south.ceil() as usize..north.ceil() as usize {
            border_points.push((west,y as f64))
        }
        border_points.push((west,north));
        for x in west.ceil() as usize..east.floor() as usize {
            border_points.push((x as f64,north))
        }
        border_points.push((east,north));
        for y in north.ceil() as usize..south.floor() as usize {
            border_points.push((east,y as f64))
        }
        border_points.push((east,south));
        for x in east.ceil() as usize..west.floor() as usize {
            border_points.push((x as f64,south))
        }
        border_points.push((west,south));
        let ring = LinearRing::from_vertices(border_points)?;
        let ocean = Polygon::from_rings([ring])?;
        Ok(VariantArealGeometry::Polygon(ocean))
    }    

    pub(crate) fn east(&self) -> f64 {
        self.west + self.width
    }

    pub(crate) fn north(&self) -> f64 {
        self.south + self.height
    }

    pub(crate) fn wraps_latitudinally(&self) -> bool {
        self.width == 360.0
    }

    pub(crate) fn reaches_south_pole(&self) -> bool {
        self.south == -90.0
    }

    pub(crate) fn reaches_north_pole(&self) -> bool {
        self.north() == 90.0
    }

}


pub(crate) trait ToGeometryCollection<Geometry: GDALGeometryWrapper> {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Collection<Geometry>,CommandError>;
}

// FUTURE: Implement traits so I can just use collect? But then I can't use progress observer.
impl<Iter: Iterator<Item=Result<GeoPoint,CommandError>>> ToGeometryCollection<GeoPoint> for Iter {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Collection<GeoPoint>,CommandError> {
        let mut result = Collection::new()?;
        for geometry in self.watch(progress,"Collecting points.","Points collected.") {
            result.push_item(geometry?)?;
        }
        Ok(result)
    }


}

#[derive(Serialize,Deserialize,Clone,PartialEq,Eq,Hash,PartialOrd,Ord,Debug)]
pub enum Edge {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest
}

impl Edge {
    
    pub(crate) fn combine_with(&self, edge: Self) -> Result<Self,CommandError> {
        match (&edge,self) {
            (Self::North, Self::Northeast) |
            (Self::Northeast, Self::North) |
            (Self::East, Self::North) |
            (Self::East, Self::Northeast) |
            (Self::Northeast, Self::East) |
            (Self::North, Self::East) => Ok(Self::Northeast),
            (Self::North, Self::West) |
            (Self::West, Self::Northwest) |
            (Self::Northwest, Self::North) |
            (Self::Northwest, Self::West) |
            (Self::West, Self::North) |
            (Self::North, Self::Northwest) => Ok(Self::Northwest),
            (Self::East, Self::South) |
            (Self::Southeast, Self::East) |
            (Self::Southeast, Self::South) |
            (Self::South, Self::East) |
            (Self::South, Self::Southeast) |
            (Self::East, Self::Southeast) => Ok(Self::Southeast),
            (Self::South, Self::West) |
            (Self::Southwest, Self::South) |
            (Self::Southwest, Self::West) |
            (Self::West, Self::Southwest) |
            (Self::West, Self::South) |
            (Self::South, Self::Southwest) => Ok(Self::Southwest),
            (Self::North, Self::North) |
            (Self::Northeast, Self::Northeast) |
            (Self::East, Self::East) |
            (Self::Southeast, Self::Southeast) |
            (Self::South, Self::South) |
            (Self::Southwest, Self::Southwest) |
            (Self::West, Self::West) |
            (Self::Northwest, Self::Northwest) => Ok(edge),
            (Self::North, Self::Southeast) |
            (Self::North, Self::South) |
            (Self::North, Self::Southwest) |
            (Self::Northeast, Self::Southeast) |
            (Self::Northeast, Self::South) |
            (Self::Northeast, Self::Southwest) |
            (Self::Northeast, Self::West) |
            (Self::Northeast, Self::Northwest) |
            (Self::East, Self::Southwest) |
            (Self::East, Self::West) |
            (Self::East, Self::Northwest) |
            (Self::Southeast, Self::North) |
            (Self::Southeast, Self::Northeast) |
            (Self::Southeast, Self::Southwest) |
            (Self::Southeast, Self::West) |
            (Self::Southeast, Self::Northwest) |
            (Self::South, Self::North) |
            (Self::South, Self::Northeast) |
            (Self::South, Self::Northwest) |
            (Self::Southwest, Self::North) |
            (Self::Southwest, Self::Northeast) |
            (Self::Southwest, Self::East) |
            (Self::Southwest, Self::Southeast) |
            (Self::Southwest, Self::Northwest) |
            (Self::West, Self::Northeast) |
            (Self::West, Self::East) |
            (Self::West, Self::Southeast) |
            (Self::Northwest, Self::Northeast) |
            (Self::Northwest, Self::East) |
            (Self::Northwest, Self::Southeast) |
            (Self::Northwest, Self::South) |
            (Self::Northwest, Self::Southwest) => Err(CommandError::InvalidTileEdge(edge,self.clone()))
        }

    }

    pub(crate) fn direction(&self) -> Deg<f64> {
        // needs to be clockwise, from the north, with a value from 0..360
        match self {
            Edge::North => Deg(0.0),
            Edge::Northeast => Deg(45.0),
            Edge::East => Deg(90.0),
            Edge::Southeast => Deg(135.0),
            Edge::South => Deg(180.0),
            Edge::Southwest => Deg(225.0),
            Edge::West => Deg(270.0),
            Edge::Northwest => Deg(315.0),
        }
    }

    pub(crate) fn contains(&self, p: &(f64, f64), extent: &Extent) -> bool {
        match self {
            Edge::North => p.1 == extent.north(),
            Edge::Northeast => p.1 == extent.north() || p.0 == extent.east(),
            Edge::East => p.0 == extent.east(),
            Edge::Southeast => p.1 == extent.south || p.0 == extent.east(),
            Edge::South => p.1 == extent.south,
            Edge::Southwest => p.1 == extent.south || p.0 == extent.west,
            Edge::West => p.0 == extent.west,
            Edge::Northwest => p.1 == extent.north() || p.0 == extent.west,
        }
    }
}

impl_simple_serde_tagged_enum!{
    Edge {
        North,
        Northeast,
        East,
        Southeast,
        South,
        Southwest,
        West,
        Northwest
    
    }
}


#[derive(Hash,Eq,PartialEq,Clone,Debug)]
pub(crate) struct Point {
    x: NotNan<f64>,
    y: NotNan<f64>
}

impl Point {

    pub(crate) fn to_tuple(&self) -> (f64,f64) {
        (*self.x,*self.y)
    }

    pub(crate) fn to_vector_2(&self) -> Vector2 {
        Vector2::new(*self.x,*self.y)
    }

    const fn new(x: NotNan<f64>, y: NotNan<f64>) -> Self {
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

    pub(crate) fn create_geometry(&self) -> Result<GeoPoint,CommandError> {
        GeoPoint::new(self.x.into(), self.y.into())
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

    pub(crate) fn longitude_across_antimeridian<Float>(source_x: Float, relative_x: Float) -> Float 
    where Float: PartialOrd + Sub<f64, Output = Float> + Add<f64, Output = Float> {
        if source_x > relative_x {
            // it's across to the west, on the far east longitudes, so shift it around to the west
            source_x - 360.0
        } else {
            // otherwise it's across to the east, so shift it around to the east
            source_x + 360.0
        }
    }

    pub(crate) fn across_antimeridian(&self, relative_to: &Self) -> Self {
        Self {
            x: Self::longitude_across_antimeridian(self.x, relative_to.x),
            y: self.y
        }
    }

    pub(crate) fn clip_point_vec_across_antimeridian(line: Vec<Self>, extent: &Extent) -> Result<Vec<Vec<Self>>,CommandError> {

        #[derive(PartialEq)]
        enum Location {
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
            segment.push(fix_point(&previous,&previous_location));
            for next in line {
                let next_location = get_location(&next);
                if next_location != previous_location {
                    let mid_point = previous.middle_point_between(&next);
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

impl TryFrom<(f64,f64,f64)> for Point {

    type Error = FloatIsNan;

    fn try_from(value: (f64,f64,f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: NotNan::new(value.0)?,
            y: NotNan::new(value.1)?
        })
    }
}

impl From<(NotNan<f64>,NotNan<f64>)> for Point {

    fn from(value: (NotNan<f64>,NotNan<f64>)) -> Self {
        Self {
            x: value.0,
            y: value.1
        }
    }
}

impl TryFrom<(f64,f64)> for Point {

    type Error = FloatIsNan;

    fn try_from(value: (f64,f64)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: value.0.try_into()?,
            y: value.1.try_into()?
        })
    }
}

impl Sub for &Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add for &Point {
    type Output = Point;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}


pub(crate) mod title_case {

    use std::fmt;

    pub(crate) struct AsTitleCase<StringType: AsRef<str>>(StringType);

    impl<T: AsRef<str>> fmt::Display for AsTitleCase<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

            let source: &str = self.0.as_ref();
            
            let mut first = true;
            for word in source.split(' ') {
                if first {
                    first = false;
                } else {
                    write!(f," ")?;
                }
                let mut chars = word.chars();
                if let Some(first_char) = chars.next() {
                    write!(f,"{}",first_char.to_uppercase())?;
                    for char in chars {
                        write!(f,"{}",char.to_lowercase())?
                    }
                }

            }

            Ok(())
        }
    }    

    pub(crate) trait ToTitleCase: ToOwned {
        /// Convert this type to title case.
        fn to_title_case(&self) -> Self::Owned;
    }

    impl ToTitleCase for str {
        fn to_title_case(&self) -> String {
            AsTitleCase(self).to_string()
        }
    }


}

pub(crate) mod namers_pretty_print {


    use std::io;
    use serde_json::ser::Formatter;

    // Mostly copy-paste from serde_json, but designed to output arrays inline at any nesting above one, for serializing namers in an array.

    fn indent<W>(wr: &mut W, n: usize, s: &[u8]) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        for _ in 0..n {
            wr.write_all(s)?;
        }

        Ok(())
    }

    /// This structure pretty prints a JSON value to make it human readable.
    #[derive(Clone, Debug)]
    pub(crate) struct PrettyFormatter<'indent> {
        current_indent: usize,
        has_value: bool,
        array_nesting: usize,
        indent: &'indent [u8],
    }

    impl<'indent> PrettyFormatter<'indent> {
        /// Construct a pretty printer formatter that defaults to using two spaces for indentation.
        pub(crate) const fn new() -> Self {
            PrettyFormatter::with_indent(b"  ")
        }

        /// Construct a pretty printer formatter that uses the `indent` string for indentation.
        pub(crate) const fn with_indent(indent: &'indent [u8]) -> Self {
            PrettyFormatter {
                current_indent: 0,
                has_value: false,
                array_nesting: 0,
                indent,
            }
        }
    }

    impl Default for PrettyFormatter<'_> {
        fn default() -> Self {
            PrettyFormatter::new()
        }
    }

    impl Formatter for PrettyFormatter<'_> {
        #[inline]
        fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            self.array_nesting += 1;
            if self.array_nesting <= 1 {
                self.current_indent += 1;
            }
            self.has_value = false;
            writer.write_all(b"[")
        }

        #[inline]
        fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            if self.array_nesting <= 1 {
                self.current_indent -= 1;
                if self.has_value {
                    writer.write_all(b"\n")?;
                    indent(writer, self.current_indent, self.indent)?;
                }
            }

            self.array_nesting -= 1;
            writer.write_all(b"]")
        }

        #[inline]
        fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            if self.array_nesting > 1 {
                writer.write_all(if first { b"" } else { b", " })
            } else {
                writer.write_all(if first { b"\n" } else { b",\n" })?;
                indent(writer, self.current_indent, self.indent)
            }
        }

        #[inline]
        fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            self.has_value = true;
            Ok(())
        }

        #[inline]
        fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            self.current_indent += 1;
            self.has_value = false;
            writer.write_all(b"{")
        }

        #[inline]
        fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            self.current_indent -= 1;

            if self.has_value {
                writer.write_all(b"\n")?;
                indent(writer, self.current_indent, self.indent)?;
            }

            writer.write_all(b"}")
        }

        #[inline]
        fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            writer.write_all(if first { b"\n" } else { b",\n" })?;
            indent(writer, self.current_indent, self.indent)
        }

        #[inline]
        fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            writer.write_all(b": ")
        }

        #[inline]
        fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
        where
            W: ?Sized + io::Write,
        {
            self.has_value = true;
            Ok(())
        }
    }


}

pub(crate) fn split_string_from_end(string: &str, char_index_from_end: usize) -> (&str, &str) {

    let char_indexes = string.char_indices();
    let mut reversed = char_indexes.rev();
    if let Some(index) = reversed.nth(char_index_from_end) {
        string.split_at(index.0)
    } else {
        (string,"")
    }

}


pub(crate) trait ToRoman {

    fn to_roman(&self) -> Option<String>;

}

macro_rules! impl_to_roman {
    ($integer: ty) => {
        impl ToRoman for $integer {

            fn to_roman(&self) -> Option<String> {
                fn romanize_part(result: &mut String, remaining: &mut $integer) -> bool {
                    if *remaining >= 4000 {
                        // can't do anything larger, although I could support vinculum, that only gets me so far anyway.
                        false
                    } else if *remaining >= 1000 {
                        result.push('M');
                        *remaining -= 1000;
                        true
                    } else if *remaining >= 900 {
                        result.push('C');
                        result.push('M');
                        *remaining -= 900;
                        true
                    } else if *remaining >= 500 {
                        result.push('D');
                        *remaining -= 500;
                        true
                    } else if *remaining >= 400 {
                        result.push('C');
                        result.push('D');
                        *remaining -= 400;
                        true
                    } else if *remaining >= 100 {
                        result.push('C');
                        *remaining -= 100;
                        true
                    } else if *remaining >= 90 {
                        result.push('X');
                        result.push('C');
                        *remaining -= 90;
                        true
                    } else if *remaining >= 50 {
                        result.push('L');
                        *remaining -= 50;
                        true
                    } else if *remaining >= 40 {
                        result.push('X');
                        result.push('L');
                        *remaining -= 40;
                        true
                    } else if *remaining >= 10 {
                        result.push('X');
                        *remaining -= 10;
                        true
                    } else if *remaining >= 9 {
                        result.push('I');
                        result.push('X');
                        *remaining -= 9;
                        true
                    } else if *remaining >= 5 {
                        result.push('V');
                        *remaining -= 5;
                        true
                    } else if *remaining >= 4 {
                        result.push('I');
                        result.push('V');
                        *remaining -= 4;
                        true
                    } else if *remaining >= 1 {
                        result.push('I');
                        *remaining -= 1;
                        true
                    } else if *remaining == 0 {
                        true
                    } else {
                        false
                    }
                }
                let mut remaining = *self;
                let mut result = String::new();
                while remaining > 0 {
                    if !romanize_part(&mut result, &mut remaining) {
                        return None
                    }
                }
                Some(result)
            }
        
        
        }
    };
}

impl_to_roman!(usize);

pub(crate) mod point_finder {
    // FUTURE: This was an implementation I found on crates.io that allowed inserting and floating point points, and wasn't too difficult to construct. Although that could be done better. It didn't have a lot of downloads, however, so I don't know if it's really something I should be using. The alternatives were lacking features I needed.
    use qutee::QuadTree; 
    use qutee::Boundary;

    use super::Extent;
    use super::Point;
    use crate::errors::CommandError;

    pub(crate) struct PointFinder {
      // It's kind of annoying, but the query method doesn't return the original point, so I have to store the point.
      inner: QuadTree<f64,Point>,
      bounds: Boundary<f64>, // it also doesn't give us access to this, which is useful for cloning
      capacity: usize // or this
    }
    
    impl PointFinder {
    
        pub(crate) fn new(extent: &Extent, capacity: usize) -> Self {
            let bounds = Boundary::between_points((extent.west,extent.south),(extent.east(),extent.north()));
            Self {
                inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
                bounds,
                capacity
            }
        }

        pub(crate) fn add_point(&mut self, point: Point) -> Result<(),CommandError> {
            self.inner.insert_at(point.to_tuple(),point).map_err(|e|  {
                match e {
                    qutee::QuadTreeError::OutOfBounds(_, qutee::Point { x, y }) => CommandError::PointFinderOutOfBounds(x,y),
                }
                
            })

        }

        pub(crate) fn points_in_target(&mut self, point: &Point, spacing: f64) -> bool {
            let west = point.x - spacing;
            let south = point.y - spacing;
            let north = point.x + spacing;
            let east = point.y + spacing;
            let boundary = Boundary::between_points((west.into(),south.into()),(east.into(),north.into()));
            for item in self.inner.query(boundary) {
                if item.distance(point) <= spacing {
                    return true;
                }
            }
            false

        }

        pub(crate) fn fill_from(other: &Self, additional_size: usize) -> Result<Self,CommandError> {
            let bounds = other.bounds.clone();
            let capacity = other.capacity + additional_size;
            let mut result = Self {
                inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
                bounds,
                capacity
            };
            for point in other.inner.iter() {
                result.add_point(point.clone())?
            }
            Ok(result)
        }
    }
    
    pub(crate) struct TileFinder {
      inner: QuadTree<f64,(Point,u64)>, // I need the original point to test distance
      bounds: Boundary<f64>, // see PointFinder
      //capacity: usize, // see PointFinder
      initial_search_radius: f64
    }
    
    impl TileFinder {
    
        pub(crate) fn new(extent: &Extent, capacity: usize, tile_spacing: f64) -> Self {
            let bounds = Boundary::between_points((extent.west,extent.south),(extent.east(),extent.north()));
            Self {
                inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
                bounds,
                //capacity,
                initial_search_radius: tile_spacing
            }
        }

        pub(crate) fn add_tile(&mut self, point: Point, tile: u64) -> Result<(),CommandError> {
            self.inner.insert_at(point.to_tuple(),(point,tile)).map_err(|e|  {
                match e {
                    qutee::QuadTreeError::OutOfBounds(_, qutee::Point { x, y }) => CommandError::PointFinderOutOfBounds(x,y),
                }
                
            })

        }

        pub(crate) fn find_nearest_tile(&self, point: &Point) -> Result<u64,CommandError> {
            let mut spacing = self.initial_search_radius;

            macro_rules! calc_search_boundary {
                () => {
                    {
                        let west = point.x - spacing;
                        let south = point.y - spacing;
                        let north = point.x + spacing;
                        let east = point.y + spacing;
                        Boundary::between_points((west.into(),south.into()),(east.into(),north.into()))
                    }
                };
            }

            let mut search_boundary = calc_search_boundary!();

            macro_rules! find_tile {
                () => {
                    let mut found = None;
                    for item in self.inner.query(search_boundary) {
                        match found {
                            None => found = Some((item.1,item.0.distance(point))),
                            Some(last_found) => {
                                let this_distance = item.0.distance(point);
                                if this_distance < last_found.1 {
                                    found = Some((item.1,this_distance))
                                }
                            },
                        }
                    }
                    if let Some((tile,_)) = found {
                        return Ok(tile)
                    }                        
                };
            }

            for _ in 0..10 { // try ten times at incrementing radiuses before giving up and searching the whole index. If they still haven't found one by then it's probably an empty tile board.
                find_tile!();
                // double the spacing and keep searching
                spacing *= 2.0;
                search_boundary = calc_search_boundary!();
            }
            // just search over the whole thing:
            search_boundary = self.bounds.clone();
            find_tile!();
            // okay, nothing was found, this is an error.
            Err(CommandError::CantFindTileNearPoint)

        }

    }
    

}

#[derive(Clone)]
pub enum ArgRange<NumberType> {
    // While I could use a real Range<> and RangeInclusive<>, I'd have to copy it every time I want to generate a number from it anyway, and
    Inclusive(NumberType,NumberType),
    Exclusive(NumberType,NumberType),
    Single(NumberType)
}


pub trait TruncOrSelf {

    fn trunc_or_self(self) -> Self;
}

macro_rules! impl_trunc_or_self_float {
    ($float: ty) => {
        impl TruncOrSelf for $float {
            fn trunc_or_self(self) -> Self {
                self.trunc()
            }
        }
                
    };
}

macro_rules! impl_trunc_or_self_int {
    ($int: ty) => {
        impl TruncOrSelf for $int {
            fn trunc_or_self(self) -> Self {
                self
            }
        }
                
    };
}

impl_trunc_or_self_float!(f64);
impl_trunc_or_self_float!(f32);
impl_trunc_or_self_int!(usize);
impl_trunc_or_self_int!(i8);
impl_trunc_or_self_int!(i16);
impl_trunc_or_self_int!(i32);
impl_trunc_or_self_int!(i64);
impl_trunc_or_self_int!(i128);
impl_trunc_or_self_int!(u8);
impl_trunc_or_self_int!(u16);
impl_trunc_or_self_int!(u32);
impl_trunc_or_self_int!(u64);
impl_trunc_or_self_int!(u128);


impl<NumberType: SampleUniform + PartialOrd + Copy + TruncOrSelf> ArgRange<NumberType> {

    pub(crate) fn choose<Random: Rng>(&self, rng: &mut Random) -> NumberType {
        match self  {
            Self::Inclusive(min,max) => rng.gen_range(*min..=*max),
            Self::Exclusive(min,max) => rng.gen_range(*min..*max),
            Self::Single(value) => *value,
        }
    }

    pub(crate) fn includes(&self, value: &NumberType) -> bool {
        match self {
            Self::Inclusive(min, max) => (value >= min) && (value <= max),
            Self::Exclusive(min, max) => (value >= min) && (value < max),
            Self::Single(single) => single.trunc_or_self() == single.trunc_or_self(),
        }
    }
}



impl<'deserializer,NumberType: FromStr + PartialOrd + Deserialize<'deserializer>> Deserialize<'deserializer> for ArgRange<NumberType> {

    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'deserializer> {

        // https://stackoverflow.com/q/56582722/300213
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StrOrNum<NumberType> {
            Str(String),
            Num(NumberType)
        }

        let value = StrOrNum::deserialize(deserializer)?;
        match value {
            StrOrNum::Str(deserialized) => deserialized.parse().map_err(|e: CommandError| serde::de::Error::custom(e.to_string())),
            StrOrNum::Num(deserialized) => Ok(Self::Single(deserialized)),
        }
        
    }
}

impl<NumberType: FromStr + Display> Serialize for ArgRange<NumberType> {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<NumberType: FromStr + PartialOrd> FromStr for ArgRange<NumberType> {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((first,mut last)) = s.split_once("..") {
            let include_last = if last.starts_with('=') {
                last = last.trim_start_matches('=');
                true
            } else {
                false
            };

            let first = first.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            let last = last.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            if first > last {
                return Err(CommandError::InvalidRangeArgument(s.to_owned()))
            }

            Ok(if include_last {
                Self::Inclusive(first,last)
            } else {
                Self::Exclusive(first,last)
            })
        } else {
            let number = s.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            Ok(Self::Single(number))
        }
    }
}

impl<NumberType: FromStr + Display> Display for ArgRange<NumberType> {

    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Inclusive(min,max) => write!(f,"{min}..={max}"),
            Self::Exclusive(min,max) => write!(f,"{min}..{max}"),
            Self::Single(single) => write!(f,"{single}"),
        }
    }
}

pub(crate) mod simple_serde {

    /*
    Yes, this is reinventing the wheel, however...

    1) It took me less time to write this (5-6 hours, 8 hours with editing), than it would to try to wrangle serde into something I could use.
    2) I wanted a serialization format that was text-based. I wanted simple enums to be output as bare identifiers (not strings), so that other applications could use their values as labels. I wanted to be able to output the Neighbor type as an untagged, I specifically designed it so the types were incompatible, so an untagged enum was possible.
    3) I didn't want json (serde_json), because all identifiers are strings.
    4) Rusty Object Notation (ron) was much closer to what I wanted, except that it could not handle the untagged enums for the Neighbor enum. Well, it could for serializing, but not deserializing. This is where a custom deserializer might have worked, but the serde API for these is so arcane it would have taken me at least 5-6 hours to figure that out before implementing it, possibly discovering that it was still impossible.
    5) YAML was right out, as it kept inserting linefeeds into my values. I didn't stick around long enough to decide if there were any other problems.

    Simple Serde is easy to use. You serialize by writing tokens and values out. You deserialize it by standard parsing techniques: expectingand matching tokens. You don't have to deal with creating visitors and implementing matchers.

    Deserialization is strictly typed. There is no deserialize_any. If you don't know what you're expecting, this isn't the right tool for you.

    To read and write types, you call 'read_from_str' or 'write_to_string' directly on the type. Buffers aren't yet supported, but who knows, maybe someday. You might be able to call 'Deserialize::read_from_str' if rust can figure out the type of the result.

    To make a type readable or writable, there are several options:
    * If it's an enum, use `impl_simple_serde_tagged_enum`.
    * If it's a tuple struct use `impl_simple_serde_tuple_struct`
    * If it's a keyed struct, use `impl_simple_serde_keyed_struct`
    * If it doesn't fit those, or you want to control serialization, implement Serialize and Deserialize. There's only one function on each that you must implement, and its straightforward. Then make sure you have tests to confirm that the value serialized will also be deserialized.

    Note that in all of the macro cases above, you almost have to repeat the entire structure of the type, because I'm too lazy to create a proc macro. Only tuple structs and tuple enum variants can get by with bare identifiers, but they still must match the count. However, you don't have to worry about whether you have them correct, because the compiler will warn you if you've got names or counts wrong.

    If you want to use a different format, such as json, you might be able to implement Serializer and Deserializer, and pass that to `read_from` and `write_to` methods on the objects. But more than likely you'll find yourself struggling, and decide that it's just better to use another library. That's why I don't use this for more file-based input data in this crate.
     
    */

    use core::str::Chars;
    use core::iter::Peekable;

    use paste::paste;
    
    use crate::errors::CommandError;

    #[derive(Debug,Clone)]
    pub enum Token {
        OpenBracket,
        CloseBracket,
        OpenParenthesis,
        CloseParenthesis,
        OpenBrace,
        CloseBrace,
        Colon,
        Comma,
        Whitespace,
        Integer(u64),
        SignedInteger(i64),
        Float(f64),
        String(String),
        Identifier(String)
    }

    pub(crate) struct Tokenizer<'string> {
        text: Peekable<Chars<'string>>,
    }

    impl Iterator for Tokenizer<'_> {

        type Item = Result<Token,CommandError>;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(char) = self.text.peek() {
                match char {
                    '[' => {
                        _ = self.text.next();
                        Some(Ok(Token::OpenBracket))
                    },
                    ']' => {
                        _ = self.text.next();
                        Some(Ok(Token::CloseBracket))
                    },
                    '(' => {
                        _ = self.text.next();
                        Some(Ok(Token::OpenParenthesis))
                    },
                    ')' => {
                        _ = self.text.next();
                        Some(Ok(Token::CloseParenthesis))
                    },
                    '{' => {
                        _ = self.text.next();
                        Some(Ok(Token::OpenBrace))
                    },
                    '}' => {
                        _ = self.text.next();
                        Some(Ok(Token::CloseBrace))
                    },
                    ':' => {
                        _ = self.text.next();
                        Some(Ok(Token::Colon))
                    },
                    ',' => {
                        _ = self.text.next();
                        Some(Ok(Token::Comma))
                    },
                    ' ' => {
                        _ = self.text.next();
                        while let Some(' ') = self.text.peek() {
                            _ = self.text.next();
                        }
                        Some(Ok(Token::OpenBracket))
                    },
                    '-' | '+' | '0'..='9' => {
                        let char = *char;
                        let signed = matches!(char,'-' | '+');
                        let mut number = String::from(char);
                        _ = self.text.next();
                        while let Some(char @ '0'..='9') = self.text.peek() {
                            number.push(*char);
                            _ = self.text.next();
                        }

                        if let Some('.') = self.text.peek() {
                            number.push('.');
                            _ = self.text.next();
                            while let Some(char @ '0'..='9') = self.text.peek() {
                                number.push(*char);
                                _ = self.text.next();
                            }

                            match number.parse() {
                                Ok(value) => Some(Ok(Token::Float(value))),
                                Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                            }                    

                        } else if signed {
                            match number.parse() {
                                Ok(value) => Some(Ok(Token::SignedInteger(value))),
                                Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                            }                    
                        } else {
                            match number.parse() {
                                Ok(value) => Some(Ok(Token::Integer(value))),
                                Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                            }                    
    
                        }

                    },
                    '"' => {
                        let mut value = String::new();
                        _ = self.text.next();
                        let mut found_quote = false;
                        while let Some(char) = self.text.next() {
                            match char {
                                '"' => {
                                    found_quote = true;
                                    break
                                },
                                '\\' => if let Some(char) = self.text.next() {
                                    value.push(char);
                                } else {
                                    value.push('\\');
                                    break;
                                },
                                c => value.push(c)
                            }
                        }

                        if found_quote {
                            Some(Ok(Token::String(value)))
                        } else {
                            Some(Err(CommandError::InvalidStringInSerializedValue(value)))
                        }
                    },
                    'A'..='Z' | 'a'..='z' => {
                        let mut value = String::from(*char);
                        _ = self.text.next();
                        while let Some(char @ 'A'..='Z' | char @ 'a'..='z' | char @ '_' | char @ '0'..='9') = self.text.peek() {
                            value.push(*char);
                            _ = self.text.next();
                        }
                        Some(Ok(Token::Identifier(value)))
                    },
                    _ => {
                        Some(Err(CommandError::InvalidCharacterInSerializedValue(*char)))
                    }

                }

            } else {
                None
            }
            
        }

    }

    pub(crate) trait Deserializer {

        fn expect(&mut self, expected: &Token) -> Result<(),CommandError>;

        fn matches(&mut self, desired: &Token) -> Result<bool,CommandError>;

        fn expect_identifier(&mut self) -> Result<String,CommandError>;

        fn skip_whitespace(&mut self) -> Result<(),CommandError>;

        fn expect_float(&mut self) -> Result<f64,CommandError>;

        fn expect_integer(&mut self, size: u32) -> Result<u64,CommandError>;

        fn matches_integer(&mut self) -> Result<Option<u64>,CommandError>;

        fn expect_signed_integer(&mut self, size: u32) -> Result<i64,CommandError>;

        fn peek_token(&mut self) -> Result<Option<&Token>,CommandError>;

    }

    impl Deserializer for Peekable<Tokenizer<'_>> {

        fn expect(&mut self, expected: &Token) -> Result<(),CommandError>  {
            self.skip_whitespace()?;
            match self.next().transpose()? {
                Some(found) => match (expected,&found) {
                    (Token::OpenBracket, Token::OpenBracket) |
                    (Token::CloseBracket, Token::CloseBracket) |
                    (Token::OpenParenthesis, Token::OpenParenthesis) |
                    (Token::CloseParenthesis, Token::CloseParenthesis) |
                    (Token::Comma, Token::Comma) |
                    (Token::Whitespace, Token::Whitespace) => Ok(()),
                    (Token::Integer(a), Token::Integer(b)) => if a == b {
                        Ok(())
                    } else {
                        Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                    },
                    (Token::SignedInteger(a), Token::SignedInteger(b)) => if a == b {
                        Ok(())
                    } else {
                        Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                    },
                    (Token::Float(a), Token::Float(b)) => if a == b {
                        Ok(())
                    } else {
                        Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                    },
                    (Token::String(a), Token::String(b)) |
                    (Token::Identifier(a), Token::Identifier(b)) => if a == b {
                        Ok(())
                    } else {
                        Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                    },
                    (_,_) => Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                },
                None => Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),None))
            }
        }


        fn matches(&mut self, desired: &Token) -> Result<bool,CommandError> {
            self.skip_whitespace()?;
            let result = match self.peek() {
                Some(Ok(found)) => match (desired,found) {
                    (Token::OpenBracket, Token::OpenBracket) |
                    (Token::CloseBracket, Token::CloseBracket) |
                    (Token::OpenParenthesis, Token::OpenParenthesis) |
                    (Token::CloseParenthesis, Token::CloseParenthesis) |
                    (Token::Comma, Token::Comma) |
                    (Token::Whitespace, Token::Whitespace) => true,
                    (Token::Integer(a), Token::Integer(b)) => if a == b {
                        true
                    } else {
                        false
                    },
                    (Token::SignedInteger(a), Token::SignedInteger(b)) => if a == b {
                        true
                    } else {
                        false
                    },
                    (Token::Float(a), Token::Float(b)) => if a == b {
                        true
                    } else {
                        false 
                    },
                    (Token::String(a), Token::String(b)) |
                    (Token::Identifier(a), Token::Identifier(b)) => if a == b {
                        true
                    } else {
                        false
                    },
                    (_,_) => false
                },
                Some(Err(err)) => return Err(err.clone()),
                None => false
            };
            if result {
                _ = self.next().transpose()?;
            }
            Ok(result)
        }

        fn expect_identifier(&mut self) -> Result<String,CommandError> {
            self.skip_whitespace()?;
            match self.next().transpose()? {
                Some(Token::Identifier(value)) => Ok(value),
                Some(token) => Err(CommandError::ExpectedIdentifierInSerializedValue(Some(token))),
                None => Err(CommandError::ExpectedIdentifierInSerializedValue(None)),
            }
        }

        fn skip_whitespace(&mut self) -> Result<(),CommandError> {
            while let Some(Ok(Token::Whitespace)) = self.peek() {
                _ = self.next().transpose()?;
            }
            Ok(())
        }

        fn expect_float(&mut self) -> Result<f64,CommandError> {
            self.skip_whitespace()?;
            match self.next().transpose()? {
                Some(Token::Float(value)) => Ok(value),
                Some(Token::Integer(value)) => Ok(value as f64),
                Some(Token::SignedInteger(value)) => Ok(value as f64),
                Some(token) => Err(CommandError::ExpectedFloatInSerializedValue(Some(token))),
                None => Err(CommandError::ExpectedFloatInSerializedValue(None)),
            }
        }

        fn expect_integer(&mut self, size: u32) -> Result<u64,CommandError> {
            self.skip_whitespace()?;
            match self.next().transpose()? {
                Some(Token::Integer(value)) => Ok(value),
                Some(token) => Err(CommandError::ExpectedIntegerInSerializedValue(size,false,Some(token))),
                None => Err(CommandError::ExpectedIntegerInSerializedValue(size,false,None)),
            }
        }

        fn expect_signed_integer(&mut self, size: u32) -> Result<i64,CommandError> {
            self.skip_whitespace()?;
            match self.next().transpose()? {
                Some(Token::SignedInteger(value)) => Ok(value),
                Some(Token::Integer(value)) => Ok(value as i64),
                Some(token) => Err(CommandError::ExpectedIntegerInSerializedValue(size,true,Some(token))),
                None => Err(CommandError::ExpectedIntegerInSerializedValue(size,true,None)),
            }
        }

        fn matches_integer(&mut self) -> Result<Option<u64>,CommandError> {
            self.skip_whitespace()?;
            match self.peek() {
                Some(Ok(Token::Integer(value))) => {
                    let value = *value;
                    _ = self.next().transpose()?;
                    Ok(Some(value))
                },
                Some(Ok(_)) => Ok(None),
                Some(Err(err)) => Err(err.clone()),
                None => Ok(None),
            }
        }

        fn peek_token(&mut self) -> Result<Option<&Token>,CommandError> {
            match self.peek() {
                Some(value) => match value {
                    Ok(value) => Ok(Some(value)),
                    Err(err) => Err(err.clone()),
                },
                None => Ok(None),
            }
        }

    }

    pub(crate) trait Serializer: Sized {

        fn write_token(&mut self, token: Token);

        fn serialize_value<Value: Serialize>(&mut self, value: Value) {
            value.write_value(self)
        }

    }

    impl Serializer for String {
        fn write_token(&mut self, token: Token) {
            match token {
                Token::OpenBracket => self.push('['),
                Token::CloseBracket => self.push(']'),
                Token::OpenParenthesis => self.push('('),
                Token::CloseParenthesis => self.push(')'),
                Token::OpenBrace => self.push('{'),
                Token::CloseBrace => self.push('}'),
                Token::Colon => self.push(':'),
                Token::Comma => self.push(','),
                Token::Whitespace => self.push(' '),
                Token::Float(number) => self.push_str(&number.to_string()),
                Token::Integer(number) => self.push_str(&number.to_string()),
                Token::SignedInteger(number) => self.push_str(&number.to_string()),
                Token::String(string) => {
                    self.push('"');
                    self.push_str(&string.replace('"', "\\\""));
                    self.push('"');
                },
                Token::Identifier(identifier) => self.push_str(&identifier),
            }
        }
    }

    pub(crate) trait Serialize {

        fn write_value<Target: Serializer>(&self, serializer: &mut Target);

        fn write_to_string(&self) -> String {
            let mut string = String::new();
            self.write_value(&mut string);
            string
        }

    }

    impl<Borrowed: Serialize> Serialize for &Borrowed {
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            (*self).write_value(serializer)
        }
    }

    pub(crate) trait Deserialize: Sized {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError>;

        fn read_from_str(string: &str) -> Result<Self,CommandError> {
            let tokenizer = Tokenizer {
                text: string.chars().peekable()
            };

            Deserialize::read_value(&mut tokenizer.peekable())        
        }
    }

    impl<ItemType: Serialize> Serialize for Vec<ItemType> {
        
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            serializer.write_token(Token::OpenBracket);
            let mut first = true;
            for item in self {
                if first {
                    first = false;
                } else {
                    serializer.write_token(Token::Comma);
                }
                item.write_value(serializer);
            }
            serializer.write_token(Token::CloseBracket)
        }
    }

    impl<ItemType: Deserialize> Deserialize for Vec<ItemType> {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
            deserializer.expect(&Token::OpenBracket)?;
            let mut result = Vec::new();
            if !deserializer.matches(&Token::CloseBracket)? {
                result.push(Deserialize::read_value(deserializer)?);
                while deserializer.matches(&Token::Comma)? {
                    result.push(Deserialize::read_value(deserializer)?);    
                }
                deserializer.expect(&Token::CloseBracket)?;
            }
            Ok(result)
        }

    }

    macro_rules! impl_simple_serde_tuple {
        ($($first_name: ident: $first_gen_param: ident $(, $name: ident: $gen_param: ident)* $(,)?)?) => {

            impl$(<$first_gen_param: Serialize $(,$gen_param: Serialize)*>)? Serialize for ($($first_gen_param, $($gen_param),*)?) {

                fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
                    serializer.write_token(Token::OpenParenthesis);
                    $(
                        let ($first_name, $($name,)*) = self;
                        $first_name.write_value(serializer);
                        $(
                            serializer.write_token(Token::Comma);
                            $name.write_value(serializer);
                        )*
                    )?
                    serializer.write_token(Token::CloseParenthesis)
                }
                        
            }
            
            impl$(<$first_gen_param: Deserialize $(,$gen_param: Deserialize)*>)? Deserialize for ($($first_gen_param, $($gen_param),*)?) {

                fn read_value<Source: Deserializer>(source: &mut Source) -> Result<Self,CommandError> {
                    source.expect(&Token::OpenParenthesis)?;
                    $(
                        let $first_name = Deserialize::read_value(source)?;
                        $(
                            source.expect(&Token::Comma)?;
                            let $name = Deserialize::read_value(source)?;
                        )*
                    )?
                    source.expect(&Token::CloseParenthesis)?;
                    Ok(($($first_name,$($name,)*)?))
                }
        
            }            
        };
        ($($first_gen_param: ident $(, $gen_param: ident)* $(,)?)?) => {
            paste!{
                impl_simple_serde_tuple!($([<$first_gen_param:snake>]: $first_gen_param $(, [<$gen_param:snake>]: $gen_param)*)?);
            }
        }
    }

    impl_simple_serde_tuple!();

    impl_simple_serde_tuple!(Item1);

    impl_simple_serde_tuple!(Item1,Item2);

    impl Serialize for f64 {
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            serializer.write_token(Token::Float(*self))
        }
    }

    impl Deserialize for f64 {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
            deserializer.expect_float()
        }
    }

    impl Serialize for u64 {
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            serializer.write_token(Token::Integer(*self))
        }
    }

    impl Deserialize for u64 {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
            deserializer.expect_integer(64)
        }
    }

    impl Serialize for usize {
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            serializer.write_token(Token::Integer(*self as u64))
        }
    }

    impl Deserialize for usize {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
            Ok(deserializer.expect_integer(usize::BITS)? as usize)
        }
    }

    impl Serialize for i32 {
        fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
            serializer.write_token(Token::SignedInteger(*self as i64))
        }

    }

    impl Deserialize for i32 {

        fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
            Ok(deserializer.expect_signed_integer(32)? as i32)
        }
    }

    #[macro_export]
    macro_rules! impl_simple_serde_tagged_enum {

        ($enum: ty {$($variant: ident $(($($name: ident),*$(,)?))?),*$(,)?}) => {
            impl $crate::utils::simple_serde::Serialize for $enum {
        
                fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                    match self {
                        $(
                            Self::$variant$(($($name,)*))? => {
                                serializer.write_token($crate::utils::simple_serde::Token::Identifier(stringify!($variant).to_owned()));
                                $(
                                    // use tuple serialization to do it. Note that I need the comma even on the one-element to convert it into a tuple
                                    ($( $name, )*).write_value(serializer)
                                )?
                            },
                        )*
                    }
                }
            }

            impl $crate::utils::simple_serde::Deserialize for $enum {
            
                fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                    let identifier = deserializer.expect_identifier()?;
                    match identifier.as_str() {
                        $(
                            stringify!($variant) => {
                                // use tuple deserialization. Note that I need the comma even on the one-element to convert it into a tuple
                                $( let ($($name,)*) = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;)?
                                Ok(Self::$variant$(($($name,)*))?)
                            }
                        ),*
                        invalid => Err($crate::errors::CommandError::InvalidEnumValueInInSerializedValue(invalid.to_owned())),
                    }
                }
            
            }
            
            
        };
    }

    #[macro_export]
    macro_rules! impl_simple_serde_tuple_struct {

        ($struct: ty {$($name: ident),*$(,)?}) => {
            impl $crate::utils::simple_serde::Serialize for $struct {
        
                fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                    let Self($($name,)*) = self;
                    // use tuple serialization to handle it. Note that I need the comma even on the one-element to convert it into a tuple
                    ($($name,)*).write_value(serializer);
                }
            }

            impl $crate::utils::simple_serde::Deserialize for $struct {
            
                fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                    // use tuple deserialization. Note that I need the comma even on the one-element to convert it into a tuple
                    let ($($name,)*) = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                    Ok(Self($($name,)*))
                }
            
            }
            
            
        };
    }

    #[allow(unused_macros)] // This is just a hint at what you could do. I don't have a need for it right now though.
    macro_rules! impl_simple_serde_keyed_struct {

        ($struct: ty {$first_name: ident $(,$name: ident)*$(,)?}) => {
            impl $crate::utils::simple_serde::Serialize for $struct {
        
                fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                    let Self{$first_name $(,$name)*} = self;
                    serializer.write_token(Token::OpenBrace);
                    $first_name.write_value(serializer);
                    $(
                        serializer.write_token(Token::Comma);
                        $name.write_value(serializer);
                    )*
                    serializer.write_token(Token::CloseBrace);
                }
            }

            impl $crate::utils::simple_serde::Deserialize for $struct {
            
                fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                    source.expect(Token::OpenBrace)?;
                    let $first_name = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                    $(
                        source.expect(Token::Comma)?;
                        let $name = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                    )*
                    Ok(Self{
                        $first_name,
                        $(,$name)*
                    })
                }
            
            }
            
            
        };
    }


    #[cfg(test)]
    mod test {

        use angular_units::Deg;

        use crate::utils::simple_serde::Serialize as SimpleSerialize;
        use crate::utils::simple_serde::Deserialize as SimpleDeserialize;    

        use crate::utils::Edge;
        use crate::world_map::Neighbor; // and vec
        use crate::world_map::NeighborAndDirection; // and vec
        use crate::world_map::Grouping;
        use crate::world_map::RiverSegmentFrom;
        use crate::world_map::RiverSegmentTo;
        use crate::world_map::LakeType;
        use crate::world_map::BiomeCriteria;
        use crate::world_map::CultureType;


        fn test_serializing<Value: SimpleSerialize + SimpleDeserialize + PartialEq + core::fmt::Debug>(value: Value, text: &str) {
            let serialized = value.write_to_string();
            assert_eq!(serialized,text);
            let deserialized = Value::read_from_str(&serialized).unwrap();
            assert_eq!(value,deserialized)
        }


        #[test]
        fn test_serde_edge() {
            test_serializing(Edge::North, "North");
            test_serializing(Edge::Southwest, "Southwest");
        }

        #[test]
        fn test_serde_neighbor() {
            test_serializing(Neighbor::Tile(36), "36");
            test_serializing(Neighbor::CrossMap(42, Edge::East), "(42,East)");
            test_serializing(Neighbor::OffMap(Edge::West), "West");
        }

        #[test]
        fn test_serde_neighbor_vec() {
            test_serializing(vec![Neighbor::Tile(36), Neighbor::CrossMap(42, Edge::East), Neighbor::OffMap(Edge::West)], "[36,(42,East),West]");
            test_serializing::<Vec<Neighbor>>(vec![], "[]");
        }

        #[test]
        fn test_serde_neighbor_and_direction() {
            test_serializing(NeighborAndDirection(Neighbor::Tile(72),Deg(45.6)), "(72,45.6)")
        }

        #[test]
        fn test_serde_neighbor_and_direction_vec() {
            test_serializing(vec![NeighborAndDirection(Neighbor::Tile(72),Deg(45.6)),NeighborAndDirection(Neighbor::CrossMap(49,Edge::Southeast),Deg(0.1))], "[(72,45.6),((49,Southeast),0.1)]")
        }

        #[test]
        fn test_serde_grouping() {
            test_serializing(Grouping::LakeIsland, "LakeIsland")
        }

        #[test]
        fn test_serde_river_segment_from() {
            test_serializing(RiverSegmentFrom::Confluence, "Confluence")
        }

        #[test]
        fn test_serde_river_segment_to() {
            test_serializing(RiverSegmentTo::Mouth, "Mouth")
        }

        #[test]
        fn test_serde_lake_type() {
            test_serializing(LakeType::Fresh, "Fresh")
        }

        #[test]
        fn test_serde_biome_criteria() {
            test_serializing(BiomeCriteria::Glacier, "Glacier");
            test_serializing(BiomeCriteria::Matrix(vec![(23,24),(12,20),(13,4)]), "Matrix([(23,24),(12,20),(13,4)])")
        }

        #[test]
        fn test_serde_culture_type() {
            test_serializing(CultureType::Hunting, "Hunting")
        }




    }


}