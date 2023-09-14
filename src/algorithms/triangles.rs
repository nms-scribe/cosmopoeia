use gdal::vector::Geometry;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::utils::GeometryGeometryIterator;
use crate::world_map::WorldMapTransaction;

pub(crate) enum DelaunayGeneratorPhase {
    Unstarted(Geometry),
    Started(GeometryGeometryIterator),
    Done
}

pub(crate) struct DelaunayGenerator {
    pub(crate) phase: DelaunayGeneratorPhase

}

impl DelaunayGenerator {

    pub(crate) fn new(source: Geometry) -> Self {
        let phase = DelaunayGeneratorPhase::Unstarted(source);
        Self {
            phase
        }
    }

    // this function is optional to call, it will automatically be called by the iterator.
    // However, that will cause a delay to the initial return.
    pub(crate) fn start<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {
        // NOTE: the delaunay thingie can only work if all of the points are known, so we can't work with an iterator here.
        // I'm not certain if some future algorithm might allow us to return an iterator, however.
        if let DelaunayGeneratorPhase::Unstarted(source) = &mut self.phase {
            // the delaunay_triangulation procedure requires a single geometry. Which means I've got to read all the points into one thingie.
            // FUTURE: Would it be more efficient to have my own algorithm which outputs triangles as they are generated?
            progress.start_unknown_endpoint(|| "Generating triangles.");
            let triangles = source.delaunay_triangulation(None)?;
            progress.finish(|| "Triangles generated.");
            self.phase = DelaunayGeneratorPhase::Started(GeometryGeometryIterator::new(triangles))
        }
        Ok(())
    }

}

impl Iterator for DelaunayGenerator {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.phase {
            DelaunayGeneratorPhase::Unstarted(_) => {
                match self.start(&mut ()) { 
                    Ok(_) => self.next(),
                    Err(e) => Some(Err(e)),
                }
            },
            DelaunayGeneratorPhase::Started(iter) => if let Some(value) = iter.next() {
                Some(Ok(value))
            } else {
                self.phase = DelaunayGeneratorPhase::Done;
                None
            },
            DelaunayGeneratorPhase::Done => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.phase {
            DelaunayGeneratorPhase::Unstarted(geometry) => (0,Some(geometry.geometry_count())),
            DelaunayGeneratorPhase::Started(iterator) => iterator.size_hint(),
            DelaunayGeneratorPhase::Done => (0,None),
        }
        
    }
}

pub(crate) fn load_triangles_layer<Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target = target.create_triangles_layer(overwrite_layer)?;

    for triangle in generator.watch(progress,"Writing triangles.","Triangles written.") {
        target.add_triangle(triangle?.to_owned())?;
    }

    Ok(())

}

