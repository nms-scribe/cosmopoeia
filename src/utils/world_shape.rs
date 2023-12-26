use clap::ValueEnum;
use angular_units::Deg;
use angular_units::Angle;

// This is used for shape calculations.
use crate::errors::CommandError;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Serialize;
use crate::utils::extent::Extent;
use crate::impl_simple_serde_tagged_enum;
use crate::utils::coordinates::Coordinates;


/// Specifies the methods to be used for calculating geographic information about coordinates and polygons.
#[derive(Clone,ValueEnum)]
pub enum WorldShape {
    /// This world wraps around so that west and east meet (at 180E,180W), and weird dimensional distortions cause the north and south bounds (90N, 90S) to meet at a single point. This is the simplest representation of a world, and is fine for small regions near the middle of the world, but gets weird further north and south. It is also good for representing a flat world. 
    Cylinder
    // TODO: Sphere - this is much closer to reality
    // FUTURE: Elipsoid - this is the closest to reality, but complex to do, even distance calculations are weird.
}
impl WorldShape {
    pub(crate) fn calculate_distance_between(&self, from: &Coordinates, to: &Coordinates) -> f64 {
        match self {
            Self::Cylinder => {
                (to.x.into_inner() - from.x.into_inner()).hypot(to.y.into_inner() - from.y.into_inner())
                // (x.hypot(y) = (x.powi(2) + y.powi(2)).sqrt();
                // (other.x - self.x).hypot(other.y - self.y) = ((other.x - self.x).powi(2) + (other.y - self.y).powi(2)).sqrt() 
            },
        }
    }

    pub(crate) fn calculate_midpoint_between(&self, from: &Coordinates, other: &Coordinates) -> Coordinates {
        match self {
            Self::Cylinder => {
                Coordinates {
                    x: (from.x + other.x) / 2.0,
                    y: (from.y + other.y) / 2.0,
                }
        
        
            }
        }
    }

    pub(crate) fn calculate_circumcenter(&self, points: (&Coordinates, &Coordinates, &Coordinates)) -> Coordinates {
        match self {
            Self::Cylinder => {
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
            // TODO: For sphere: https://www.redblobgames.com/x/1842-delaunay-voronoi-sphere/ and https://gamedev.stackexchange.com/questions/60630/how-do-i-find-the-circumcenter-of-a-triangle-in-3d
        }
    }

    pub(crate) fn calculate_bearing(&self, site_x: f64, site_y: f64, neighbor_site_x: f64, neighbor_site_y: f64) -> Deg<f64> {
        match self {
            Self::Cylinder => {
                // needs to be clockwise, from the north, with a value from 0..360

                // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                let counter_clockwise_from_east = Deg(((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round());
                // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                let clockwise_from_north = Deg(450.0) - counter_clockwise_from_east; 

                // And, the Deg structure allows me to normalize it
                clockwise_from_north.normalize()

            }
        }
        // TODO: https://math.stackexchange.com/questions/2688803/angle-between-two-points-on-a-sphere
    }

    pub(crate) fn estimate_average_tile_area(&self, extent: Extent, tiles: usize) -> f64 {
        match self {
            Self::Cylinder => {
                (extent.width*extent.height)/tiles as f64        
            }
        }
    }
}

impl_simple_serde_tagged_enum!{
    WorldShape {
        Cylinder
    }
}

impl From<&WorldShape> for String {

    fn from(value: &WorldShape) -> Self {
        // store as tuple for simplicity
        value.write_to_string()
    }
}

impl TryFrom<String> for WorldShape {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // store as tuple for simplicity
        Deserialize::read_from_str(&value)
    }
}
