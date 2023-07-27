use std::hash::Hash;

use ordered_float::NotNan;
use ordered_float::FloatIsNan;
use rand::rngs::StdRng;
use rand::SeedableRng;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::LayerAccess;
use gdal::vector::FeatureIterator;
use gdal::vector::OGRwkbGeometryType::wkbNone;
use gdal::vector::OGRwkbGeometryType::wkbGeometryCollection;
use gdal::vector::OGRwkbGeometryType::wkbPolygon;
use gdal::vector::OGRwkbGeometryType::wkbLinearRing;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;

pub(crate) fn random_number_generator(seed_vec: Vec<u8>) -> StdRng {
    if seed_vec.len() > 0 {
        let mut seeds = [0u8; 32];
        for (&x, p) in seed_vec.iter().zip(seeds.iter_mut()) {
            *p = x;
        }
        StdRng::from_seed(seeds)
    } else {
        // FUTURE: It would be nice if I could print out the seed that is being used so the user can reproduce a map.
        // But this doesn't do it. The only option right now is to generate the seed myself, but rand doesn't publicise the
        // stuff it's using.
        StdRng::from_entropy()
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
        create_polygon(vertices)
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
                Some(Geometry::empty(wkbNone).map_err(Into::into))
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
        progress.start(|| ("Reading geometries.",self.size_hint().1));
        let mut result = Geometry::empty(wkbGeometryCollection)?;
        for (i,geometry) in self.enumerate() {
            result.add_geometry(geometry?)?;
            progress.update(|| i);
        }
        progress.finish(|| "Geometries read.");
        Ok(result)
    }


}

#[derive(Hash,Eq,PartialEq,Clone)]
pub(crate) struct Point {
    pub(crate) x: NotNan<f64>,
    pub(crate) y: NotNan<f64>
}

impl Point {

    pub(crate) fn to_tuple(&self) -> (f64,f64) {
        (*self.x,*self.y)
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

pub(crate) fn create_polygon(vertices: Vec<Point>) -> Result<Geometry,CommandError> {
    // close the polygon if necessary
    let mut vertices = vertices;
    if vertices.get(0) != vertices.last() {
        vertices.push(vertices[0].clone())
    }
    let mut line = Geometry::empty(wkbLinearRing)?;
    for point in vertices {
        line.add_point_2d(point.to_tuple())
    }
    let mut polygon = Geometry::empty(wkbPolygon)?;
    polygon.add_geometry(line)?;
    Ok(polygon)

}
