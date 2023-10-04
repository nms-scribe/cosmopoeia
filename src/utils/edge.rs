use angular_units::Deg;

use crate::impl_simple_serde_tagged_enum;
use crate::errors::CommandError;
use crate::utils::extent::Extent;

#[derive(Clone,PartialEq,Eq,Hash,PartialOrd,Ord,Debug)]
pub enum Edge {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest
}

impl Edge {

    pub(crate) fn combine_with(&self, edge: Self) -> Result<Self,CommandError> {
        match (&edge,self) {
            (Self::North, Self::Northeast) |
            (Self::Northeast, Self::North) |
            (Self::East, Self::North) |
            (Self::East, Self::Northeast) |
            (Self::Northeast, Self::East) |
            (Self::North, Self::East) => Ok(Self::Northeast),
            (Self::North, Self::West) |
            (Self::West, Self::Northwest) |
            (Self::Northwest, Self::North) |
            (Self::Northwest, Self::West) |
            (Self::West, Self::North) |
            (Self::North, Self::Northwest) => Ok(Self::Northwest),
            (Self::East, Self::South) |
            (Self::Southeast, Self::East) |
            (Self::Southeast, Self::South) |
            (Self::South, Self::East) |
            (Self::South, Self::Southeast) |
            (Self::East, Self::Southeast) => Ok(Self::Southeast),
            (Self::South, Self::West) |
            (Self::Southwest, Self::South) |
            (Self::Southwest, Self::West) |
            (Self::West, Self::Southwest) |
            (Self::West, Self::South) |
            (Self::South, Self::Southwest) => Ok(Self::Southwest),
            (Self::North, Self::North) |
            (Self::Northeast, Self::Northeast) |
            (Self::East, Self::East) |
            (Self::Southeast, Self::Southeast) |
            (Self::South, Self::South) |
            (Self::Southwest, Self::Southwest) |
            (Self::West, Self::West) |
            (Self::Northwest, Self::Northwest) => Ok(edge),
            (Self::North, Self::Southeast) |
            (Self::North, Self::South) |
            (Self::North, Self::Southwest) |
            (Self::Northeast, Self::Southeast) |
            (Self::Northeast, Self::South) |
            (Self::Northeast, Self::Southwest) |
            (Self::Northeast, Self::West) |
            (Self::Northeast, Self::Northwest) |
            (Self::East, Self::Southwest) |
            (Self::East, Self::West) |
            (Self::East, Self::Northwest) |
            (Self::Southeast, Self::North) |
            (Self::Southeast, Self::Northeast) |
            (Self::Southeast, Self::Southwest) |
            (Self::Southeast, Self::West) |
            (Self::Southeast, Self::Northwest) |
            (Self::South, Self::North) |
            (Self::South, Self::Northeast) |
            (Self::South, Self::Northwest) |
            (Self::Southwest, Self::North) |
            (Self::Southwest, Self::Northeast) |
            (Self::Southwest, Self::East) |
            (Self::Southwest, Self::Southeast) |
            (Self::Southwest, Self::Northwest) |
            (Self::West, Self::Northeast) |
            (Self::West, Self::East) |
            (Self::West, Self::Southeast) |
            (Self::Northwest, Self::Northeast) |
            (Self::Northwest, Self::East) |
            (Self::Northwest, Self::Southeast) |
            (Self::Northwest, Self::South) |
            (Self::Northwest, Self::Southwest) => Err(CommandError::InvalidTileEdge(edge,self.clone()))
        }

    }

    pub(crate) fn direction(&self) -> Deg<f64> {
        // needs to be clockwise, from the north, with a value from 0..360
        match self {
            Edge::North => Deg(0.0),
            Edge::Northeast => Deg(45.0),
            Edge::East => Deg(90.0),
            Edge::Southeast => Deg(135.0),
            Edge::South => Deg(180.0),
            Edge::Southwest => Deg(225.0),
            Edge::West => Deg(270.0),
            Edge::Northwest => Deg(315.0),
        }
    }

    pub(crate) fn contains(&self, p: &(f64, f64), extent: &Extent) -> bool {
        match self {
            Edge::North => p.1 == extent.north(),
            Edge::Northeast => p.1 == extent.north() || p.0 == extent.east(),
            Edge::East => p.0 == extent.east(),
            Edge::Southeast => p.1 == extent.south || p.0 == extent.east(),
            Edge::South => p.1 == extent.south,
            Edge::Southwest => p.1 == extent.south || p.0 == extent.west,
            Edge::West => p.0 == extent.west,
            Edge::Northwest => p.1 == extent.north() || p.0 == extent.west,
        }
    }
}

impl_simple_serde_tagged_enum!{
    Edge {
        North,
        Northeast,
        East,
        Southeast,
        South,
        Southwest,
        West,
        Northwest

    }
}
