use angular_units::Deg;
use gdal::vector::Feature;
use gdal::vector::field_type_to_name;
use gdal::vector::FieldValue;
use gdal::vector::OGRFieldType;
use prisma::Rgb;

use crate::errors::CommandError;
use crate::impl_documentation_for_tagged_enum;
use crate::impl_simple_serde_tagged_enum;
use crate::utils::edge::Edge;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Deserializer;
use crate::utils::simple_serde::Serialize;
use crate::utils::simple_serde::Serializer;
use crate::utils::simple_serde::Token;
use crate::typed_map::fields::DocumentedFieldType;
use crate::typed_map::fields::FieldTypeDocumentation;
use crate::typed_map::fields::TypedField;
use crate::typed_map::fields::IdRef;


impl TypedField for Edge {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl TypedField for Option<Edge> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        feature.field_as_string_by_name(field_name)?.map(|a| Deserialize::read_from_str(&a)).transpose()
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(feature.set_field_string(field_name, &value.write_to_string())?)
        } else {
            Ok(feature.set_field_null(field_name)?)
        }
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        if let Some(value) = self {
            value.to_field_value()
        } else {
            Ok(None)
        }
    }
}

impl_documentation_for_tagged_enum!{
    /// The name of a side or corner of the map.
    Edge {
        /// The north edge of the map
        North,
        /// The northeast corner of the map
        Northeast,
        /// The east edge of the map
        East,
        /// The southeast corner of the map
        Southeast,
        /// The south edge of the map
        South,
        /// The southwest corner of the map
        Southwest,
        /// The west edge of the map
        West,
        /// The northwest corner of the map
        Northwest
    }
}

#[allow(variant_size_differences)] // Not sure how else to build this enum
#[derive(Clone,PartialEq,Eq,Hash,PartialOrd,Ord,Debug)]
pub(crate) enum Neighbor {
    OffMap(Edge),
    Tile(IdRef),
    CrossMap(IdRef, Edge),
}

impl TypedField for Neighbor {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }


}

impl TypedField for Option<Neighbor> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        feature.field_as_string_by_name(field_name)?.map(|a| Deserialize::read_from_str(&a)).transpose()
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(value.set_field(feature,field_name)?)
        } else {
            Ok(feature.set_field_null(field_name)?)
        }
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        if let Some(value) = self {
            value.to_field_value()
        } else {
            Ok(None)
        }
    }


}

impl TypedField for Vec<Neighbor> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl DocumentedFieldType for Neighbor {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation::new(
            "Neighbor".to_owned(),
            "Specifies a type of neighbor for a tile. There are three possibilities. They are described by their contents, not their name, in order to simplify the NeighborDirection fields.\n* Tile: a regular contiguous tile, which is specified by it's id.\n* CrossMap: a tile that sits on the opposite side of the map, specified by it's id and direction as an 'Edge'.\n* OffMap: unknown content that is off the edges of the map, specified merely by a direction as an 'Edge'".to_owned(),
            field_type_to_name(Self::STORAGE_TYPE),
            "<integer> | (<integer>,<Edge>) | <Edge>".to_owned(),
            vec![Edge::get_field_type_documentation()]
        )
    }
}

impl Serialize for Neighbor {
    // implemented so there are no tags.

    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        match self {
            Self::OffMap(edge) => edge.write_value(serializer),
            Self::Tile(id) => id.write_value(serializer),
            Self::CrossMap(id, edge) => (id,edge).write_value(serializer),
        }
    }
}

impl Deserialize for Neighbor {
    // implemented so there are no tags
    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        if deserializer.matches(&Token::OpenParenthesis)? {
            let id = Deserialize::read_value(deserializer)?;
            deserializer.expect(&Token::Comma)?;
            let edge = Deserialize::read_value(deserializer)?;
            deserializer.expect(&Token::CloseParenthesis)?;
            Ok(Self::CrossMap(id, edge))
        } else if let Some(id) = deserializer.matches_integer()? {
            Ok(Self::Tile(IdRef::new(id)))
        } else {
            let edge = Deserialize::read_value(deserializer)?;
            Ok(Self::OffMap(edge))
        }
    }
}

#[derive(Clone,PartialEq,Debug)]
pub(crate) struct NeighborAndDirection(pub Neighbor,pub Deg<f64>);

impl TypedField for Vec<NeighborAndDirection> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl DocumentedFieldType for Vec<NeighborAndDirection> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation::new(
            "NeighborAndDirection".to_owned(),
            "A pair of Neighbor and angular direction (in degrees, clockwise from north) surrounded by parentheses.".to_owned(),
            field_type_to_name(Self::STORAGE_TYPE),
            "(<Neighbor>,<real>)".to_owned(),
            vec![Neighbor::get_field_type_documentation()]
        )
    }
}

impl Serialize for NeighborAndDirection {

    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        // serialize it as a neighbor and the float inside the angle
        (&self.0,self.1.0).write_value(serializer)
    }
}

