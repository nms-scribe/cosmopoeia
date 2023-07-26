use std::error::Error;
use std::fmt::Display;

pub(crate) use gdal::errors::GdalError;
use gdal::raster::GdalDataType;
use ordered_float::FloatIsNan;

pub(crate) use clap::error::Error as ArgumentError;

#[derive(Debug)]
pub(crate) enum CommandError {
    GdalError(GdalError),
    VoronoiExpectsPolygons,
    VoronoiExpectsTriangles,
    FloatIsNan,
    MissingField(&'static str),
    MissingGeometry,
    #[allow(dead_code)] RasterDatasetRequired,
    #[allow(dead_code)] UnsupportedRasterSourceBand(GdalDataType)
}

impl Error for CommandError {

}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GdalError(a) => write!(f,"gdal: {}",a),
            Self::VoronoiExpectsPolygons => write!(f,"Voronoi input data should be polygons."),
            Self::VoronoiExpectsTriangles => write!(f,"Voronoi input polygons should be triangles."),
            Self::FloatIsNan => write!(f,"A float was not a number."),
            Self::MissingField(a) => write!(f,"While loading data, a record had no value in for '{}'",a),
            Self::MissingGeometry => write!(f,"While loading data, a record had no geometry"),
            Self::RasterDatasetRequired => write!(f,"a raster file is required"),
            Self::UnsupportedRasterSourceBand(a) => write!(f,"raster source band type ({}) is not supported",a),
        }
    }
}

impl From<GdalError> for CommandError {

    fn from(value: GdalError) -> Self {
        Self::GdalError(value)
    }
}

impl From<FloatIsNan> for CommandError {

    fn from(_: FloatIsNan) -> Self {
        Self::FloatIsNan
    }

}

#[derive(Debug)]
pub(crate) enum ProgramError {
    ArgumentError(ArgumentError),
    CommandError(CommandError)
}

impl Error for ProgramError {

}

impl Display for ProgramError {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentError(a) => write!(f,"{}",a),
            Self::CommandError(a) => write!(f,"{}",a),
        }
    }
}

impl From<ArgumentError> for ProgramError {

    fn from(value: ArgumentError) -> Self {
        Self::ArgumentError(value)
    }
}

impl From<CommandError> for ProgramError {

    fn from(value: CommandError) -> Self {
        Self::CommandError(value)
    }
}