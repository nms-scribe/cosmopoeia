use rand::rngs::StdRng;
use rand::SeedableRng;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::LayerAccess;
use gdal::vector::FeatureIterator;
use gdal::vector::OGRwkbGeometryType::wkbNone;
use gdal::vector::OGRwkbGeometryType::wkbGeometryCollection;

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
        StdRng::from_entropy()
    }
}

pub(crate) trait RoundHundredths {

    fn round_hundredths(&self) -> Self;
}

impl RoundHundredths for f64 {

    fn round_hundredths(&self) -> Self {
        (self * 100.0).round() / 100.0
    }
}

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