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
            (Self::North | Self::East, Self::Northeast) |
            (Self::Northeast | Self::East, Self::North) |
            (Self::Northeast | Self::North, Self::East) => Ok(Self::Northeast),
            (Self::North | Self::Northwest, Self::West) |
            (Self::West | Self::North, Self::Northwest) |
            (Self::Northwest | Self::West, Self::North) => Ok(Self::Northwest),
            (Self::East | Self::Southeast, Self::South) |
            (Self::Southeast | Self::South, Self::East) |
            (Self::South | Self::East, Self::Southeast) => Ok(Self::Southeast),
            (Self::South | Self::Southwest, Self::West) |
            (Self::Southwest | Self::West, Self::South) |
            (Self::West | Self::South, Self::Southwest) => Ok(Self::Southwest),
            (Self::North, Self::North) |
            (Self::Northeast, Self::Northeast) |
            (Self::East, Self::East) |
            (Self::Southeast, Self::Southeast) |
            (Self::South, Self::South) |
            (Self::Southwest, Self::Southwest) |
            (Self::West, Self::West) |
            (Self::Northwest, Self::Northwest) => Ok(edge),
            (Self::North | Self::Northeast | Self::Southwest | Self::West | Self::Northwest, Self::Southeast) |
            (Self::North | Self::Northeast | Self::Northwest, Self::South) |
            (Self::North | Self::Northeast | Self::East | Self::Southeast |
            Self::Northwest, Self::Southwest) |
            (Self::Northeast | Self::East | Self::Southeast, Self::West) |
            (Self::Northeast | Self::East | Self::Southeast | Self::South |
            Self::Southwest, Self::Northwest) |
            (Self::Southeast | Self::South | Self::Southwest, Self::North) |
            (Self::Southeast | Self::South | Self::Southwest | Self::West |
            Self::Northwest, Self::Northeast) |
            (Self::Southwest | Self::West | Self::Northwest, Self::East) => Err(CommandError::InvalidTileEdge(edge,self.clone()))
        }

    }

    pub(crate) const fn direction(&self) -> Deg<f64> {
        // needs to be clockwise, from the north, with a value from 0..360
        match self {
            Self::North => Deg(0.0),
            Self::Northeast => Deg(45.0),
            Self::East => Deg(90.0),
            Self::Southeast => Deg(135.0),
            Self::South => Deg(180.0),
            Self::Southwest => Deg(225.0),
            Self::West => Deg(270.0),
            Self::Northwest => Deg(315.0),
        }
    }

    pub(crate) fn contains(&self, p: &(f64, f64), extent: &Extent) -> bool {
        match self {
            // (p.1 - extent.north()).abs() < 
            Self::North => (p.1 - extent.north()).abs() < f64::EPSILON,
            Self::Northeast => (p.1 - extent.north()).abs() < f64::EPSILON || (p.0 - extent.east()).abs() < f64::EPSILON,
            Self::East => (p.0 - extent.east()).abs() < f64::EPSILON,
            Self::Southeast => (p.1 - extent.south).abs() < f64::EPSILON || (p.0 - extent.east()).abs() < f64::EPSILON,
            Self::South => (p.1 - extent.south).abs() < f64::EPSILON,
            Self::Southwest => (p.1 - extent.south).abs() < f64::EPSILON || (p.0 - extent.west).abs() < f64::EPSILON,
            Self::West => (p.0 - extent.west).abs() < f64::EPSILON,
            Self::Northwest => (p.1 - extent.north()).abs() < f64::EPSILON || (p.0 - extent.west).abs() < f64::EPSILON,
        }
    }

    pub(crate) const fn opposite(&self) -> Self {
        match self {
            Self::North => Self::South,
            Self::Northeast => Self::Southwest,
            Self::East => Self::West,
            Self::Southeast => Self::Northwest,
            Self::South => Self::North,
            Self::Southwest => Self::Northeast,
            Self::West => Self::East,
            Self::Northwest => Self::Southeast,
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
