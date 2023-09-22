use std::error::Error;
use core::fmt::Display;

pub(crate) use gdal::errors::GdalError;
use gdal::raster::GdalDataType;
use ordered_float::FloatIsNan;
use gdal::vector::geometry_type_to_name;

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
    InvalidValueForIdList(String),
    InvalidValueForNeighborDirections(String),
    InvalidValueForIdRef(String), 
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
    EmptyNamerInput(String),
    TilePreferenceMultiplyMissingData,
    TilePreferenceDivideMissingData,
    TilePreferenceAddMissingData,
    GdalUnionFailed,
    GdalIntersectionFailed,
    GdalDifferenceFailed,
    UnsupportedGdalGeometryType(u32),
    IncorrectGdalGeometryType{ expected: u32, found: u32},
    LakeDissolveMadeAMultiPolygon,
    CantConvertMultiPolygonToPolygon,
    EmptyLinearRing,
    UnclosedLinearRing,
    InvalidValueForColor(String),     
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GdalError(a) => write!(f,"gdal: {a}"),
            Self::VoronoiExpectsPolygons => write!(f,"Voronoi input data should be polygons."),
            Self::VoronoiExpectsTriangles => write!(f,"Voronoi input polygons should be triangles."),
            Self::FloatIsNan => write!(f,"A float was not a number."),
            Self::MissingField(a) => write!(f,"While loading data, a record had no value for '{a}'"),
            Self::MissingGeometry(a) => write!(f,"While loading data, a record had no geometry in '{a}'"),
            Self::MissingFeature(layer, id) => write!(f,"Layer '{layer}' has no feature id '{id}'"),
            Self::InvalidValueForIdList(a) => write!(f,"Invalid value ('{a}') found for id_list field."),
            Self::InvalidValueForNeighborDirections(a) => write!(f,"Invalid value ('{a}') found for neighbors field"),
            Self::InvalidValueForIdRef(a) => write!(f,"Invalid value ('{a}') an id reference field"),
            Self::InvalidValueForSegmentFrom(a) => write!(f,"Invalid value ('{a}') found for river from_type field."),
            Self::InvalidValueForSegmentTo(a) => write!(f,"Invalid value ('{a}') found for river to_type field."),
            Self::InvalidBiomeMatrixValue(a) => write!(f,"Invalid value ('{a}') for biome matrix field."),
            Self::InvalidValueForLakeType(a) => write!(f,"Invalid value ('{a}') for lake type field."),
            Self::InvalidValueForGroupingType(a) => write!(f,"Invalid value ('{a}') for grouping type field."),
            Self::InvalidValueForCultureType(a) => write!(f,"Invalid value ('{a}') for culture type field."),
            Self::InvalidValueForColor(a) => write!(f,"Invalid value ('{a}') for color field."),
            Self::MissingGlacierBiome => write!(f,"Glacier biome is not specified as criteria in biomes table."),
            Self::MissingWetlandBiome => write!(f,"Wetland biome is not specified as criteria in biomes table."),
            Self::MissingOceanBiome => write!(f,"Ocean biome is not specified as criteria in biomes table."),
            Self::MissingBiomeMatrixSlot(a, b) => write!(f,"Matrix criteria at ({a},{b}) not specified in biome table."),
            Self::DuplicateGlacierBiome => write!(f,"Glacier biome is specified twice in biomes table."),
            Self::DuplicateWetlandBiome => write!(f,"Wetland biome is specified twice in biomes table."),
            Self::DuplicateOceanBiome => write!(f,"Ocean biome is specified twice in biomes table."),
            Self::DuplicateBiomeMatrixSlot(a, b) => write!(f,"Matrix criteria at ({a},{b}) specified twice in biome table."),
            Self::UnknownLookup(a,b) => write!(f,"Layer '{a}' has no feature with the name '{b}'."),
            Self::UnknownNamer(a) => write!(f,"Namer '{a}' not found in supplied name generators."),
            Self::NamerSourceRead(a) => write!(f,"Error reading namer source: {a}"),
            Self::NamerSourceWrite(a) => write!(f,"Error writing namer source: {a}"),
            Self::CultureSourceRead(a) => write!(f,"Error reading culture source: {a}"),
            Self::CultureSourceWrite(a) => write!(f,"Error writing culture source: {a}"),
            Self::PointFinderOutOfBounds(a, b) => write!(f,"An out of bounds point ({a},{b}) was added to a point finder"),
            Self::CantFindMiddlePoint(a, b, len) => match len {
                0 => write!(f,"Can't find middle point between tiles {a} and {b}. No matching points found."),
                1 => write!(f,"Can't find middle point between tiles {a} and {b}. One matching point found."),
                len => write!(f,"Can't find middle point between tiles {a} and {b}. {len} matching points found, need 2."),
            },
            Self::RasterDatasetRequired => write!(f,"a raster file is required"),
            Self::UnsupportedRasterSourceBand(a) => write!(f,"raster source band type ({a}) is not supported"),
            Self::MaxElevationMustBePositive(a) => write!(f,"maximum elevation {a} must be positive"),
            Self::MinElevationMustBeLess(a, b) => write!(f,"minimum elevation {a} must be less than maximum {b}"),
            Self::RecipeFileRead(a) => write!(f,"Error reading recipe file: {a}"),
            Self::TerrainProcessWrite(a)  => write!(f,"Error serializing terrain process: {a}"),
            Self::InvalidPropertyValue(a,b) => write!(f,"Invalid value for property {a} ('{b}')"),
            Self::PropertyNotSet(a) => write!(f,"Property {a} has not been set."),
            Self::InvalidRangeArgument(a) => write!(f,"Invalid range expression '{a}' in terrain processing parameters."),
            Self::CantFindTileNearPoint => write!(f,"No tile was found close to a supplied point, even at max expansion."),
            Self::EmptyNamerInput(a) => write!(f,"Namer '{a}' data did not contain any words."),
            Self::TilePreferenceMultiplyMissingData => write!(f,"Tile preference multiplication in culture set needs at least one term"),
            Self::TilePreferenceDivideMissingData => write!(f,"Tile preference division in culture set needs at least one term"),
            Self::TilePreferenceAddMissingData => write!(f,"Tile preference addition in culture set needs at least one term"),
            Self::GdalUnionFailed => write!(f,"Gdal union operation returned null"),
            Self::GdalDifferenceFailed => write!(f,"Gdal difference operation returned null"),
            Self::GdalIntersectionFailed => write!(f,"Gdal intersection operation returned null"),
            Self::UnsupportedGdalGeometryType(a) => write!(f,"Unsupported gdal geometry type '{}'",geometry_type_to_name(*a)),
            Self::IncorrectGdalGeometryType { expected, found } => write!(f,"Expected geometry type '{}', found '{}'.",geometry_type_to_name(*expected),geometry_type_to_name(*found)),
            Self::LakeDissolveMadeAMultiPolygon => write!(f,"While attempting to dissolve lake tiles, a multipolygon was created instead of a polygon."),
            Self::CantConvertMultiPolygonToPolygon => write!(f,"Attempted to convert a multi-polygon with more than one polygon into a simple polygon."),
            Self::EmptyLinearRing => write!(f,"Attempted to create an empty ring for a polygon."),
            Self::UnclosedLinearRing => write!(f,"Attempted to create an unclosed polygon ring.")
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

    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ArgumentError(a) => write!(f,"{a}"),
            Self::CommandError(a) => write!(f,"{a}"),
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