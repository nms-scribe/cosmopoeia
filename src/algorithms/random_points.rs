use rand::Rng;

use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::geometry::Point;

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
    random: Random,
    extent: Extent,
    world_shape: WorldShape,
    spacing: f64,
    estimated_points: usize,
    phase: PointGeneratorPhase,

}

impl<Random: Rng> PointGenerator<Random> {
    pub(crate) const START_X: f64 = 0.0;
    // You would think I'd be able to start generating at 0, but that appears to be one pixel below the bottom of the grid on my test.
    // FUTURE: Revisit this, could this have just been bad starting data?
    pub(crate) const START_Y: f64 = 1.0;

    pub(crate) fn new(random: Random, extent: Extent, world_shape: WorldShape, estimated_points: usize) -> Self {
        let density = estimated_points as f64/world_shape.calculate_extent_area(&extent); // number of points per unit square
        let unit_point_count = density.sqrt(); // number of points along a line of unit length
        let spacing = 1.0/unit_point_count; // if there are x points along a unit, then it divides it into x spaces.
        let phase = PointGeneratorPhase::NortheastInfinity;

        Self {
            random,
            extent,
            world_shape,
            spacing,
            estimated_points,
            phase
        }

    }

    pub(crate) fn make_point(&self, x: f64, y: f64) -> Result<Point,CommandError> {
        Point::new(self.extent.west + x,self.extent.south + y)
    }

    fn jitter(random: &mut Random, spacing: f64) -> f64 {
        let jitter_shift = (spacing / 2.0) * 0.9;
        // This is subtracted from the randomly generated jitter so the range is -0.9*spacing to 0.9*spacing
        let jitter_spread = jitter_shift * 2.0;
        // This + jitter_shift causes the jitter to move by up to 0.9*spacing. If it were 1 times spacing, there might 
        let jitter = random.gen::<f64>().mul_add(jitter_spread, -jitter_shift);
        jitter
    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Point,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {

        // Randomizing algorithms borrowed from AFMG with many modifications


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
                let y_spacing = self.spacing;
                if x < &self.extent.width {
                    let x_spacing = self.world_shape.calculate_longitudinal_spacing_for_latitude(self.spacing,*y);
                    let x_jitter = Self::jitter(&mut self.random,x_spacing);
                    let jittered_x = (x + x_jitter).clamp(Self::START_X,self.extent.width);

                    let y_jitter = Self::jitter(&mut self.random, y_spacing);
                    let jittered_y = (y + y_jitter).clamp(Self::START_Y,self.extent.height);

                    self.phase = PointGeneratorPhase::Random(x + x_spacing, *y);
                    Some(self.make_point(jittered_x,jittered_y))
                } else {
                    self.phase = PointGeneratorPhase::Random(Self::START_X, y + y_spacing);
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
        // size_hint is supposed to talk about how many remaining, not a range from start to end
        // but this is still fair, because I don't know how many are remaining and it's too difficult
        // to estimate that. Also, the results are allowed to be buggy.
        (0,Some(self.estimated_points))
    }
}

// the points layer is only created and loaded if we're using the dev- commands, so it's not output by the point generator.
pub(crate) fn load_points_layer<Generator: Iterator<Item=Result<Point,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target_points = target.create_points_layer(overwrite_layer)?;

    // boundary points    

    for point in generator.watch(progress,"Writing points.","Points written.") {
        _ = target_points.add_point(point?)?;
    }

    Ok(())

}


