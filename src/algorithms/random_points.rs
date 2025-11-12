use rand::Rng;

use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator as _;
use crate::geometry::Point;

pub(crate) enum PointGeneratorPhase {
    NortheastInfinity,
    SoutheastInfinity,
    SouthwestInfinity,
    NorthwestInfinity,
    Random{ 
        x: f64, 
        y: f64,
        x_spacing: f64
    },
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
        let density = estimated_points as f64/extent.shaped_area(&world_shape); // number of points per unit square
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
        let result = Point::new(self.extent.west() + x,self.extent.south() + y)?;
        Ok(result)
    }

    fn jitter(random: &mut Random, spacing: f64) -> f64 {
        let jitter_shift = (spacing / 2.0) * 0.9;
        // This is subtracted from the randomly generated jitter so the range is -0.9*spacing to 0.9*spacing
        let jitter_spread = jitter_shift * 2.0;
        // This + jitter_shift causes the jitter to move by up to 0.9*spacing. If it were 1 times spacing, there might 
        random.gen::<f64>().mul_add(jitter_spread, -jitter_shift)
    }

    /**
    Calculates the spherical spacing of random points on a specific row of random points, given a standard spacing. 
    
    The calculation requires taking the reciprocal of the cosine of the latitude. In order to avoid a division by 0 or negative value, a result below f64::EPSILON is maximized to that. This simply creates a very large spacing that likely results in no points being generated at all.
    */
    pub(crate) fn spherical_spacing(&self, y: f64) -> f64 {
        /* 
        What I want it to do here is increase spacing depending on the line of latitude. Think of the intial spacing as being
        in units of 1 degree at the equator. The spherical length of that spacing does not change, but it's ratio to the length
        of the latitude does. 

        So, what I want is a ratio of the length of the equator to the specified line of latitude. This will be the ratio that the spacing increases by.
            R = Le/L(phi)
            where: 
                Le is the length of the equator
                L(phi) is the length of a given line of latitude

        The length of the equator is constant 360 degrees.

        The length of a degree of longitude at a given latitude is:
            L = (π/180) * a * cos(phi)
            where: 
                a is the radius of the sphere
                phi is the latitude

        There are 360 degrees of longitude in a line of latitude. So, the length of a line of latitude is:
            L = (π/180) * 360 * a * cos(phi)
            L = 2π * a * cos(phi)
        
        I don't have a radius. But I do have the circumference. Since I'm measuring these values in degrees, the circumference of the world is 360. And I can get the radius from that:
            a = C / (2 * π)
            a = 360 / (2 * π)
            a = 180 / π

        Now, solve for L given that value for a:
            L = 2π * (180 / π) * cos(phi)
            L = 2 * 180 * cos(phi)
            L = 360 * cos(phi)

        Finally, we have the ratio:
            R = 360/L
            R = 360/(360 * cos(phi))
            R = 1/cos(phi)

        */
        let lat = self.extent.south() + y; // remember, 'y' is not the actual latitude.
        let length_of_lat_degree = lat.to_radians().cos().max(f64::EPSILON);
        let ratio = length_of_lat_degree.recip();
        ratio * self.spacing
    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Point,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {

        // Randomizing algorithms borrowed from AFMG with many modifications

        macro_rules! init_x_spacing {
            ($y: ident) => {
                match self.world_shape {
                    WorldShape::Cylinder => self.spacing,
                    WorldShape::Sphere => self.spherical_spacing($y)
                }
            };
        }


        match &self.phase { 
            PointGeneratorPhase::NortheastInfinity => {
                self.phase = PointGeneratorPhase::SoutheastInfinity;
                Some(self.make_point(self.extent.width()*2.0, self.extent.height()*2.0))
            },
            PointGeneratorPhase::SoutheastInfinity => {
                self.phase = PointGeneratorPhase::SouthwestInfinity;
                Some(self.make_point(self.extent.width()*2.0, -self.extent.height()))
            },
            PointGeneratorPhase::SouthwestInfinity => {
                self.phase = PointGeneratorPhase::NorthwestInfinity;
                Some(self.make_point(-self.extent.width(), -self.extent.height()))
            },
            PointGeneratorPhase::NorthwestInfinity => {
                let y = Self::START_Y;
                let x_spacing = init_x_spacing!(y);
                self.phase = PointGeneratorPhase::Random{ 
                    x: Self::START_X + (x_spacing/2.0), 
                    y,
                    x_spacing
                };
                Some(self.make_point(-self.extent.width(), self.extent.height()*2.0))
            },
            PointGeneratorPhase::Random{x, y, x_spacing} => if y < &self.extent.height() {
                let y_spacing = self.spacing;
                if x < &self.extent.width() {
                    // if x_spacing is None, then we are at the poles. I want to skip that.
                    
                    let x_jitter = Self::jitter(&mut self.random,*x_spacing);
                    let jittered_x = (x + x_jitter).clamp(Self::START_X,self.extent.width());

                    let y_jitter = Self::jitter(&mut self.random,y_spacing);
                    let jittered_y = (y + y_jitter).clamp(Self::START_Y,self.extent.height());

                    self.phase = PointGeneratorPhase::Random{
                        x: x + x_spacing, 
                        y: *y,
                        x_spacing: *x_spacing
                    };
                    Some(self.make_point(jittered_x,jittered_y))
                } else {

                    let y = y + y_spacing;
                    let next_x_spacing = init_x_spacing!(y);
    
                    self.phase = PointGeneratorPhase::Random{
                        x: Self::START_X + (next_x_spacing/2.0), 
                        y,
                        x_spacing: next_x_spacing
                    };
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