impl Deserialize for NeighborAndDirection {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        let (neighbor,float) = Deserialize::read_value(deserializer)?;
        Ok(Self(neighbor,Deg(float)))

    }
}



pub(crate) trait ColorConversion {

    fn try_from_hex_str(value: &str) -> Result<Rgb<u8>,CommandError>;

    fn into_hex_string(self) -> String;

}

impl ColorConversion for Rgb<u8> {

    fn try_from_hex_str(value: &str) -> Result<Self,CommandError> {
        let mut colors = (1..=5).step_by(2).flat_map(|n| {
            let str = &value.get(n..(n+2));
            str.and_then(|astr| u8::from_str_radix(astr, 16).ok()) // I'm going to drop the error anyway.
        });
        let red = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing red.".to_owned()))?;
        let green = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing green.".to_owned()))?;
        let blue = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing blue.".to_owned()))?;
        Ok(Self::new(red,green,blue))
    }

    fn into_hex_string(self) -> String {
        let (red,green,blue) = (self.red(),self.green(),self.blue());
        format!("#{red:02X?}{green:02X?}{blue:02X?}")
    }
}


impl TypedField for Deg<f64> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTReal;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Ok(Self(f64::get_field(feature,field_name,field_id)?))
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        self.0.set_field(feature, field_name)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        self.0.to_field_value()
    }

}


impl DocumentedFieldType for Deg<f64> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation::new(
            "Angle".to_owned(),
            "A real number from 0 to 360.".to_owned(),
            field_type_to_name(Self::STORAGE_TYPE),
            "<real>".to_owned(),
            Vec::new()
        )
    }
}



impl TypedField for Rgb<u8> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::try_from_hex_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    
    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.into_hex_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.into_hex_string())))
    }

}


impl DocumentedFieldType for Rgb<u8> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation::new(
            "Color".to_owned(),
            "A color in #RRGGBB syntax.".to_owned(),
            field_type_to_name(Self::STORAGE_TYPE),
            "<color>".to_owned(),
            Vec::new(),
        )
    }
}


#[derive(Clone,PartialEq,Debug)]
pub(crate) enum Grouping {
    LakeIsland,
    Islet,
    Island,
    Continent,
    Lake,
    Ocean
}

impl Grouping {

    pub(crate) const fn is_ocean(&self) -> bool {
        matches!(self,Self::Ocean)
    }

    pub(crate) const fn is_water(&self) -> bool {
        matches!(self,Self::Ocean | Self::Lake)
    }


}

impl TypedField for Grouping {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }


    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}


impl_simple_serde_tagged_enum!{
    Grouping {
        Continent,
        Island,
        Islet,
        Lake,
        LakeIsland,
        Ocean
    }
}

impl_documentation_for_tagged_enum!{
    /// A type of land or water feature.
    Grouping {
        /// A large land mass surrounded by ocean or the edge of the map if no ocean
        Continent,
        /// A small land mass surrounded by ocean
        Island,
        /// A smaller land mass surrounded by ocean
        Islet,
        /// A body of water created from rainfall, usually not at elevation 0.
        Lake,
        /// A land mass surrounded by a lake
        LakeIsland,
        /// A body of water created by flooding the terrain to elevation 0.
        Ocean
    }

}


impl From<&Grouping> for String {
    fn from(value: &Grouping) -> Self {
        value.write_to_string()
    }
}

impl TryFrom<String> for Grouping {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidValueForGroupingType(value,format!("{e}")))
    }
}



#[derive(Clone,PartialEq,Debug)]
pub(crate) enum RiverSegmentFrom {
    Source,
    Lake,
    Branch,
    Continuing,
    BranchingLake,
    BranchingConfluence,
    Confluence,
}

impl TypedField for RiverSegmentFrom {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}


impl_documentation_for_tagged_enum!{
    /// A name for how the river segment begins
    RiverSegmentFrom {
        /// The segment begins with the splitting of another river
        Branch,
        /// The segment begins with a split that coincides with a confluence in the same tile
        BranchingConfluence,
        /// The segment begins with a split coming out of a lake
        BranchingLake,
        /// The segment begins with the joining of two rivers
        Confluence,
        /// The segment begins at the end of a single other segment
        Continuing,
        /// The segment begins at the outflow of a lake
        Lake,
        /// The segment begins where no other segment ends, with enough waterflow to make a river
        Source,
    }
}

impl_simple_serde_tagged_enum!{
    RiverSegmentFrom {
        Branch,
        BranchingConfluence,
        BranchingLake,
        Confluence,
        Continuing,
        Lake,
        Source,
    }
}

impl TryFrom<String> for RiverSegmentFrom {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidValueForSegmentFrom(value,format!("{e}")))
    }
}

impl From<&RiverSegmentFrom> for String {

    fn from(value: &RiverSegmentFrom) -> Self {
        value.write_to_string()
    }
}

#[derive(Clone,PartialEq,Debug)]
pub(crate) enum RiverSegmentTo {
    Mouth,
    Confluence,
    Continuing,
    Branch,
    BranchingConfluence,
}

