use clap::ValueEnum;

// This is used for shape calculations.
use crate::errors::CommandError;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Serialize;
use crate::impl_simple_serde_tagged_enum;


/// Specifies the methods to be used for calculating geographic information about coordinates and polygons.
#[derive(Clone,ValueEnum)]
pub enum WorldShape {
    /// This world wraps around so that west and east meet (at 180E,180W), and weird dimensional distortions cause the north and south bounds (90N, 90S) to meet at a single point. This is the simplest representation of a world, and is fine for small regions near the middle of the world, but gets weird further north and south. It is also good for representing a flat world.
    Cylinder,
    /// A world on a sphere, with longitude distances becoming closer together at higher latitudes until they reach the poles. This is not quite the same as Earth, but it is close enough.
    Sphere

    // NOTE: I'm not planning to ever support Elipsoids. Cosmopoeia is not a scientific model, and doesn't require that much precision.

}

impl_simple_serde_tagged_enum!{
    WorldShape {
        Cylinder,
        Sphere
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
