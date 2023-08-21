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
    MissingFeature(&'static str,u64),
    InvalidValueForSegmentFrom(String),
    InvalidValueForSegmentTo(String),
    InvalidBiomeMatrixValue(String),
    InvalidValueForLakeType(String),
    InvalidValueForGroupingType(String),
    InvalidValueForCultureType(String),
    MissingGlacierBiome,
    MissingWetlandBiome,
    MissingOceanBiome,
    MissingBiomeMatrixSlot(usize,usize),
    UnknownBiome(String),
    UnknownNamer(String),
    DuplicateBiomeMatrixSlot(usize,usize),
    DuplicateGlacierBiome,
    DuplicateWetlandBiome,
    DuplicateOceanBiome,
    NamerSourceRead(String),
    NamerSourceWrite(String),
    CultureSourceRead(String),
    CultureSourceWrite(String),
    #[allow(dead_code)] RasterDatasetRequired,
    #[allow(dead_code)] UnsupportedRasterSourceBand(GdalDataType),
}

impl Error for CommandError {

}

pub(crate) trait MissingErrorToOption<ValueType> {

    fn missing_to_option(self) -> Result<Option<ValueType>,CommandError>;
    
}

impl<OkType> MissingErrorToOption<OkType> for Result<OkType,CommandError> {

    fn missing_to_option(self) -> Result<Option<OkType>,CommandError> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(CommandError::MissingField(_)) => Ok(None),
            Err(err) => Err(err)
        }
    }
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
            Self::MissingFeature(layer, id) => write!(f,"While loading data, layer '{}' had no feature id '{}'",layer,id),
            Self::InvalidValueForSegmentFrom(a) => write!(f,"Invalid value ('{}') found for from_type in river segments layer.",a),
            Self::InvalidValueForSegmentTo(a) => write!(f,"Invalid value ('{}') found for to_type in river segments layer.",a),
            Self::InvalidBiomeMatrixValue(a) => write!(f,"Invalid value ('{}') for biome matrix field.",a),
            Self::InvalidValueForLakeType(a) => write!(f,"Invalid value ('{}') for lake type field.",a),
            Self::InvalidValueForGroupingType(a) => write!(f,"Invalid value ('{}') for grouping type field.",a),
            Self::InvalidValueForCultureType(a) => write!(f,"Invalid value ('{}') for culture type field.",a),
            Self::MissingGlacierBiome => write!(f,"Glacier biome is not specified as criteria in biomes table."),
            Self::MissingWetlandBiome => write!(f,"Wetland biome is not specified as criteria in biomes table."),
            Self::MissingOceanBiome => write!(f,"Ocean biome is not specified as criteria in biomes table."),
            Self::MissingBiomeMatrixSlot(a, b) => write!(f,"Matrix criteria at ({},{}) not specified in biome table.",a,b),
            Self::DuplicateGlacierBiome => write!(f,"Glacier biome is specified twice in biomes table."),
            Self::DuplicateWetlandBiome => write!(f,"Wetland biome is specified twice in biomes table."),
            Self::DuplicateOceanBiome => write!(f,"Ocean biome is specified twice in biomes table."),
            Self::DuplicateBiomeMatrixSlot(a, b) => write!(f,"Matrix criteria at ({},{}) specified twice in biome table.",a,b),
            Self::UnknownBiome(a) => write!(f,"Biome '{}' not found in biomes table.",a),
            Self::UnknownNamer(a) => write!(f,"Namer '{}' not found in supplied name generators.",a),
            Self::NamerSourceRead(a) => write!(f,"Error reading namer source: {}",a),
            Self::NamerSourceWrite(a) => write!(f,"Error writing namer source: {}",a),
            Self::CultureSourceRead(a) => write!(f,"Error reading culture source: {}",a),
            Self::CultureSourceWrite(a) => write!(f,"Error writing culture source: {}",a),
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