impl TypedField for RiverSegmentTo {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}


impl_simple_serde_tagged_enum!{
    RiverSegmentTo {
        Branch,
        BranchingConfluence,
        Confluence,
        Continuing,
        Mouth,
    }
}

impl_documentation_for_tagged_enum!{
    /// A name for how a river segment ends
    RiverSegmentTo {
        /// The segment ends by branching into multiple segments
        Branch,
        /// The segment ends by branching where other segments also join
        BranchingConfluence,
        /// The segment ends by joining with another segment
        Confluence,
        /// The segment ends with the beginning of a single other segment
        Continuing,
        /// The segment ends by emptying into an ocean or lake
        Mouth,
    }
}

impl TryFrom<String> for RiverSegmentTo {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidValueForSegmentTo(value,format!("{e}")))
    }
}

impl From<&RiverSegmentTo> for String {

    fn from(value: &RiverSegmentTo) -> Self {
        value.write_to_string()
    }
}

#[derive(Clone,PartialEq,Debug)]
pub(crate) enum LakeType {
    Fresh,
    Salt,
    Frozen,
    Pluvial, // lake forms intermittently, it's also salty
    Dry,
    Marsh,
}



impl_documentation_for_tagged_enum!{
    /// A name for a type of lake.
    LakeType {
        /// Lake is freshwater
        Fresh,
        /// Lake is saltwater
        Salt,
        /// Lake is frozen
        Frozen,
        /// Lake is intermittent
        Pluvial,
        /// Lakebed is dry
        Dry,
        /// Lake is shallow
        Marsh
    }
}

impl TypedField for LakeType {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}


impl_simple_serde_tagged_enum!{
    LakeType {
        Fresh,
        Salt,
        Frozen,
        Pluvial,
        Dry,
        Marsh
    }
}


impl From<&LakeType> for String {

    fn from(value: &LakeType) -> Self {
        value.write_to_string()
    }
}

impl TryFrom<String> for LakeType {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidValueForLakeType(value,format!("{e}")))
    }
}


#[derive(Clone,PartialEq,Debug)]
pub(crate) enum BiomeCriteria {
    Matrix(Vec<(usize,usize)>), // moisture band, temperature band
    Wetland(f64), // waterflow above which climate is considered wetland
    Glacier(f64), // temperature below which climate is considered glacier
    Ocean
}

impl DocumentedFieldType for (usize,usize) {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation::new(
            "Unsigned Integer Pair".to_owned(), 
            "A pair of unsigned integers in parentheses.".to_owned(), 
            field_type_to_name(OGRFieldType::OFTString), 
            "(<integer>,<integer>)".to_owned(),
            Vec::new()
        )
    }
}

impl_documentation_for_tagged_enum!{
    /// Criteria for how the biome is to be mapped to the world based on generated climate data.
    BiomeCriteria {
        /// This biome should be used for glacier -- only one is allowed
        Glacier(temp: f64),
        /// The biome should be placed in the following locations in the moisture and temperature matrix -- coordinates must not be used for another biome
        Matrix(list: Vec<(usize,usize)>),
        /// The biome should be used for ocean -- only one is allowed
        Ocean,
        /// The biome should be used for wetland -- only one is allowed
        Wetland(water_flow: f64),
    }
}

impl TypedField for BiomeCriteria {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}



impl_simple_serde_tagged_enum!{
    BiomeCriteria {
        Glacier(temp: f64),
        Matrix(list: Vec<(usize,usize)>),
        Ocean,
        Wetland(water_flow: f64),
    }
}

impl TryFrom<String> for BiomeCriteria {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidBiomeMatrixValue(value,format!("{e}")))
    }
}

impl From<&BiomeCriteria> for String {

    fn from(value: &BiomeCriteria) -> Self {
        value.write_to_string()
    }
}


#[derive(Clone,Hash,Eq,PartialEq,Debug)]
pub(crate) enum CultureType {
    Generic,
    Lake,
    Naval,
    River,
    Nomadic,
    Hunting,
    Highland
}

impl TypedField for CultureType {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl_documentation_for_tagged_enum!{
    /// The name for the type of culture, which specifies how the culture behaves during generation
    CultureType {
        /// A culture with no landscape preferences, created when no other culture type is suggested
        Generic,
        /// A culture that prefers higher elevations
        Highland,
        /// A culture that prefers forested landscapes
        Hunting,
        /// A culture that prefers to live on the shore of lakes
        Lake,
        /// A culture that prefers ocean shores
        Naval,
        /// A culture that prevers drier elevations
        Nomadic,
        /// A culture that prefers to live along rivers
        River,
    }
}



impl_simple_serde_tagged_enum!{
    CultureType {
        Generic,
        Highland,
        Hunting,
        Lake,
        Naval,
        Nomadic,
        River,
    }
}



impl From<&CultureType> for String {

    fn from(value: &CultureType) -> Self {
        value.write_to_string()
    }
}


impl TryFrom<String> for CultureType {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidValueForCultureType(value,format!("{e}")))
    }
}
