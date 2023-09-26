use core::hash::Hash;
use core::str::FromStr;
use core::fmt::Display;
use core::cmp::Ordering; 

use ordered_float::NotNan;
use ordered_float::FloatIsNan;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use rand_distr::uniform::SampleUniform;
use serde::Deserialize;
use serde::Serialize;
use adaptive_bezier::Vector2;


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

impl core::ops::Sub for &Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl core::ops::Add for &Point {
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
