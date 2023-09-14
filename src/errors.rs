use std::error::Error;
use std::fmt::Display;

pub(crate) use gdal::errors::GdalError;
use gdal::raster::GdalDataType;
use ordered_float::FloatIsNan;

pub(crate) use clap::error::Error as ArgumentError;

#[derive(Debug)]
pub enum CommandError {
    GdalError(GdalError),
    VoronoiExpectsPolygons,
    VoronoiExpectsTriangles,
    FloatIsNan,
    MissingField(&'static str),
    MissingGeometry(&'static str),
    MissingFeature(&'static str,u64),
    InvalidValueForIdRef(&'static str), 
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
    UnknownLookup(&'static str,String),
    UnknownNamer(String),
    DuplicateBiomeMatrixSlot(usize,usize),
    DuplicateGlacierBiome,
    DuplicateWetlandBiome,
    DuplicateOceanBiome,
    NamerSourceRead(String),
    NamerSourceWrite(String),
    CultureSourceRead(String),
    CultureSourceWrite(String),
    PointFinderOutOfBounds(f64,f64),
    CantFindMiddlePoint(u64,u64,usize),
    RasterDatasetRequired,
    UnsupportedRasterSourceBand(GdalDataType),
    MaxElevationMustBePositive(f64),
    MinElevationMustBeLess(f64, f64),
    RecipeFileRead(String),
    TerrainProcessWrite(String),
    InvalidPropertyValue(String,String),
    PropertyNotSet(String),
    InvalidRangeArgument(String),
    CantFindTileNearPoint,
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
            Self::MissingField(a) => write!(f,"While loading data, a record had no value for '{}'",a),
            Self::MissingGeometry(a) => write!(f,"While loading data, a record had no geometry in '{}'",a),
            Self::MissingFeature(layer, id) => write!(f,"Layer '{}' has no feature id '{}'",layer,id),
            Self::InvalidValueForIdRef(a) => write!(f,"While loading data, {} had an invalid value for an id reference",a),
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
            Self::UnknownLookup(a,b) => write!(f,"Layer '{}' has no feature with the name '{}'.",a,b),
            Self::UnknownNamer(a) => write!(f,"Namer '{}' not found in supplied name generators.",a),
            Self::NamerSourceRead(a) => write!(f,"Error reading namer source: {}",a),
            Self::NamerSourceWrite(a) => write!(f,"Error writing namer source: {}",a),
            Self::CultureSourceRead(a) => write!(f,"Error reading culture source: {}",a),
            Self::CultureSourceWrite(a) => write!(f,"Error writing culture source: {}",a),
            Self::PointFinderOutOfBounds(a, b) => write!(f,"An out of bounds point ({},{}) was added to a point finder",a,b),
            Self::CantFindMiddlePoint(a, b, len) => match len {
                0 => write!(f,"Can't find middle point between tiles {} and {}. No matching points found.",a,b),
                1 => write!(f,"Can't find middle point between tiles {} and {}. One matching point found.",a,b),
                len => write!(f,"Can't find middle point between tiles {} and {}. {len} matching points found, need 2.",a,b),
            },
            Self::RasterDatasetRequired => write!(f,"a raster file is required"),
            Self::UnsupportedRasterSourceBand(a) => write!(f,"raster source band type ({}) is not supported",a),
            Self::MaxElevationMustBePositive(a) => write!(f,"maximum elevation {} must be positive",a),
            Self::MinElevationMustBeLess(a, b) => write!(f,"minimum elevation {} must be less than maximum {}",a,b),
            Self::RecipeFileRead(a) => write!(f,"Error reading recipe file: {}",a),
            Self::TerrainProcessWrite(a)  => write!(f,"Error serializing terrain process: {}",a),
            Self::InvalidPropertyValue(a,b) => write!(f,"Invalid value for property {} ('{}')",a,b),
            Self::PropertyNotSet(a) => write!(f,"Property {} has not been set.",a),
            Self::InvalidRangeArgument(a) => write!(f,"Invalid range expression '{}' in terrain processing parameters.",a),
            Self::CantFindTileNearPoint => write!(f,"No tile was found close to a supplied point, even at max expansion."),
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