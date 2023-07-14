use std::error::Error;
use std::fmt::Display;

pub use gdal::errors::GdalError;
use gdal::raster::GdalDataType;

pub use clap::error::Error as ArgumentError;

#[derive(Debug)]
pub enum CommandError {
    GdalError(GdalError),
    RasterDatasetRequired,
    UnsupportedRasterSourceBand(GdalDataType)
}

impl Error for CommandError {

}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GdalError(a) => write!(f,"gdal: {}",a),
            Self::RasterDatasetRequired => write!(f,"a raster file is required"),
            Self::UnsupportedRasterSourceBand(a) => write!(f,"raster source band type ({}) is not supported",a)
        }
    }
}

impl From<GdalError> for CommandError {

    fn from(value: GdalError) -> Self {
        Self::GdalError(value)
    }
}

#[derive(Debug)]
pub enum ProgramError {
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