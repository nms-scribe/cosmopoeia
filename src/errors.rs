use core::error::Error;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use std::io::Error as IOError;

pub(crate) use gdal::errors::GdalError;
use ordered_float::FloatIsNan;
use gdal::vector::geometry_type_to_name;
pub(crate) use clap::error::Error as ArgumentError;

use crate::utils::edge::Edge;
use crate::utils::simple_serde::Token;
use crate::typed_map::fields::IdRef;

#[derive(Debug,Clone)]
pub enum CommandError {
    GdalError(GdalError),
    // (never constructed) VoronoiExpectsPolygons,
    VoronoiExpectsTriangles(String),
    FloatIsNan,
    MissingField(&'static str),
    MissingGeometry(&'static str),
    MissingFeature(&'static str,IdRef),
    // (never constructed) InvalidValueForIdList(String,String),
    // (never constructed) InvalidValueForNeighborDirections(String,String),
    // (never constructed) InvalidValueForIdRef(String,String),
    InvalidValueForSegmentFrom(String,String),
    InvalidValueForSegmentTo(String,String),
    InvalidBiomeMatrixValue(String,String),
    InvalidValueForLakeType(String,String),
    InvalidValueForGroupingType(String,String),
    InvalidValueForCultureType(String,String),
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
    CantFindMiddlePoint(IdRef,IdRef,usize),
    RasterDatasetRequired,
    // (never constructed) UnsupportedRasterSourceBand(GdalDataType),
    MaxElevationMustBePositive(f64),
    MinElevationMustBeLess(f64, f64),
    RecipeFileRead(String),
    TerrainProcessWrite(String),
    InvalidPropertyValue(String,String,String),
    PropertyNotSet(String),
    InvalidRangeArgument(String,String),
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
    // (never constructed) LakeDissolveMadeAMultiPolygon,
    CantConvertMultiPolygonToPolygon,
    EmptyLinearRing,
    UnclosedLinearRing,
    InvalidValueForColor(String,String),
    InvalidTileEdge(Edge,Edge),
    // (never constructed) InvalidValueForNeighbor(String,String),
    // (never constructed) InvalidValueForNeighborList(String,String),
    // (never constructed) InvalidValueForEdge(String,String),
    CantFindMiddlePointOnEdge(IdRef, Edge, usize),
    InvalidNumberInSerializedValue(String,String),
    InvalidStringInSerializedValue(String),
    InvalidCharacterInSerializedValue(char),
    ExpectedTokenInSerializedValue(Token, Option<Token>),
    ExpectedIdentifierInSerializedValue(Option<Token>),
    ExpectedFloatInSerializedValue(Option<Token>),
    ExpectedIntegerInSerializedValue(u32,bool,Option<Token>),
    InvalidEnumValueInInSerializedValue(String),
    NamerDistributionError(String,String),
    IOError(String),
    SerdeJSONError(String),
    /// This one is thrown by attempting to convert geo_types
    CantConvert { expected: &'static str, found: &'static str },
}

impl Error for CommandError {

}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::GdalError(a) => write!(f,"gdal: {a}"),
            // Self::VoronoiExpectsPolygons => write!(f,"Voronoi input data should be polygons."),
            Self::VoronoiExpectsTriangles(message) => write!(f,"Voronoi input polygons should be triangles. ('{message}')"),
            Self::FloatIsNan => write!(f,"A float was not a number."),
            Self::MissingField(a) => write!(f,"While loading data, a record had no value for '{a}'"),
            Self::MissingGeometry(a) => write!(f,"While loading data, a record had no geometry in '{a}'"),
            Self::MissingFeature(layer, id) => write!(f,"Layer '{layer}' has no feature id '{id}'"),
            // Self::InvalidValueForIdList(a,message) => write!(f,"Invalid value ('{a}') found for id_list field. ('{message}')"),
            // Self::InvalidValueForNeighborList(a,message) => write!(f,"Invalid value ('{a}') found for neighbor_list field. ('{message}')"),
            // Self::InvalidValueForNeighborDirections(a,message) => write!(f,"Invalid value ('{a}') found for tile neighbors field. ('{message}')"),
            // Self::InvalidValueForIdRef(a,message) => write!(f,"Invalid value ('{a}') an id reference field. ('{message}')"),
            // Self::InvalidValueForNeighbor(a,message) => write!(f,"Invalid value ('{a}') a tile neighbor field. ('{message}')"),
            Self::InvalidValueForSegmentFrom(a,message) => write!(f,"Invalid value ('{a}') found for river from_type field. ('{message}')"),
            Self::InvalidValueForSegmentTo(a,message) => write!(f,"Invalid value ('{a}') found for river to_type field. ('{message}')"),
            Self::InvalidBiomeMatrixValue(a,message) => write!(f,"Invalid value ('{a}') for biome matrix field. ('{message}')"),
            Self::InvalidValueForLakeType(a,message) => write!(f,"Invalid value ('{a}') for lake type field. ('{message}')"),
            Self::InvalidValueForGroupingType(a,message) => write!(f,"Invalid value ('{a}') for grouping type field. ('{message}')"),
            Self::InvalidValueForCultureType(a,message) => write!(f,"Invalid value ('{a}') for culture type field. ('{message}')"),
            Self::InvalidValueForColor(a,message) => write!(f,"Invalid value ('{a}') for color field. ('{message}')"),
            // Self::InvalidValueForEdge(a,message) => write!(f,"Invalid value ('{a}') for map edge field. ('{message}')"),
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
            Self::CantFindMiddlePointOnEdge(a, b, len) => match len {
                0 => write!(f,"Can't find middle point between tiles {a} on edge '{b:?}'. No matching points found."),
                1 => write!(f,"Can't find middle point between tiles {a} on edge '{b:?}'. One matching point found."),
                len => write!(f,"Can't find middle point between tiles {a} on edge '{b:?}'. {len} matching points found, need 2."),
            },
            Self::RasterDatasetRequired => write!(f,"a raster file is required"),
            // Self::UnsupportedRasterSourceBand(a) => write!(f,"raster source band type ({a}) is not supported"),
            Self::MaxElevationMustBePositive(a) => write!(f,"maximum elevation {a} must be positive"),
            Self::MinElevationMustBeLess(a, b) => write!(f,"minimum elevation {a} must be less than maximum {b}"),
            Self::RecipeFileRead(a) => write!(f,"Error reading recipe file: {a}"),
            Self::TerrainProcessWrite(a)  => write!(f,"Error serializing terrain process: {a}"),
            Self::InvalidPropertyValue(a,b,message) => write!(f,"Invalid value for property {a} :'{b}'. ('{message}')"),
            Self::PropertyNotSet(a) => write!(f,"Property {a} has not been set."),
            Self::InvalidRangeArgument(a,message) => write!(f,"Invalid range expression '{a}' in terrain processing parameters. ('{message}')"),
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
            // Self::LakeDissolveMadeAMultiPolygon => write!(f,"While attempting to dissolve lake tiles, a multipolygon was created instead of a polygon."),
            Self::CantConvertMultiPolygonToPolygon => write!(f,"Attempted to convert a multi-polygon with more than one polygon into a simple polygon."),
            Self::EmptyLinearRing => write!(f,"Attempted to create an empty ring for a polygon."),
            Self::UnclosedLinearRing => write!(f,"Attempted to create an unclosed polygon ring."),
            Self::InvalidTileEdge(a,b) => write!(f,"A tile was calculated to be on a conflicting edges ('{a:?}' and '{b:?}') of the map. Perhaps the tile count is too small."),
            Self::InvalidNumberInSerializedValue(a,message) => write!(f,"While parsing field value: found invalid number  '{a}'. ('{message}')"),
            Self::InvalidStringInSerializedValue(a) => write!(f,"While parsing field value: found unterminated string '{a}'."),
            Self::InvalidCharacterInSerializedValue(a) => write!(f,"While parsing field value: found unexpected character '{a}'."),
            Self::ExpectedTokenInSerializedValue(expected, found) => if let Some(found) = found {
                write!(f,"While parsing field value: expected '{expected:?}', found '{found:?}'.")
            } else {
                write!(f,"While parsing field value: expected '{expected:?}', found end of text.")

            },
            Self::ExpectedIdentifierInSerializedValue(found) => if let Some(found) = found {
                write!(f,"While parsing field value: expected identifier, found '{found:?}'.")
            } else {
                write!(f,"While parsing field value: expected identifier, found end of text.")

            },
            Self::ExpectedFloatInSerializedValue(found) => if let Some(found) = found {
                write!(f,"While parsing field value: expected float, found '{found:?}'.")
            } else {
                write!(f,"While parsing field value: expected float, found end of text.")

            },
            Self::ExpectedIntegerInSerializedValue(size,signed,found) => if let Some(found) = found {
                write!(f,"While parsing field value: expected {} integer({size}), found '{found:?}'.",if *signed { "signed" } else { "" })
            } else {
                write!(f,"While parsing field value: expected {} integer({size}), found end of text.",if *signed { "signed" } else { "" })

            },
            Self::InvalidEnumValueInInSerializedValue(value) => write!(f,"While parsing field value: found invalid enum value '{value}'."),
            Self::NamerDistributionError(namer,message) => write!(f,"While loading namer '{namer}', the length distribution could not be calculated. ('{message}')"),
            Self::IOError(message) => write!(f,"Error writing to file: ('{message}')."),
            Self::SerdeJSONError(message) => write!(f,"Error serializing data: ('{message}')."),
            Self::CantConvert { expected, found } => write!(f,"Error converting geo types. Expected {expected}, found {found}.")

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

impl From<IOError> for CommandError {
    fn from(value: IOError) -> Self {
        Self::IOError(format!("{value}"))
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJSONError(format!("{value}"))
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

    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
