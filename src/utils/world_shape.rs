use clap::ValueEnum;

// This is used for shape calculations.
use crate::errors::CommandError;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Serialize;
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
