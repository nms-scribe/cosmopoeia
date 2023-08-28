use std::hash::Hash;

use ordered_float::NotNan;
use ordered_float::FloatIsNan;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::LayerAccess;
use gdal::vector::FeatureIterator;
use gdal::vector::OGRwkbGeometryType;
use adaptive_bezier::adaptive_bezier_curve; 
use adaptive_bezier::Vector2;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;

pub(crate) fn random_number_generator(seed: Option<u64>) -> StdRng {
    let seed = if let Some(seed) = seed {
        seed
    } else {
        // FUTURE: It would be nice if I could print out the seed that is being used so the user can reproduce a map.
        // The only option right now is to generate the seed myself, but rand doesn't publicise the stuff it's using (I suspect that actually makes sense).
        let mut seeder = StdRng::from_entropy();
        let seed = seeder.gen::<u64>();
        println!("Using random seed {}",seed);
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
            west,
            south,
            width,
            height
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

    pub(crate) fn create_geometry(&self) -> Result<Geometry,CommandError> {
        let mut vertices = Vec::new();
        vertices.push((self.west,self.south).try_into()?);
        vertices.push((self.west,self.south+self.height).try_into()?);
        vertices.push((self.west+self.width,self.south+self.height).try_into()?);
        vertices.push((self.west+self.width,self.south).try_into()?);
        create_polygon(&vertices)
    }

    pub(crate) fn create_boundary_geometry(&self) -> Result<Geometry, CommandError> {
        let north = NotNan::try_from(self.north())?;
        let east = NotNan::try_from(self.east())?;
        let west = NotNan::try_from(self.west)?;
        let south = NotNan::try_from(self.south)?;
        let mut border_points = Vec::new();
        border_points.push(Point::new(west,south));
        for y in south.ceil() as usize..north.ceil() as usize {
            border_points.push(Point::new(west,NotNan::try_from(y as f64)?))
        }
        border_points.push(Point::new(west,north));
        for x in west.ceil() as usize..east.floor() as usize {
            border_points.push(Point::new(NotNan::try_from(x as f64)?,north))
        }
        border_points.push(Point::new(east,north));
        for y in north.ceil() as usize..south.floor() as usize {
            border_points.push(Point::new(east,NotNan::try_from(y as f64)?))
        }
        border_points.push(Point::new(east,south));
        for x in east.ceil() as usize..west.floor() as usize {
            border_points.push(Point::new(NotNan::try_from(x as f64)?,south))
        }
        border_points.push(Point::new(west,south));
        let ocean = create_polygon(&border_points)?;
        Ok(ocean)
    }    

    pub(crate) fn east(&self) -> f64 {
        self.west + self.width
    }

    pub(crate) fn north(&self) -> f64 {
        self.south + self.height
    }

}


pub(crate) struct GeometryGeometryIterator {
    geometry: Geometry,
    index: usize

}

impl GeometryGeometryIterator {

    pub(crate) fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            index: 0
        }
    }
}

impl Iterator for GeometryGeometryIterator {
    type Item = Geometry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.geometry.geometry_count() {
            let a = self.geometry.get_geometry(self.index);
            self.index += 1;
            Some(a.clone())
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0,Some(self.geometry.geometry_count()))
    }
}

pub(crate) struct LayerGeometryIterator<'lifetime> {
    count: usize,
    source: FeatureIterator<'lifetime>
}

impl<'lifetime> LayerGeometryIterator<'lifetime> {

    pub(crate) fn new(source: &'lifetime mut Layer) -> Self {
        Self {
            count: source.feature_count() as usize,
            source: source.features()
        }
    }

}

impl<'lifetime> Iterator for LayerGeometryIterator<'lifetime> {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.source.next() {
            if let Some(geometry) = feature.geometry() {
                Some(Ok(geometry.clone()))
            } else {
                Some(Geometry::empty(OGRwkbGeometryType::wkbNone).map_err(Into::into))
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0,Some(self.count))
    }
    
}

pub(crate) trait ToGeometryCollection {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Geometry,CommandError>;
}

impl<Iter: Iterator<Item=Result<Geometry,CommandError>>> ToGeometryCollection for Iter {

    fn to_geometry_collection<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<Geometry,CommandError> {
        let mut result = Geometry::empty(OGRwkbGeometryType::wkbGeometryCollection)?;
        for geometry in self.watch(progress,"Reading geometries.","Geometries read.") {
            result.add_geometry(geometry?)?;
        }
        Ok(result)
    }


}

