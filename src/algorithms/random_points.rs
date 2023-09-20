use rand::Rng;
use gdal::vector::Geometry;

use crate::utils::Extent;
use crate::utils::Point;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;

pub(crate) enum PointGeneratorPhase {
    NortheastInfinity,
    SoutheastInfinity,
    SouthwestInfinity,
    NorthwestInfinity,
    Random(f64,f64),
    Done
}

/// FUTURE: This one would be so much easier to read if I had real Function Generators. However, even in unstable rust, they are only intended for closures.
pub(crate) struct PointGenerator<Random: Rng> {
    pub(crate) random: Random,
    pub(crate) extent: Extent,
    pub(crate) spacing: f64,
    pub(crate) jitter_shift: f64,
    pub(crate) jitter_spread: f64,
    pub(crate) phase: PointGeneratorPhase,

}

impl<Random: Rng> PointGenerator<Random> {
    pub(crate) const START_X: f64 = 0.0;
    // You would think I'd be able to start generating at 0, but that appears to be one pixel below the bottom of the grid on my test.
    // FUTURE: Revisit this, could this have just been bad starting data?
    pub(crate) const START_Y: f64 = 1.0;

    pub(crate) fn new(random: Random, extent: Extent, est_point_count: usize) -> Self {
        let density = est_point_count as f64/(extent.width*extent.height); // number of points per unit square
        let unit_point_count = density.sqrt(); // number of points along a line of unit length
        let spacing = 1.0/unit_point_count; // if there are x points along a unit, then it divides it into x spaces.
        let jitter_shift = (spacing / 2.0) * 0.9; // This is subtracted from the randomly generated jitter so the range is -0.9*spacing to 0.9*spacing
        let jitter_spread = jitter_shift * 2.0; // This + jitter_shift causes the jitter to move by up to 0.9*spacing. If it were 1 times spacing, there might be some points on top of each other (although this would probably be rare)
        let phase = PointGeneratorPhase::NortheastInfinity;

        Self {
            random,
            extent,
            spacing,
            jitter_shift,
            jitter_spread,
            phase
        }

    }

    pub(crate) fn estimate_points(&self) -> usize {
        ((self.extent.width/self.spacing).floor() as usize * (self.extent.height/self.spacing).floor() as usize) + 4
    }

    pub(crate) fn make_point(&self, x: f64, y: f64) -> Result<Geometry,CommandError> {
        Point::try_from((self.extent.west + x,self.extent.south + y))?.create_geometry()
    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {

        // Randomizing algorithms borrowed from AFMG with many modifications


        macro_rules! jitter {
            () => {
                // gen creates random number between >= 0.0, < 1.0
                self.random.gen::<f64>().mul_add(self.jitter_spread, -self.jitter_shift)
            };
        }

        match &self.phase { 
            PointGeneratorPhase::NortheastInfinity => {
                self.phase = PointGeneratorPhase::SoutheastInfinity;
                Some(self.make_point(self.extent.width*2.0, self.extent.height*2.0))
            },
            PointGeneratorPhase::SoutheastInfinity => {
                self.phase = PointGeneratorPhase::SouthwestInfinity;
                Some(self.make_point(self.extent.width*2.0, -self.extent.height))
            },
            PointGeneratorPhase::SouthwestInfinity => {
                self.phase = PointGeneratorPhase::NorthwestInfinity;
                Some(self.make_point(-self.extent.width, -self.extent.height))
            },
            PointGeneratorPhase::NorthwestInfinity => {
                self.phase = PointGeneratorPhase::Random(Self::START_X,Self::START_Y);
                Some(self.make_point(-self.extent.width, self.extent.height*2.0))
            },
            PointGeneratorPhase::Random(x, y) => if y < &self.extent.height {
                if x < &self.extent.width {
                    let x_j = (x + jitter!()).clamp(Self::START_X,self.extent.width);
                    let y_j = (y + jitter!()).clamp(Self::START_Y,self.extent.height);
                    self.phase = PointGeneratorPhase::Random(x + self.spacing, *y);
                    Some(self.make_point(x_j,y_j))
                } else {
                    self.phase = PointGeneratorPhase::Random(Self::START_X, y + self.spacing);
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

// the points layer is only created and loaded if we're using the dev- commands, so it's not output by the point generator.
pub(crate) fn load_points_layer<Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target_points = target.create_points_layer(overwrite_layer)?;

    // boundary points    

    for point in generator.watch(progress,"Writing points.","Points written.") {
        _ = target_points.add_point(point?)?;
    }

    Ok(())

}


