use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbPoint;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Extent;
use crate::utils::GeometryGeometryIterator;

pub(crate) const DEFAULT_POINT_COUNT: f64 = 10_000.0;

enum PointGeneratorPhase {
    Top(f64),
    Bottom(f64),
    Left(f64),
    Right(f64),
    Random(f64,f64),
    Done
}

/// FUTURE: This one would be so much easier to read if I had real Function Generators.
pub(crate) struct PointGenerator<Random: Rng> {
    random: Random,
    extent: Extent,
    spacing: f64,
    offset: f64,
    boundary_width: f64,
    boundary_height: f64,
    boundary_count_x: f64,
    boundary_count_y: f64,
    radius: f64,
    jittering: f64,
    double_jittering: f64,
    phase: PointGeneratorPhase,
    
}

impl<Random: Rng> PointGenerator<Random> {

    const INITIAL_INDEX: f64 = 0.5;

    pub(crate) fn default_spacing_for_extent(spacing: Option<f64>, extent: &Extent) -> f64 {
        if let Some(spacing) = spacing {
            spacing
        } else {
            ((extent.width * extent.height)/DEFAULT_POINT_COUNT).sqrt().round_hundredths()
        }
        
    }

    pub(crate) fn new(random: Random, extent: Extent, spacing: f64) -> Self {
        let offset = -1.0 * spacing; 
        let boundary_spacing: f64 = spacing * 2.0; 
        let boundary_width = extent.width - offset * 2.0; 
        let boundary_height = extent.height - offset * 2.0; 
        let boundary_count_x = (boundary_width/boundary_spacing).ceil() - 1.0; 
        let boundary_count_y = (boundary_height/boundary_spacing).ceil() - 1.0; 
        let radius = spacing / 2.0; // FUTURE: Why is this called 'radius'?
        let jittering = radius * 0.9; // FUTURE: Customizable factor?
        let double_jittering = jittering * 2.0;
        let phase = PointGeneratorPhase::Top(Self::INITIAL_INDEX); 

        Self {
            random,
            extent,
            spacing,
            offset,
            boundary_width,
            boundary_height,
            boundary_count_x,
            boundary_count_y,
            radius,
            jittering,
            double_jittering,
            phase
        }

    }

    fn estimate_points(&self) -> usize {
        (self.boundary_count_x.floor() as usize * 2) + (self.boundary_count_y.floor() as usize * 2) + ((self.extent.width/self.spacing).floor() as usize * (self.extent.height/self.spacing).floor() as usize)
    }

    fn make_point(&self, x: f64, y: f64) -> Result<Geometry,CommandError> {
        let mut point = Geometry::empty(wkbPoint)?;
        point.add_point_2d((self.extent.west + x,self.extent.south + y));
        Ok(point)
    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: The points laying beyond the edge of the heightmap looks weird. Once I get to the voronoi, see if those are absolutely necessary.
        // TODO: Those boundary points should also be jittered, at least along the line.

        // Randomizing algorithms borrowed from AFMG with many modifications


        macro_rules! horizontal {
            ($index: ident, $this_phase: ident, $next_phase: ident, $y: expr) => {
                if $index < self.boundary_count_x {
                    let x = ((self.boundary_width * $index)/self.boundary_count_x + self.offset).ceil(); 
                    self.phase = PointGeneratorPhase::$this_phase($index + 1.0);
                    Some(self.make_point(x,$y)) 
                } else {
                    self.phase = PointGeneratorPhase::$next_phase(Self::INITIAL_INDEX);
                    self.next()
                }
            };
        }

        macro_rules! vertical {
            ($index: ident, $this_phase: ident, $next_phase: expr, $x: expr) => {
                if $index < self.boundary_count_y {
                    let y = ((self.boundary_height * $index)/self.boundary_count_y + self.offset).ceil(); 
                    self.phase = PointGeneratorPhase::$this_phase($index + 1.0);
                    Some(self.make_point($x,y))
                } else {
                    self.phase = $next_phase;
                    self.next()
                }                
            };
        }

        macro_rules! jitter {
            () => {
                // gen creates random number between >= 0.0, < 1.0
                self.random.gen::<f64>() * self.double_jittering - self.jittering    
            };
        }

        match self.phase {
            PointGeneratorPhase::Top(index) => horizontal!(index,Top,Bottom,self.offset),
            PointGeneratorPhase::Bottom(index) => horizontal!(index,Bottom,Left,self.boundary_height + self.offset),
            PointGeneratorPhase::Left(index) => vertical!(index,Left,PointGeneratorPhase::Right(Self::INITIAL_INDEX),self.offset),
            PointGeneratorPhase::Right(index) => vertical!(index,Right,PointGeneratorPhase::Random(self.radius,self.radius),self.boundary_width+ self.offset),
            PointGeneratorPhase::Random(x, y) => if y < self.extent.height {
                if x < self.extent.width {
                    let x_j = (x + jitter!()).round_hundredths().min(self.extent.width);
                    let y_j = (y + jitter!()).round_hundredths().min(self.extent.height);
                    self.phase = PointGeneratorPhase::Random(x + self.spacing, y);
                    Some(self.make_point(x_j,y_j))
                } else {
                    self.phase = PointGeneratorPhase::Random(self.radius, y + self.spacing);
                    self.next()
                }
                
            } else {
                self.phase = PointGeneratorPhase::Done;
                self.next()
            },
            PointGeneratorPhase::Done => None,
        }

    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0,Some(self.estimate_points()))
    }
}

enum DelaunayGeneratorPhase {
    Unstarted(Geometry),
    Started(GeometryGeometryIterator),
    Done
}

pub(crate) struct DelaunayGenerator {
    phase: DelaunayGeneratorPhase

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
    pub(crate) fn start(&mut self) -> Result<(),CommandError> {
        // NOTE: the delaunay thingie can only work if all of the points are known, so we can't work with an iterator here.
        // I'm not certain if some future algorithm might allow us to return an iterator, however.
        if let DelaunayGeneratorPhase::Unstarted(source) = &mut self.phase {
            // the delaunay_triangulation procedure requires a single geometry. Which means I've got to read all the points into one thingie.
            // FUTURE: Would it be more efficient to have my own algorithm which outputs triangles as they are generated?
            let triangles = source.delaunay_triangulation(None)?; // FUTURE: Should this be configurable?
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
                match self.start() {
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
}