#[derive(Hash,Eq,PartialEq,Clone,Debug)]
pub(crate) struct Point {
    pub(crate) x: NotNan<f64>,
    pub(crate) y: NotNan<f64>
}

impl Point {

    pub(crate) fn to_tuple(&self) -> (f64,f64) {
        (*self.x,*self.y)
    }

    pub(crate) fn from_f64(x: f64, y: f64) -> Result<Self,FloatIsNan> {
        Ok(Self::new(NotNan::try_from(x)?,NotNan::try_from(y)?))
    }

    pub(crate) fn new(x: NotNan<f64>, y: NotNan<f64>) -> Self {
        Self { x, y }
    }

    pub(crate) fn subtract(&self, other: &Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub(crate) fn normalized(&self) -> Self {
        let length = (self.x * self.x + self.y * self.y).sqrt();
        if length != 0.0 {
            Point::new(self.x / length, self.y / length)
        } else {
            Point::new(NotNan::from(0), NotNan::from(0))
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn multiply(&self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    fn abs(&self) -> f64 {
        // the absolute value of a vector is it's distance from 0,0.
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }

    pub(crate) fn perpendicular(&self, negate_second: bool) -> Self {
        if negate_second {
            Self::new(self.y,-self.x)
        } else {
            Self::new(-self.y,self.x)
        }
    }

    pub(crate) fn distance(&self, other: &Self) -> f64 {
        ((other.x - self.x).powi(2) + (other.y - self.y).powi(2)).sqrt()
    }

    pub(crate) fn middle_point_between(&self, other: &Self) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }

    }

    pub(crate) fn create_geometry(&self) -> Result<Geometry,CommandError> {
        let mut point = Geometry::empty(OGRwkbGeometryType::wkbPoint)?;
        point.add_point_2d((self.x.into(),self.y.into()));
        Ok(point)

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

impl TryFrom<(NotNan<f64>,NotNan<f64>)> for Point {

    type Error = FloatIsNan;

    fn try_from(value: (NotNan<f64>,NotNan<f64>)) -> Result<Self, Self::Error> {
        Ok(Self {
            x: value.0,
            y: value.1
        })
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

impl std::ops::Sub for &Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Add for &Point {
    type Output = Point;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

pub(crate) fn create_polygon(vertices: &Vec<Point>) -> Result<Geometry,CommandError> {
    // close the polygon if necessary
    let mut line = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;
    for point in vertices {
        line.add_point_2d(point.to_tuple())
    
    }
    if vertices.get(0) != vertices.last() {
        // if the vertices don't link up, then link them.
        line.add_point_2d(vertices[0].to_tuple())
    }
    let mut polygon = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
    polygon.add_geometry(line)?;
    Ok(polygon)

}

pub(crate) fn polygon_to_vertices(geometry: &Geometry) -> Result<Vec<Point>, CommandError> {
    let mut input = Vec::new();
    
    if geometry.geometry_count() > 0 {
        let line = geometry.get_geometry(0);
        for i in 0..line.point_count() {
            let (x,y,_) = line.get_point(i as i32);
            input.push(Point::from_f64(x,y)?);
        }
    }
    
    Ok(input)
}

pub(crate) fn bezierify_polygon(geometry: &Geometry, scale: f64) -> Result<Geometry, CommandError> {
    let input = polygon_to_vertices(&geometry)?;
    let bezier = PolyBezier::from_poly_line(&input);
    let geometry = create_polygon(&bezier.to_poly_line(scale)?)?;
    Ok(geometry)
}

// TODO: This should be what bezierify_polygon actually does, but I'll have to test it after I change that.
// NOTE: Theres a small chance that bezierifying will create invalid geometries. These are automatically
// made valid by splitting them into multiple polygons, hence this returns a vec.
pub(crate) fn bezierify_polygon_with_rings(geometry: &Geometry, scale: f64) -> Result<Vec<Geometry>,CommandError> {
    let mut output = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
    for i in 0..geometry.geometry_count() {
        let ring = geometry.get_geometry(i);
        let mut points = Vec::new();
        for i in 0..ring.point_count() {
            let (x,y,_) = ring.get_point(i as i32);
            points.push(Point::from_f64(x,y)?)
        }
        let bezier = PolyBezier::from_poly_line(&points);
        let mut new_ring = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;
        for point in bezier.to_poly_line(scale)? {
            new_ring.add_point_2d(point.to_tuple())
        }
        output.add_geometry(new_ring)?;
    }
    // Primary causes: the original dissolved tiles meet at the same point, or a point that is very close.
    let validate_options = gdal::cpl::CslStringList::new();
    Ok(multipolygon_to_polygons(output.make_valid(&validate_options)?))
}

pub(crate) fn multipolygon_to_polygons(geometry: Geometry) -> Vec<Geometry> {
    if geometry.geometry_type() == OGRwkbGeometryType::wkbMultiPolygon {
        let mut result = Vec::new();
        for i in 0..geometry.geometry_count() {
            result.push(geometry.get_geometry(i).clone())
        }
        result
    } else {
        vec![geometry]
    }
}


pub(crate) fn create_line(vertices: &Vec<Point>) -> Result<Geometry,CommandError> {
    let mut line = Geometry::empty(OGRwkbGeometryType::wkbLineString)?;
    for point in vertices {
        line.add_point_2d(point.to_tuple());
    }
    Ok(line)

}

pub(crate) struct PolyBezier {
    vertices: Vec<Point>,
    controls: Vec<(Point,Point)> // this should always have one less item than vertices.
}


impl PolyBezier {

    #[cfg(test)] pub(crate) fn segment_at(&self, index: usize) -> Option<(&Point,&Point,&Point,&Point)> {
        if index < self.controls.len() {
            let v1 = &self.vertices[index];
            let c = &self.controls[index];
            let v2 = &self.vertices[index + 1];
            Some((v1,&c.0,&c.1,v2))
        } else {
            None
        }
    }

    pub(crate) fn trim_end(self) -> Self {
        let vertices_take = self.vertices.len() - 1;
        let controls_take = self.controls.len() - 1;
        Self {
            vertices: self.vertices.into_iter().take(vertices_take).collect(),
            controls: self.controls.into_iter().take(controls_take).collect(),
        }
    }

    pub(crate) fn trim_start(self) -> Self {
        Self {
            vertices: self.vertices.into_iter().skip(1).collect(),
            controls: self.controls.into_iter().skip(1).collect(),
        }
    }

    pub(crate) fn trim_both(self) -> Self {
        let vertices_take = self.vertices.len() - 1;
        let controls_take = self.controls.len() - 1;
        Self {
            vertices: self.vertices.into_iter().take(vertices_take).skip(1).collect(),
            controls: self.controls.into_iter().take(controls_take).skip(1).collect(),
        }
    }

    // finds a curve from a line where the first points and last points are curved with influence of optional extended points.
    // The curves created by these end segments are not included in the result.
    pub(crate) fn from_poly_line_with_phantoms(phantom_start: Option<Point>, line: &[Point], phantom_end: Option<Point>) -> Self {
        match (phantom_start,phantom_end) {
            (None, None) => Self::from_poly_line(line),
            (None, Some(end)) => {
                let mut vertices = line.to_vec();
                vertices.push(end);
                let result = Self::from_poly_line(&vertices);
                result.trim_end()
            },
            (Some(start), None) => {
                let mut vertices = vec![start];
                vertices.extend(line.into_iter().cloned());
                let result = Self::from_poly_line(&vertices);
                result.trim_start()
            },
            (Some(start), Some(end)) => {
                let mut vertices = vec![start];
                vertices.extend(line.into_iter().cloned());
                vertices.push(end);
                let result = Self::from_poly_line(&vertices);
                result.trim_both()
            },
        }

    }

    pub(crate) fn from_poly_line(vertices: &[Point]) -> Self {
        if vertices.len() < 2 {
            return Self {
                vertices: vertices.to_vec(),
                controls: Vec::new()
            }
        }

        // https://math.stackexchange.com/a/4207568
        /*
        STORY: I had a little artifical help from chatgpt to get the initial translation from python code in 
        the SO answer to rust. As an experience, it was like getting help from an idiot who is good at programming 
        and thinks he's an expert. The initial result looked like real code, and seemed to be doing what it
        was supposed to. 

        But, I would report compilation errors to it and it would say "Oh, sorry about that. This will compile." 
        Except it didn't. Every time it was confidently incorrect.

        It missed out on what was going on. For some reason, the initial translation required the input to be a vector
        of tuples of points, which didn't make sense. At one point it got into a cycle where it decided to fix things 
        by turning the points into tuples, then turning those into points, then back into tuples.

        I finally got the best results by starting over with a new conversation. Then I took the original code from
        stackoverflow, removed all of the plotting stuff to remove confusion, and told it to straight up translate that.
        I then used the engine linked to in the stackoverflow comment to change the code to print out the results, so 
        I could compare, and they were way off.

        I discovered two mistakes I never would have known about if I didn't look through the code carefully. It was like 
        chat decided that one operation was as good as another. The first was how it decided what to add to the start and
        end when the line wasn't a ring. The second was the call to get the absolute value of the point (`vertex.subtract(vertex0).abs()`).

        Even though it had figured out point subtraction, addition and multiplication, it decided that that the original
        code (`abs(p - p0)`) meant to take the absolute values of x and y and add them together. I searched for what it meant
        to get the absolute value of a point, and learned it was the distance from 0. Which meant chat decided that adding
        the values together was the same as adding their squares and then returning the square root.

        What if the difference between real intelligence and artificial intelligence is understanding the pythagorean theorem? What
        if Pythagoras was the person who invented human intelligence?

        The final result got me to almost match the values returned from the python code. The only differences were in the last digits
        and the number of digits returned, so it was just a matter of precision.
        */

        // Make the normalized tangent vectors
        
        // Tangents for interior points are parallel to the lines between the points to either side 
        // (tangent for point B is parallel to the line between A and B), so we need to pair up
        // the vertices as p,p+2. This will create n-2 vertices to match up with interior points.
        let pairs = vertices.iter().zip(vertices.iter().skip(2));
        // tangents for these pairs are found by subtracting the points
        let tangents: Vec<Point> = pairs.map(|(u, v)| v.subtract(u)).collect();
    
        // the start and end tangents are from different pairs.
        let (start,end) = if vertices[0] == vertices[vertices.len() - 1] {
            // this is a polygonal ring, so the points are the same, and the tangents for
            // them are the same. This tangent is parallel to a line from the second point to the penultimate point.
            // ABCDEA => paralell to BE
            // TODO: This is going to error if we have too few points
            let end = vec![vertices[1].subtract(&vertices[vertices.len() - 2])];
            (end.clone(),end)
        } else {
            // otherwise, the start tangent is parallel to a line between the first and second point,
            // and the end tangent the same between the last and penultimate point.
            // ABCDE => parallel to AB and DE
            // start is the difference between the second and first
            let start = vec![vertices[1].subtract(&vertices[0])];
            // end is the difference between the last and second-to-last
            // TODO: This is going to error if we have too few points
            let end = vec![vertices[vertices.len()-1].subtract(&vertices[vertices.len()-2])];
            (start,end)
        };
    
        let tangents = start.iter().chain(tangents.iter()).chain(end.iter());
        // the tangents are normalized -- we just need the direction, not the distance, so this is a unit vector pointing the same direction.
        let tangents = tangents.map(|u| u.normalized());
        let tangents: Vec<Point> = tangents.collect();
    
        // Build Bezier curves
        // zip up the points into pairs with their tangents
        let mut vertex_tangents = vertices.iter().zip(tangents.iter());
        // the first one should always be there? TODO: What if we were given no points as input?
        let (mut vertex0, mut tangent0) = vertex_tangents.next().unwrap();
        let mut controls = Vec::new();
        for (vertex, tangent) in vertex_tangents {
            // original code: s = abs(p - p0) / 3 -- the absolute value for a point is the distance from 0.
            let s = vertex.subtract(vertex0).abs() / 3.0;
            controls.push((
                // control point from previous point, on its tangent, 1/3 along the way between the two points
                vertex0.add(&tangent0.multiply(s)),
                // control point for the next point, on its tangent, 1/3 along the way
                vertex.subtract(&tangent.multiply(s))
            ));
    
            vertex0 = vertex;
            tangent0 = tangent;
        }
        Self { 
            vertices: vertices.to_vec(), 
            controls 
        }
    }

    pub(crate) fn to_poly_line(&self, scale: f64) -> Result<Vec<Point>,CommandError> {
        // I don't just want to put equally spaced points, I want what appears to be called an adaptive bezier:
        // https://agg.sourceforge.net/antigrain.com/research/adaptive_bezier/index.html 
        // I found a Javascript translation of that here: https://github.com/mattdesl/adaptive-bezier-curve, 
        // I also found a rust translation of that javascript translation (https://crates.io/crates/adaptive-bezier).
        // I'm not comfortable with it, since it uses it's own vector structure which pulls in a huge library,
        // but it works, so.... 
        // TODO: However, it might be nice to use this Vector2 structure for points anyway. I'm basically reproducing
        // a lot of it anyway.
        let mut result = Vec::new();
        let mut vertices = self.vertices.iter();
        let mut controls = self.controls.iter();
        if let Some(vertex1) = vertices.next() {
            let mut vertex1 = vertex1;
            result.push(vertex1.clone());
            while let Some(vertex2) = vertices.next() {
                if let Some((c1,c2)) = controls.next() {
                    let curve = adaptive_bezier_curve(
                        Vector2::new(*vertex1.x,*vertex1.y),
                        Vector2::new(*c1.x,*c1.y),
                        Vector2::new(*c2.x,*c2.y),
                        Vector2::new(*vertex2.x,*vertex2.y),
                        scale
                    );
                    // convert back to points.
                    for point in curve.iter().take(curve.len() - 2).skip(1) {
                        result.push(Point::from_f64(point[0], point[1])?);
                    }
                }
                result.push(vertex2.clone());
                vertex1 = vertex2;
            }

        };

        Ok(result)

    }

}

pub(crate) fn find_curve_making_point(start_point: &Point, end_point: &Point) -> Point {
    // This function creates a phantom point which can be used to give an otherwise straight ending segment a bit of a curve.
    let parallel = start_point.subtract(end_point);
    // I want to switch the direction of the curve in some way that looks random, but is reproducible.
    // The easiest way I can think of is basically to base it off of whether the integral part of a value is even.
    let is_even = start_point.x.rem_euclid(2.0) < 1.0;
    let perpendicular = parallel.perpendicular(is_even);
    let normalized = perpendicular.normalized();
    end_point.add(&normalized)
}

mod title_case {

    use std::fmt;

    pub struct AsTitleCase<StringType: AsRef<str>>(StringType);

    impl<T: AsRef<str>> fmt::Display for AsTitleCase<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

            let source: &str = self.0.as_ref();
            
            let mut first = true;
            for word in source.split(' ') {
                if !first {
                    write!(f," ")?;
                } else {
                    first = false;
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

    pub trait ToTitleCase: ToOwned {
        /// Convert this type to title case.
        fn to_title_case(&self) -> Self::Owned;
    }

    impl ToTitleCase for str {
        fn to_title_case(&self) -> String {
            AsTitleCase(self).to_string()
        }
    }


}

pub(crate) use title_case::ToTitleCase;

pub(crate) mod namers_pretty_print {


    use std::io;

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
    pub struct PrettyFormatter<'a> {
        current_indent: usize,
        has_value: bool,
        array_nesting: usize,
        indent: &'a [u8],
    }

    impl<'a> PrettyFormatter<'a> {
        /// Construct a pretty printer formatter that defaults to using two spaces for indentation.
        pub fn new() -> Self {
            PrettyFormatter::with_indent(b"  ")
        }

        /// Construct a pretty printer formatter that uses the `indent` string for indentation.
        pub fn with_indent(indent: &'a [u8]) -> Self {
            PrettyFormatter {
                current_indent: 0,
                has_value: false,
                array_nesting: 0,
                indent,
            }
        }
    }

    impl<'a> Default for PrettyFormatter<'a> {
        fn default() -> Self {
            PrettyFormatter::new()
        }
    }

    impl<'a> serde_json::ser::Formatter for PrettyFormatter<'a> {
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
    // FUTURE: This was an implementation I found on crates.io that allowed inserting and floating point points, and wasn't too difficult to construct. Although that could be done better. It didn't have a lot of downloads, however, so I don't know if it's really something I should be using.
    use qutee::QuadTree; 
    use qutee::Boundary;

    use super::Extent;
    use super::Point;
    use crate::errors::CommandError;

    pub(crate) struct PointFinder {
      // It's kind of annoying, but the query method doesn't return the original point, so I have to store the point.
      inner: QuadTree<f64,Point>,
      bounds: Boundary<f64>, // it also doesn't give us access to this
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
            self.inner.insert_at(point.to_tuple(),point.clone()).map_err(|_| CommandError::PointFinderOutOfBounds(point.x.into(),point.y.into()))

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
            return false;

        }

        pub(crate) fn fill_from(other: &PointFinder, additional_size: usize) -> Result<Self,CommandError> {
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
    

}

pub(crate) trait TryGetMap<Key,Value> {

    fn try_get(&self, key: &Key) -> Result<&Value,CommandError>;

    fn try_get_mut(&mut self, key: &Key) -> Result<&mut Value,CommandError>;

}

