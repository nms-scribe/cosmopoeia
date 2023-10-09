use core::hash::Hash;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::IntoIter;
use std::fmt::Display;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use gdal::LayerOptions;
use gdal::spatial_ref::SpatialRef;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Layer;
use gdal::vector::Feature;
use gdal::vector::FeatureIterator;
use gdal::Transaction;
use gdal::vector::field_type_to_name;
use ordered_float::OrderedFloat;
use ordered_float::NotNan;
use indexmap::IndexMap;
// rename these imports in case I want to use these from hash_map sometime.
use indexmap::map::Keys as IndexKeys;
use indexmap::map::Iter as IndexIter;
use indexmap::map::IterMut as IndexIterMut;
use indexmap::map::IntoIter as IndexIntoIter;
use paste::paste;
use prisma::Rgb;
use angular_units::Deg;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::utils::coordinates::Coordinates; // renamed so it doesn't conflict with geometry::Point, which is more important that it keep this name.
use crate::utils::extent::Extent;
use crate::utils::title_case::ToTitleCase;
use crate::gdal_fixes::FeatureFix;
use crate::algorithms::naming::Namer;
use crate::algorithms::naming::NamerSet;
use crate::commands::OverwriteTilesArg;
use crate::commands::OverwriteCoastlineArg;
use crate::commands::OverwriteOceanArg;
use crate::commands::OverwriteRiversArg;
use crate::commands::OverwriteLakesArg;
use crate::commands::OverwriteBiomesArg;
use crate::commands::OverwriteCulturesArg;
use crate::commands::OverwriteTownsArg;
use crate::commands::OverwriteSubnationsArg;
use crate::commands::OverwriteNationsArg;
use crate::algorithms::water_flow::WaterFlowResult;
use crate::geometry::GDALGeometryWrapper;
use crate::geometry::Point;
use crate::geometry::Polygon;
use crate::geometry::LineString;
use crate::geometry::MultiPolygon;
use crate::geometry::NoGeometry;
use crate::utils::edge::Edge;
use crate::geometry::MultiLineString;
use crate::utils::simple_serde::Deserializer as Deserializer;
use crate::utils::simple_serde::Serializer as Serializer;
use crate::utils::simple_serde::Serialize as Serialize;
use crate::utils::simple_serde::Deserialize as Deserialize;    
use crate::utils::simple_serde::Token;
use crate::impl_simple_serde_tagged_enum;


// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.

// FUTURE: Another problem with the gdal crate is the lifetimes. Feature, for example, only requires the lifetimes because it holds a reference to 
// a field definition pointer, which is never used except in the constructor. Once the feature is created, this reference could easily be forgotten. Layer is
// a little more complex, it holds a phantom value of the type of a reference to its dataset. On the one hand, it also doesn't do anything with it at all,
// on the other this reference might keep it from outliving it's dataset reference. Which, I guess, is the same with Feature, so maybe that's what they're 
// doing. I just wish there was another way, as it would make the TypedFeature stuff I'm trying to do below work better. However, if that were built into
// the gdal crate, maybe it would be better.


#[derive(Clone,PartialEq)]
pub(crate) struct FieldTypeDocumentation {
    pub(crate) name: String,
    pub(crate) description: String, // More detailed description for the format
    pub(crate) storage_type: String, // the concrete field type in the database
    pub(crate) syntax: String, // syntax for the format
    pub(crate) sub_types: Vec<FieldTypeDocumentation>

}

pub(crate) trait DocumentedFieldType {

    fn get_field_type_documentation() -> FieldTypeDocumentation;

}

pub(crate) trait TypedField: Sized + DocumentedFieldType {

    const STORAGE_TYPE: OGRFieldType::Type;

    fn get_required<FieldType>(value: Option<FieldType>, field_id: &'static str) -> Result<FieldType,CommandError> {
        value.ok_or_else(|| CommandError::MissingField(field_id))
    }
    
    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError>;

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError>;

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError>;
}




pub(crate) fn describe_variant_syntax(variants: &[(&str,&[&[FieldTypeDocumentation]])]) -> (String,Vec<FieldTypeDocumentation>) {
    let mut result = String::new();
    let mut sub_types = Vec::new();
    let mut first_variant = true;
    for (variant,has_tuple) in variants {
        if first_variant {
            first_variant = false;
        } else {
            result.push_str(" | ")
        }
        result.push('"');
        result.push_str(&variant);
        result.push('"');
        for tuple_items in *has_tuple {
            result.push_str(" (");
            let mut first_tuple_item = true;
            for item in *tuple_items {
                if first_tuple_item {
                    first_tuple_item = false
                } else {
                    result.push_str(", ");
                }
                result.push_str(&item.name);
                sub_types.push(item.clone());
            }
            result.push(')');
        }

    }
    (result,sub_types)


}

macro_rules! impl_documentation_for_tagged_enum {

    (#[doc=$description: literal] $enum: ty {$(#[doc=$variant_description: literal] $variant: ident $(($($tuple_name: ident: $tuple_type: ty),*$(,)?))?),*$(,)?}) => {
        impl DocumentedFieldType for $enum {

            fn get_field_type_documentation() -> FieldTypeDocumentation {

                let mut description = String::new();
                description.push_str($description.trim_start());
                description.push('\n');
                $(
                    description.push_str("* ");
                    description.push_str(stringify!($variant));
                    description.push(':');
                    description.push(' ');
                    description.push_str($variant_description.trim_start());
                    description.push('\n');
                )*

                let (syntax,sub_types) = describe_variant_syntax(&[$((stringify!($variant),&[$(&[$(<$tuple_type>::get_field_type_documentation()),*])?])),*]);

                FieldTypeDocumentation {
                    name: stringify!($enum).to_owned(),
                    description,
                    storage_type: field_type_to_name(<$enum>::STORAGE_TYPE), // TODO: Should take this from a TypedField trait instead
                    syntax,
                    sub_types
                }
            }
        
        }

        // This enforce that we have all of the same enum values
        impl $enum {

            fn _documented_field_type_identity(self) -> Self {
                match self {
                    $(
                        Self::$variant$(($($tuple_name),*))? => Self::$variant$(($($tuple_name),*))?,
                    )*
                }
            }

        }
    }
}

impl TypedField for Edge {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}



impl DocumentedFieldType for Neighbor {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "Neighbor".to_owned(),
            description: "Specifies a type of neighbor for a tile. There are three possibilities. They are described by their contents, not their name, in order to simplify the NeighborDirection fields.\n* Tile: a regular contiguous tile, which is specified by it's id.\n* CrossMap: a tile that sits on the opposite side of the map, specified by it's id and direction as an 'Edge'.\n* OffMap: unknown content that is off the edges of the map, specified merely by a direction as an 'Edge'".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "<integer> | (<integer>,<Edge>) | <Edge>".to_owned(),
            sub_types: vec![Edge::get_field_type_documentation()]
        }
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
    fn read_value<Source: Deserializer>(source: &mut Source) -> Result<Self,CommandError> {
        if source.matches(&Token::OpenParenthesis)? {
            let id = Deserialize::read_value(source)?;
            source.expect(&Token::Comma)?;
            let edge = Deserialize::read_value(source)?;
            source.expect(&Token::CloseParenthesis)?;
            Ok(Self::CrossMap(id, edge))
        } else if let Some(id) = source.matches_integer()? {
            Ok(Self::Tile(IdRef::new(id)))
        } else {
            let edge = Deserialize::read_value(source)?;
            Ok(Self::OffMap(edge))
        }
    }
}


#[derive(Clone,PartialEq,Debug)]
pub(crate) struct NeighborAndDirection(pub(crate) Neighbor,pub(crate) Deg<f64>);

impl TypedField for Vec<NeighborAndDirection> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}



impl DocumentedFieldType for Vec<NeighborAndDirection> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "NeighborAndDirection".to_owned(),
            description: "A pair of Neighbor and angular direction (in degrees, clockwise from north) surrounded by parentheses.".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "(<Neighbor>,<real>)".to_owned(),
            sub_types: vec![Neighbor::get_field_type_documentation()]
        }
    }
}

impl Serialize for NeighborAndDirection {

    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        // serialize it as a neighbor and the float inside the angle
        (&self.0,self.1.0).write_value(serializer)
    }
}

impl Deserialize for NeighborAndDirection {

    fn read_value<Source: Deserializer>(source: &mut Source) -> Result<Self,CommandError> {
        let (neighbor,float) = Deserialize::read_value(source)?;
        Ok(Self(neighbor,Deg(float)))

    }
}

#[derive(PartialEq,Eq,Hash,PartialOrd,Ord,Clone,Debug)]
pub struct IdRef(u64);

impl IdRef {

    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }

}


impl Display for IdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}

impl Deserialize for IdRef {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        let inner = Deserialize::read_value(deserializer)?;
        Ok(Self(inner))
    }

}

impl Serialize for IdRef {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        self.0.write_value(serializer)
    }
}

impl TypedField for IdRef {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    
    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl TypedField for Option<IdRef> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        feature.field_as_string_by_name(field_name)?.map(|a| Deserialize::read_from_str(&a)).transpose()
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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



impl DocumentedFieldType for IdRef {
    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "ID Reference".to_owned(),
            description: "A reference to the 'fid' field in another table. This is stored as a String field because an unsigned integer field is not available.".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE), 
            syntax: "<integer>".to_owned(),
            sub_types: Vec::new()
        }
    }
}

fn color_to_string(value: Rgb<u8>) -> String {
    let (red,green,blue) = (value.red(),value.green(),value.blue());
    format!("#{red:02X?}{green:02X?}{blue:02X?}")
}

fn string_to_color(value: &str) -> Result<Rgb<u8>,CommandError> {
    let mut colors = (1..=5).step_by(2).flat_map(|n| {
        let str = &value.get(n..(n+2));
        str.and_then(|astr| u8::from_str_radix(astr, 16).ok()) // I'm going to drop the error anyway.
    });
    let red = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing red.".to_owned()))?;
    let green = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing green.".to_owned()))?;
    let blue = colors.next().ok_or_else(|| CommandError::InvalidValueForColor(value.to_owned(),"Missing blue.".to_owned()))?;
    Ok(Rgb::new(red,green,blue))

}

pub(crate) trait Schema {

    type Geometry: GDALGeometryWrapper;

    const LAYER_NAME: &'static str;

    fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)];

}



pub(crate) trait TypedFeature<'data_life,SchemaType: Schema>: From<Feature<'data_life>>  {

    fn fid(&self) -> Result<IdRef,CommandError>;

    fn into_feature(self) -> Feature<'data_life>;

    fn geometry(&self) -> Result<SchemaType::Geometry,CommandError>;

    fn set_geometry(&mut self, value: SchemaType::Geometry) -> Result<(),CommandError>;


}

pub(crate) trait NamedFeature<'data_life,SchemaType: Schema>: TypedFeature<'data_life,SchemaType> {

    fn get_name(&self) -> Result<String,CommandError>;

}

macro_rules! feature_count_fields {
    () => {
        0
    };
    ($prop: ident) => {
        1
    };
    ($prop: ident, $($props: ident),+) => {
        $(feature_count_fields!($props)+)+ feature_count_fields!($prop)
    };
}

impl TypedField for Deg<f64> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTReal;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Ok(Deg(f64::get_field(feature,field_name,field_id)?))
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        self.0.set_field(feature, field_name)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        self.0.to_field_value()
    }

}


impl DocumentedFieldType for Deg<f64> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "Angle".to_owned(),
            description: "A real number from 0 to 360.".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "<real>".to_owned(),
            sub_types: Vec::new()
        }
    }
}

impl TypedField for String {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)
    }


    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self)?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.clone())))
    }

}



impl TypedField for Option<String> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;


    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        if let Some(value) = feature.field_as_string_by_name(field_name)? {
            if value == "" {
                // we're storing null strings as empty for now.
                Ok(None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }

    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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


impl DocumentedFieldType for String {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "String".to_owned(),
            description: "A string of text".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "<string>".to_owned(),
            sub_types: Vec::new(),
        }
    }
}

impl<Inner: DocumentedFieldType> DocumentedFieldType for Option<Inner> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        let inner = Inner::get_field_type_documentation();
        FieldTypeDocumentation { 
            name: format!("Optional {}",inner.name),
            description: inner.description.clone(), 
            storage_type: inner.storage_type.clone(), 
            syntax: format!("{}?",inner.syntax),
            sub_types: vec![inner]
        }
    }

}

impl<Inner: DocumentedFieldType> DocumentedFieldType for Vec<Inner> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        let inner = Inner::get_field_type_documentation();
        FieldTypeDocumentation { 
            name: format!("List of {}",inner.name),
            description: format!("A list of comma-separated {} values in brackets.",inner.name), 
            storage_type: field_type_to_name(OGRFieldType::OFTString), 
            syntax: format!("[<{}>, ..]",inner.name),
            sub_types: vec![inner]
        }
    }

}

impl TypedField for bool {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTInteger;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Ok(Self::get_required(feature.field_as_integer_by_name(field_name)?, field_id)? != 0)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_integer(field_name, (*self).into())?)

    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::IntegerValue((*self).into())))
    }

}

impl DocumentedFieldType for bool {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "Boolean".to_owned(),
            description: "An value of 1 or 0".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "<bool>".to_owned(),
            sub_types: Vec::new(),
        }
    }
}

impl TypedField for Rgb<u8> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        string_to_color(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    
    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &color_to_string(*self))?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(color_to_string(*self))))
    }

}


impl DocumentedFieldType for Rgb<u8> {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation {
            name: "Color".to_owned(),
            description: "A color in #RRGGBB syntax.".to_owned(),
            storage_type: field_type_to_name(Self::STORAGE_TYPE),
            syntax: "<color>".to_owned(),
            sub_types: Vec::new(),
        }
    }
}

impl TypedField for f64 {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTReal;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::get_required(feature.field_as_double_by_name(field_name)?, field_id)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        // The NotNan thing verifies that the value is not NaN, which would be treated as null.
        // This can help me catch math problems early...
        Ok(feature.set_field_double(field_name, NotNan::try_from(*self)?.into_inner())?)

    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::RealValue(NotNan::try_from(*self)?.into_inner())))
    }

}

impl DocumentedFieldType for f64 {
    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation { 
            name: "Real".to_owned(), 
            description: "A real number.".to_owned(), 
            storage_type: field_type_to_name(Self::STORAGE_TYPE), 
            syntax: "<real>".to_owned(),
            sub_types: Vec::new() 
        }
    }
}

impl TypedField for i32 {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTInteger;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::get_required(feature.field_as_integer_by_name(field_name)?, field_id)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_integer(field_name,*self)?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::IntegerValue(*self)))
    }
    

}

impl TypedField for Option<i32> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTInteger;


    
    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        Ok(feature.field_as_integer_by_name(field_name)?)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(feature.set_field_integer(field_name, *value)?)
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

impl DocumentedFieldType for i32 {
    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation { 
            name: "Signed Integer".to_owned(), 
            description: "A signed integer.".to_owned(), 
            storage_type: field_type_to_name(Self::STORAGE_TYPE), 
            syntax: "<integer>".to_owned(),
            sub_types: Vec::new()
        }
    }
}


pub(crate) struct FieldDocumentation {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) field_type: FieldTypeDocumentation
}

pub(crate) struct LayerDocumentation {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) geometry: String,
    pub(crate) fields: Vec<FieldDocumentation>
}


macro_rules! hide_item {
    ($anything: ident false, $content: item) => {
        $content
    };
    ($anything: ident $helper: literal, $content: item) => {
    };
    (, $content: item) => {
        $content
    };
}

macro_rules! layer {
    ($(#[doc = $layer_doc_attr: literal])? $name: ident [$layer_name: literal]: $geometry_type: ident {$(
        $(#[doc = $field_doc_attr: literal])? $(#[get($get_attr: meta)])* $(#[set($set_attr: meta)])* $prop: ident: $prop_type: ty
    ),*$(,)?} $(hide_add($hide_add: literal))? $(hide_doc($hide_doc: literal))?) => {

        paste!{
            pub(crate) struct [<$name Feature>]<'data_life> {

                feature: Feature<'data_life>
            }
    
        }
        
        paste!{
            impl<'impl_life> From<Feature<'impl_life>> for [<$name Feature>]<'impl_life> {
        
                fn from(feature: Feature<'impl_life>) -> Self {
                    Self {
                        feature
                    }
                }
            }
    
        }

        paste!{
            $(#[doc = $layer_doc_attr])?
            pub(crate) struct [<$name Schema>];
        }

        paste!{
            impl [<$name Schema>] {
                // constant field names
                paste!{
                    $(pub(crate) const [<FIELD_ $prop:snake:upper>]: &str = stringify!($prop);)*
                }

                // field definitions
                const FIELD_DEFS: [(&str,OGRFieldType::Type); feature_count_fields!($($prop),*)] = [
                    $((paste!{Self::[<FIELD_ $prop:snake:upper>]},<$prop_type>::STORAGE_TYPE)),*
                ];


            }
        }


        paste!{
            impl Schema for [<$name Schema>] {

                type Geometry = $geometry_type;

                const LAYER_NAME: &'static str = $layer_name;

                fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)] {
                    &Self::FIELD_DEFS
                }


            }
        }

        paste!{

            impl<'impl_life> TypedFeature<'impl_life,[<$name Schema>]> for [<$name Feature>]<'impl_life> {

                // fid field
                fn fid(&self) -> Result<IdRef,CommandError> {
                    Ok(IdRef::new(self.feature.fid().ok_or_else(|| CommandError::MissingField(concat!($layer_name,".","fid")))?))
                }

                fn into_feature(self) -> Feature<'impl_life> {
                    self.feature
                }

                fn geometry(&self) -> Result<$geometry_type,CommandError> {
                    self.feature.geometry().ok_or_else(|| CommandError::MissingGeometry($layer_name))?.clone().try_into()
                }

                fn set_geometry(&mut self, value: $geometry_type) -> Result<(),CommandError> {
                    Ok(self.feature.set_geometry(value.into())?)
                }

            }
        }
            
        paste!{
            
            impl [<$name Feature>]<'_> {

                // property functions
                $(
                    paste!{
                        $(#[doc = $field_doc_attr])?
                        $(#[$get_attr])* pub(crate) fn $prop(&self) -> Result<$prop_type,CommandError> {
                            <$prop_type>::get_field(&self.feature,[<$name Schema>]::[<FIELD_ $prop:snake:upper>],concat!($layer_name,".",stringify!($prop)))
                        }
                    }
            
                    paste!{
                        $(#[doc = $field_doc_attr])?
                        $(#[$set_attr])* pub(crate) fn [<set_ $prop>](&mut self, value: &$prop_type) -> Result<(),CommandError> {
                            value.set_field(&self.feature,[<$name Schema>]::[<FIELD_ $prop:snake:upper>])
                        }            
        
                    }
            
                )*

            }

        }

        paste!{

            hide_item!{$(hide_add $hide_add)?,
                pub(crate) struct [<New $name>] {
                    $(
                        pub(crate) $prop: $prop_type
                    ),*
                }
            }
        }

        paste!{
            pub(crate) type [<$name Layer>]<'layer,'feature> = MapLayer<'layer,'feature,[<$name Schema>],[<$name Feature>]<'feature>>;

            impl [<$name Layer>]<'_,'_> {

                hide_item!{$(hide_add $hide_add)?,
                    // I've marked entity as possibly not used because some calls have no fields and it won't be assigned.          
                    fn add_struct(&mut self, _entity: &[<New $name>], geometry: Option<<[<$name Schema>] as Schema>::Geometry>) -> Result<IdRef,CommandError> {
                        let field_names = [
                            $(paste!{
                                [<$name Schema>]::[<FIELD_ $prop:snake:upper>]
                            }),*
                        ];
                        let field_values = [
                            $(_entity.$prop.to_field_value()?),*
                        ];
                        if let Some(geometry) = geometry {
                            self.add_feature_with_geometry(geometry, &field_names, &field_values)
                        } else {
                            self.add_feature_without_geometry(&field_names, &field_values)
                        }

                    }
                }
            }

        }

        paste!{
            hide_item!{$(hide_doc $hide_doc)?,
                pub(crate) fn [<document_ $name:snake _layer>]() -> Result<LayerDocumentation,CommandError> {
                    Ok(LayerDocumentation {
                        name: $layer_name.to_owned(),
                        description: concat!("",$($layer_doc_attr: literal)?).trim_start().to_owned(),
                        geometry: stringify!($geometry_type).to_owned(),
                        fields: vec![
                            $(
                                FieldDocumentation {
                                    name: stringify!($prop).to_owned(),
                                    description: concat!("",$($field_doc_attr)?).trim_start().to_owned(),
                                    field_type: <$prop_type>::get_field_type_documentation()
                                }
                            ),*
                
                        ],
                    })            

                }
            }
        }

    };
}



pub(crate) struct TypedFeatureIterator<'data_life, SchemaType: Schema, Feature: TypedFeature<'data_life,SchemaType>> {
    features: FeatureIterator<'data_life>,
    _phantom_feature: core::marker::PhantomData<Feature>,
    _phantom_schema: core::marker::PhantomData<SchemaType>
}

impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>> Iterator for TypedFeatureIterator<'impl_life, SchemaType, Feature> {
    type Item = Feature;

    fn next(&mut self) -> Option<Self::Item> {
        self.features.next().map(Feature::from)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.features.size_hint()
    }
}

impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>> From<FeatureIterator<'impl_life>> for TypedFeatureIterator<'impl_life,SchemaType, Feature> {
    fn from(features: FeatureIterator<'impl_life>) -> Self {
        Self {
            features,
            _phantom_feature: core::marker::PhantomData,
            _phantom_schema: core::marker::PhantomData
        }
    }
}

impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>> TypedFeatureIterator<'impl_life, SchemaType, Feature> {

    pub(crate) fn into_entities_vec<Progress: ProgressObserver, Data: TryFrom<Feature,Error=CommandError>>(self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        let mut result = Vec::new();
        for entity in self.watch(progress,format!("Reading {}.",SchemaType::LAYER_NAME),format!("{} read.",SchemaType::LAYER_NAME.to_title_case())) {
            result.push(Data::try_from(entity)?);
        }
        Ok(result)
    }

    pub(crate) fn into_entities<Data: Entity<SchemaType>>(self) -> EntityIterator<'impl_life, SchemaType, Feature, Data> {
        self.into()
    }


    pub(crate) fn into_entities_index<Progress: ProgressObserver, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>>(self, progress: &mut Progress) -> Result<EntityIndex<SchemaType,Data>,CommandError> {

        self.into_entities_index_for_each(|_,_| Ok(()), progress)

    }

    pub(crate) fn into_entities_index_for_each<Progress: ProgressObserver, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>, Callback: FnMut(&IdRef,&Data) -> Result<(),CommandError>>(self, mut callback: Callback, progress: &mut Progress) -> Result<EntityIndex<SchemaType,Data>,CommandError> {

        let mut result = IndexMap::new();
        for feature in self.watch(progress,format!("Indexing {}.",SchemaType::LAYER_NAME),format!("{} indexed.",SchemaType::LAYER_NAME.to_title_case())) {
            let fid = feature.fid()?;
            let entity = Data::try_from(feature)?;

            callback(&fid,&entity)?;
    
            _ = result.insert(fid,entity);
        }

        Ok(EntityIndex::from(result))
    }

    pub(crate) fn into_named_entities_index<Progress: ProgressObserver, Data: NamedEntity<SchemaType> + TryFrom<Feature,Error=CommandError>>(self, progress: &mut Progress) -> Result<EntityLookup<SchemaType, Data>,CommandError> {
        let mut result = HashMap::new();

        for feature in self.watch(progress,format!("Indexing {}.",SchemaType::LAYER_NAME),format!("{} indexed.",SchemaType::LAYER_NAME.to_title_case())) {
            let entity = Data::try_from(feature)?;
            let name = entity.name().to_owned();
            _ = result.insert(name, entity);
        }

        Ok(EntityLookup::from(result))

    }


    

}


pub(crate) trait Entity<SchemaType: Schema> {

}

pub(crate) trait NamedEntity<SchemaType: Schema>: Entity<SchemaType> {
    fn name(&self) -> &str;
}


pub(crate) struct EntityIterator<'data_life, SchemaType: Schema, Feature: TypedFeature<'data_life,SchemaType>, Data: Entity<SchemaType>> {
    features: TypedFeatureIterator<'data_life,SchemaType,Feature>,
    data: core::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>> Iterator for EntityIterator<'impl_life,SchemaType,Feature,Data> {
    type Item = Result<(IdRef,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from(feature)) {
                (Ok(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (Err(e), Ok(_)) | (_, Err(e)) => Some(Err(e)),
            }
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.features.size_hint()
    }
}

impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life, SchemaType>, Data: Entity<SchemaType>> From<TypedFeatureIterator<'impl_life, SchemaType, Feature>> for EntityIterator<'impl_life, SchemaType, Feature, Data> {
    fn from(features: TypedFeatureIterator<'impl_life,SchemaType,Feature>) -> Self {
        Self {
            features,
            data: core::marker::PhantomData
        }
    }
}

pub(crate) struct EntityIndex<SchemaType: Schema, EntityType: Entity<SchemaType>> {
    // I use an IndexMap instead of HashMap as it ensures that the map maintains an order when iterating.
    // This helps me get reproducible results with the same random seed.
    inner: IndexMap<IdRef,EntityType>,
    _phantom: core::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> EntityIndex<SchemaType,EntityType> {

    // NOTE: There is no 'insert' or 'new' function because this should be created with to_entities_index.

    fn from(mut inner: IndexMap<IdRef,EntityType>) -> Self {
        // I want to ensure that the tiles are sorted in insertion order (by fid). So do this here.
        // if there were an easy way to insert_sorted from the beginning, then I wouldn't need to do this.
        inner.sort_keys();
        Self {
            inner,
            _phantom: core::marker::PhantomData
        }
    }
    
    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn try_get(&self, key: &IdRef) -> Result<&EntityType,CommandError> {
        self.inner.get(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, key.clone()))
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn try_get_mut(&mut self, key: &IdRef) -> Result<&mut EntityType,CommandError> {
        self.inner.get_mut(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, key.clone()))
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn try_remove(&mut self, key: &IdRef) -> Result<EntityType,CommandError> {
        self.inner.remove(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, key.clone()))
    }

    pub(crate) fn keys(&self) -> IndexKeys<'_, IdRef, EntityType> {
        self.inner.keys()
    }

    pub(crate) fn iter(&self) -> IndexIter<'_, IdRef, EntityType> {
        self.inner.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> IndexIterMut<'_, IdRef, EntityType> {
        self.inner.iter_mut()
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn maybe_get(&self, key: &IdRef) -> Option<&EntityType> {
        self.inner.get(key)
    }

    pub(crate) fn pop(&mut self) -> Option<(IdRef, EntityType)> {
        self.inner.pop()
    }

    pub(crate) fn watch_queue<StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &mut Progress, start: StartMessage, finish: FinishMessage) -> EntityIndexQueueWatcher<FinishMessage, Progress, SchemaType, EntityType> {
        progress.start(|| (start,Some(self.len())));
        EntityIndexQueueWatcher { 
            finish, 
            progress, 
            inner: self,
            popped: 0
        }

    }

}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> IntoIterator for EntityIndex<SchemaType,EntityType> {
    type Item = (IdRef,EntityType);

    type IntoIter = IndexIntoIter<IdRef,EntityType>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> FromIterator<(IdRef,EntityType)> for EntityIndex<SchemaType,EntityType> {

    fn from_iter<Iter: IntoIterator<Item = (IdRef,EntityType)>>(iter: Iter) -> Self {
        Self::from(IndexMap::from_iter(iter))
    }
}
        

pub(crate) struct EntityIndexQueueWatcher<'progress,Message: AsRef<str>, Progress: ProgressObserver, SchemaType: Schema, EntityType: Entity<SchemaType>> {
    finish: Message,
    progress: &'progress mut Progress,
    inner: EntityIndex<SchemaType,EntityType>,
    popped: usize,
}

impl<Message: AsRef<str>, Progress: ProgressObserver, SchemaType: Schema, EntityType: Entity<SchemaType>> EntityIndexQueueWatcher<'_,Message,Progress,SchemaType,EntityType> {

    pub(crate) fn pop(&mut self) -> Option<(IdRef,EntityType)> {
        let result = self.inner.pop();
        self.popped += 1;
        let len = self.inner.len();
        if len == 0 {
            self.progress.finish(|| &self.finish)
        } else {
            self.progress.update(|| self.popped);
        }
        result
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn maybe_get(&self, key: &IdRef) -> Option<&EntityType> {
        self.inner.maybe_get(key)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)] // except that the inner method only wants a ref as well.
    pub(crate) fn try_remove(&mut self, key: &IdRef) -> Result<EntityType,CommandError> {
        let result = self.inner.try_remove(key)?;
        self.popped += 1;
        let len = self.inner.len();
        if len == 0 {
            self.progress.finish(|| &self.finish)
        } else {
            self.progress.update(|| self.popped);
        }
        Ok(result)
    }

} 


pub(crate) struct EntityLookup<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> {
    inner: HashMap<String,EntityType>,
    _phantom: core::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> EntityLookup<SchemaType,EntityType> {

    const fn from(inner: HashMap<String,EntityType>) -> Self {
        Self {
            inner,
            _phantom: core::marker::PhantomData
        }
    }

    pub(crate) fn try_get(&self, key: &str) -> Result<&EntityType,CommandError> {
        self.inner.get(key).ok_or_else(|| CommandError::UnknownLookup(SchemaType::LAYER_NAME, key.to_owned()))
    }

}

impl<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> IntoIterator for EntityLookup<SchemaType,EntityType> {
    type Item = (String,EntityType);

    type IntoIter = IntoIter<String,EntityType>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
        

#[macro_export]
/// Used by `entity!` to generate expression for assigning to an entity field.
macro_rules! entity_field_assign {
    ($feature: ident geometry) => {
        $feature.geometry()?.clone()
    };
    ($feature: ident fid) => {
        $feature.fid()?
    };
    ($feature: ident $field: ident) => {
        $feature.$field()?
    };
    ($feature: ident $field: ident = $function: expr) => {
        $function(&$feature)?
    };
}

#[macro_export]
/// Used by `entity!` to generate an expression which tries to convert a feature into an entity.
macro_rules! entity_from_data {
    ($name: ident $feature: ident, $($field: ident: $type: ty $(= $function: expr)?),*) => {{
        #[allow(clippy::redundant_closure_call)] // I need to use a closure to call the expression from inside the macro, so it's not redundant.
        Ok($name {
            $(
                $field: $crate::entity_field_assign!($feature $field $(= $function)?)
            ),*
        })
    }};
}

#[macro_export]
/// Used by `entity!` to generate the type for an entity field
macro_rules! entity_field_def {
    ($type: ty [$function: expr]) => {
        $type
    };
    ($type: ty) => {
        $type
    };
}

#[macro_export]
/** 
Creates an entity struct that contains the specified fields. (See Entity trait)

* `$struct_attr` is an attribute that will be placed on the entity struct.
* `$name` is the name of the struct
* `$schema` is the identifier for the Schema type for the entity
* `$feature` is the identifier for the TypedFeature type for the entity

The body of the entity is a set of braces with a comma-separated list items describing the fields of the entity.

* `$field` is the identifier of the struct field
* `$type` is the type of the field generated
* `$function` is the assignment function, an optional closure used to initialize the value of the field. 

The assignment function closure takes an TypedFeature value and returns a result. The Ok result must be the type of the field being assigned to. The Err result must be a CommandError. If no assignment functions is specified, the macro will attempt to assign the ok result of a function on the feature with the same name as the field.

*/ 
macro_rules! entity {
    ($(#[$struct_attr: meta])* $name: ident: $layer: ident {$($field: ident: $type: ty $(= $function: expr)?),*$(,)?}) => {
        #[derive(Clone)]
        $(#[$struct_attr])* 
        pub(crate) struct $name {
            $(
                pub(crate) $field: $crate::entity_field_def!($type $([$function])?)
            ),*
        }

        paste::paste!{
            impl $crate::world_map::Entity<[<$layer Schema>]> for $name {

            }
    
    
        }

        paste::paste!{
            impl TryFrom<$crate::world_map::[<$layer Feature>]<'_>> for $name {

                type Error = CommandError;
    
                fn try_from(value: $crate::world_map::[<$layer Feature>]) -> Result<Self,Self::Error> {
                    $crate::entity_from_data!($name value, $($field: $type $(= $function)?),*)
                }
            }
    
        }

    };
}

pub(crate) struct MapLayer<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> {
    layer: Layer<'layer>,
    _phantom_feature: core::marker::PhantomData<&'feature Feature>,
    _phantom_schema: core::marker::PhantomData<SchemaType>
}

impl<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> MapLayer<'layer,'feature,SchemaType,Feature> {


    fn create_from_dataset(dataset: &'layer mut Dataset, overwrite: bool) -> Result<Self,CommandError> {

        // 4326 is WGS 84, although this is a fictional world and isn't necessarily shaped like Earth.
        // That coordinate system just seems "safe" as far as other tools are expecting an Earth-shape.
        let srs = SpatialRef::from_epsg(4326)?;
        let layer = dataset.create_layer(LayerOptions {
            name: SchemaType::LAYER_NAME,
            ty: SchemaType::Geometry::INTERNAL_TYPE,
            srs: if SchemaType::Geometry::INTERNAL_TYPE == OGRwkbGeometryType::wkbNone {
                // A few layers, such as properties, aren't actually supposed to hold any geography.
                // Okay, just properties so far...
                None
            } else {
                Some(&srs)
            },
            options: if overwrite { 
                Some(&["OVERWRITE=YES"])
            } else {
                None
            }
        })?;
        layer.create_defn_fields(SchemaType::get_field_defs())?;
        
        Ok(Self {
            layer,
            _phantom_feature: core::marker::PhantomData,
            _phantom_schema: core::marker::PhantomData
        })
    }

    fn open_from_dataset(dataset: &'layer Dataset) -> Result<Self,CommandError> {
        
        let layer = dataset.layer_by_name(SchemaType::LAYER_NAME)?;
        Ok(Self {
            layer,
            _phantom_feature: core::marker::PhantomData,
            _phantom_schema: core::marker::PhantomData
        })

    }

    pub(crate) fn try_feature_by_id(&'feature self, fid: &IdRef) -> Result<Feature,CommandError> {
        self.layer.feature(fid.0).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME,fid.clone())).map(Feature::from)
    }


    pub(crate) fn update_feature(&self, feature: Feature) -> Result<(),CommandError> {
        Ok(self.layer.set_feature(feature.into_feature())?)
    }

    pub(crate) fn feature_count(&self) -> usize {
        self.layer.feature_count() as usize
    }

    fn add_feature_with_geometry(&mut self, geometry: SchemaType::Geometry, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<IdRef,CommandError> {
        // I dug out the source to get this. I wanted to be able to return the feature being created.
        let mut feature = gdal::vector::Feature::new(self.layer.defn())?;
        feature.set_geometry(geometry.into())?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            if let Some(value) = value {
                feature.set_field(field, value)?;
            } else {
                feature.set_field_null(field)?;
            }
        }
        feature.create(&self.layer)?;
        Ok(IdRef::new(feature.fid().ok_or_else(|| CommandError::MissingField("fid"))?))
    }

    fn add_feature_without_geometry(&mut self, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<IdRef,CommandError> {
        // This function is used for lookup tables, like biomes.

        // I had to dig into the source to get this stuff...
        let feature = gdal::vector::Feature::new(self.layer.defn())?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            if let Some(value) = value {
                feature.set_field(field, value)?;
            } else {
                feature.set_field_null(field)?;
            }
        }
        feature.create(&self.layer)?;
        Ok(IdRef::new(feature.fid().ok_or_else(|| CommandError::MissingField("fid"))?))

    }

}

layer!(Point["points"]: Point {} hide_doc(true));


impl PointLayer<'_,'_> {

    pub(crate) fn add_point(&mut self, point: Point) -> Result<IdRef,CommandError> {

        self.add_struct(&NewPoint {  }, Some(point))
    
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<PointSchema,PointFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

}

layer!(Triangle["triangles"]: Polygon {} hide_doc(true));

impl TriangleLayer<'_,'_> {

    pub(crate) fn add_triangle(&mut self, geo: Polygon) -> Result<IdRef,CommandError> {

        self.add_struct(&NewTriangle {  }, Some(geo))
        
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TriangleSchema,TriangleFeature> {
        TypedFeatureIterator::from(self.layer.features())
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


    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

layer!(Tile["tiles"]: Polygon {
    /// longitude of the node point for the tile's voronoi
    #[set(allow(dead_code))] site_x: f64,
    /// latitude of the node point for the tile's voronoi
    #[set(allow(dead_code))] site_y: f64,
    /// elevation in meters of the node point for the tile's voronoi
    elevation: f64,
    // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
    // If I ever get rid of those algorithms, this field can go away.
    /// elevation scaled into a value from 0 to 100, where 20 is sea-level.
    elevation_scaled: i32,
    /// Indicates whether the tile is part of the ocean, an island, a continent, a lake, and maybe others.
    grouping: Grouping,
    /// A unique id for each grouping. These id's do not map to other tables, but will tell when tiles are in the same group. Use lake_id to link to the lake table.
    // NOTE: This isn't an IdRef, but let's store it that way anyway
    grouping_id: IdRef,
    /// average annual temperature of tile in imaginary units
    temperature: f64,
    /// roughly estimated average wind direction for tile
    wind: Deg<f64>,
    /// average annual precipitation of tile in imaginary units
    precipitation: f64,
    /// amount of water flow through tile in imaginary units
    water_flow: f64,
    /// amount of water accumulating (because it couldn't flow on) in imaginary units
    water_accumulation: f64,
    /// if the tile is in a lake, this is the id of the lake in the lakes layer
    lake_id: Option<IdRef>,
    /// id of neighboring tile which water flows to
    flow_to: Vec<Neighbor>,
    /// shortest distance in number of tiles to an ocean or lake shoreline. This will be positive on land and negative inside a water body.
    shore_distance: i32,
    /// If this is a land tile neighboring a water body, this is the id of the closest tile
    harbor_tile_id: Option<Neighbor>,
    /// if this is a land tile neighboring a water body, this is the number of neighbor tiles that are water
    water_count: Option<i32>,
    /// The biome for this tile
    biome: String,
    /// the factor used to generate population numbers, along with the area of the tile
    habitability: f64,
    /// base population of the cell outside of the towns.
    population: i32,
    /// The name of the culture assigned to this tile, unless wild
    culture: Option<String>,
    /// if the tile has a town, this is the id of the town in the towns layer
    town_id: Option<IdRef>, 
    /// if the tile is part of a nation, this is the id of the nation which controls it
    nation_id: Option<IdRef>,
    /// if the tile is part of a subnation, this is the id of the nation which controls it
    subnation_id: Option<IdRef>,
    /// If this tile is an outlet from a lake, this is the tile ID from which the water is flowing.
    outlet_from_id: Option<IdRef>,
    /// A list of all tile neighbors and their angular directions (tile_id:direction)
    neighbors: Vec<NeighborAndDirection>,
    /// A value indicating whether the tile is on the edge of the map
    #[set(allow(dead_code))] edge: Option<Edge>,

} hide_add(true) hide_doc(false)); // NOTE: I'm only using the hide_doc(false) here to prove to the compiler that we could use that option.


impl TileFeature<'_> {

    pub(crate) fn site(&self) -> Result<Coordinates,CommandError> {
        Ok(Coordinates::try_from((self.site_x()?,self.site_y()?))?)
    }

}

pub(crate) trait TileWithNeighbors: Entity<TileSchema> {

    fn neighbors(&self) -> &Vec<NeighborAndDirection>;

}

pub(crate) trait TileWithGeometry: Entity<TileSchema> {
    fn geometry(&self) -> &Polygon;
}

pub(crate) trait TileWithShoreDistance: Entity<TileSchema> {
    fn shore_distance(&self) -> &i32;
}



entity!(NewTileSite: Tile {
    geometry: Polygon,
    site: Coordinates,
    edge: Option<Edge>
}); 

entity!(TileForCalcNeighbors: Tile {
    geometry: Polygon,
    edge: Option<Edge>,
    site: Coordinates,
    neighbor_set: HashSet<IdRef> = |_| Ok::<_,CommandError>(HashSet::new()),
    cross_neighbor_set: HashSet<IdRef> = |_| Ok::<_,CommandError>(HashSet::new())
});

entity!(TileForTerrain: Tile {
    site: Coordinates, 
    elevation: f64,
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    // 'old' values so the algorithm can check if it's changed.
    old_elevation: f64 = TileFeature::elevation,
    old_grouping: Grouping = TileFeature::grouping
});

impl TileForTerrain {

    pub(crate) fn elevation_changed(&self) -> bool {
        (self.elevation - self.old_elevation).abs() > f64::EPSILON
    }

    pub(crate) fn grouping_changed(&self) -> bool {
        self.grouping != self.old_grouping
    }
}


entity!(TileForTemperatures: Tile {
    fid: IdRef, 
    site_y: f64, 
    elevation: f64, 
    grouping: Grouping
});

entity!(TileForWinds: Tile {
    fid: IdRef, 
    site_y: f64
});

entity!(TileForWaterflow: Tile {
    elevation: f64, 
    flow_to: Vec<Neighbor> = |_| Ok::<_,CommandError>(Vec::new()),
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    precipitation: f64, // not in TileForWaterFill
    temperature: f64,
    water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
});

// Basically the same struct as WaterFlow, except that the fields are initialized differently. I can't
// just use a different function because it's based on a trait. I could take this one out
// of the macro and figure something out, but this is easier.
entity!(TileForWaterFill: Tile {
    elevation: f64, 
    flow_to: Vec<Neighbor>, // Initialized to blank in TileForWaterFlow
    grouping: Grouping, 
    lake_id: Option<IdRef> = |_| Ok::<_,CommandError>(None), // Not in TileForWaterFlow
    neighbors: Vec<NeighborAndDirection>,
    outlet_from_id: Option<IdRef> = |_| Ok::<_,CommandError>(None), // Not in TileForWaterFlow
    temperature: f64,
    water_accumulation: f64,  // Initialized to blank in TileForWaterFlow
    water_flow: f64,  // Initialized to blank in TileForWaterFlow
});

impl From<TileForWaterflow> for TileForWaterFill {

    fn from(value: TileForWaterflow) -> Self {
        Self {
            elevation: value.elevation,
            temperature: value.temperature,
            grouping: value.grouping,
            neighbors: value.neighbors,
            water_flow: value.water_flow,
            water_accumulation: value.water_accumulation,
            flow_to: value.flow_to,
            outlet_from_id: None,
            lake_id: None
        }
    }
}


entity!(TileForRiverConnect: Tile {
    water_flow: f64,
    flow_to: Vec<Neighbor>,
    outlet_from_id: Option<IdRef>
});

entity!(TileForWaterDistance: Tile {
    site: Coordinates,
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    water_count: Option<i32> = |_| Ok::<_,CommandError>(None),
    closest_water_tile_id: Option<Neighbor> = |_| Ok::<_,CommandError>(None)
});


entity!(TileForGroupingCalc: Tile {
    grouping: Grouping,
    edge: Option<Edge>,
    lake_id: Option<IdRef>,
    neighbors: Vec<NeighborAndDirection>
});

entity!(TileForPopulation: Tile {
    water_flow: f64,
    elevation_scaled: i32,
    biome: String,
    shore_distance: i32,
    water_count: Option<i32>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
    harbor_tile_id: Option<Neighbor>,
    lake_id: Option<IdRef>
});

entity!(TileForPopulationNeighbor: Tile {
    grouping: Grouping,
    lake_id: Option<IdRef>
});



entity!(TileForCultureGen: Tile {
    fid: IdRef,
    site: Coordinates,
    population: i32,
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    water_count: Option<i32>,
    harbor_tile_id: Option<Neighbor>,
    grouping: Grouping,
    water_flow: f64,
    temperature: f64

});

pub(crate) struct TileForCulturePrefSorting<'struct_life> { // NOT an entity because we add in data from other layers.
    pub(crate) fid: IdRef,
    pub(crate) site: Coordinates,
    pub(crate) habitability: f64,
    pub(crate) shore_distance: i32,
    pub(crate) elevation_scaled: i32,
    pub(crate) biome: &'struct_life BiomeForCultureGen,
    pub(crate) water_count: Option<i32>,
    pub(crate) neighboring_lake_size: Option<i32>,
    pub(crate) grouping: Grouping,
    pub(crate) water_flow: f64,
    pub(crate) temperature: f64
}

impl TileForCulturePrefSorting<'_> {

    pub(crate) fn from<'biomes>(tile: TileForCultureGen, tiles: &TileLayer, biomes: &'biomes EntityLookup<BiomeSchema,BiomeForCultureGen>, lakes: &EntityIndex<LakeSchema,LakeForCultureGen>) -> Result<TileForCulturePrefSorting<'biomes>,CommandError> {
        let biome = biomes.try_get(&tile.biome)?;
        let neighboring_lake_size = if let Some(closest_water) = tile.harbor_tile_id {
            match closest_water {
                Neighbor::Tile(closest_water) | Neighbor::CrossMap(closest_water,_) => {
                    let closest_water = tiles.try_feature_by_id(&closest_water)?;
                    if let Some(lake_id) = closest_water.lake_id()? {
                        let lake_id = lake_id;
                        let lake = lakes.try_get(&lake_id)?;
                        Some(lake.size)
                    } else {
                        None
                    }
       
                },
                Neighbor::OffMap(_) => unreachable!("Why on earth would the closest_water be off the map?"),
            }
        } else {
            None
        };
        Ok(TileForCulturePrefSorting::<'biomes> {
            fid: tile.fid,
            site: tile.site,
            habitability: tile.habitability,
            shore_distance: tile.shore_distance,
            elevation_scaled: tile.elevation_scaled,
            biome,
            water_count: tile.water_count,
            neighboring_lake_size,
            grouping: tile.grouping,
            water_flow: tile.water_flow,
            temperature: tile.temperature,
        })

    }
}


entity!(TileForCultureExpand: Tile {
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<NeighborAndDirection>,
    lake_id: Option<IdRef>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
    culture: Option<String> = |_| Ok::<_,CommandError>(None)

});

entity!(TileForTowns: Tile {
    fid: IdRef,
    habitability: f64,
    site: Coordinates,
    culture: Option<String>,
    grouping_id: IdRef
});

entity!(TileForTownPopulation: Tile {
    fid: IdRef,
    geometry: Polygon,
    habitability: f64,
    site: Coordinates,
    grouping_id: IdRef,
    harbor_tile_id: Option<Neighbor>,
    water_count: Option<i32>,
    temperature: f64,
    lake_id: Option<IdRef>,
    water_flow: f64,
    grouping: Grouping
});

impl TileForTownPopulation {

    pub(crate) fn find_middle_point_between(&self, other: &Self) -> Result<Coordinates,CommandError> {
        let self_ring = self.geometry.get_ring(0)?;
        let other_ring = other.geometry.get_ring(0)?;
        let other_vertices: Vec<_> = other_ring.into_iter().collect();
        let mut common_vertices: Vec<_> = self_ring.into_iter().collect();
        common_vertices.truncate(common_vertices.len() - 1); // remove the last point, which matches the first
        common_vertices.retain(|p| other_vertices.contains(p));
        if common_vertices.len() == 2 {
            let point1: Coordinates = (common_vertices[0].0,common_vertices[0].1).try_into()?;
            let point2 = (common_vertices[1].0,common_vertices[1].1).try_into()?;
            Ok(point1.middle_point_between(&point2))
        } else {
            Err(CommandError::CantFindMiddlePoint(self.fid.clone(),other.fid.clone(),common_vertices.len()))
        }

    }

    pub(crate) fn find_middle_point_on_edge(&self, edge: &Edge, extent: &Extent) -> Result<Coordinates,CommandError> {
        let self_ring = self.geometry.get_ring(0)?;
        let mut common_vertices: Vec<_> = self_ring.into_iter().collect();
        common_vertices.truncate(common_vertices.len() - 1); // remove the last point, which matches the first
        common_vertices.retain(|p| edge.contains(p,extent));
        if common_vertices.len() > 2 {
            // NOTE: There will be a problem in cases where the edge is NE,NW,SE,SW, as there are likely going to 
            // be 3 points at least. However, this shouldn't happen since there shouldn't be any cross-map tiles in
            // those directions.
            let point1: Coordinates = (common_vertices[0].0,common_vertices[0].1).try_into()?;
            let point2 = (common_vertices[1].0,common_vertices[1].1).try_into()?;
            Ok(point1.middle_point_between(&point2))
        } else {
            Err(CommandError::CantFindMiddlePointOnEdge(self.fid.clone(),edge.clone(),common_vertices.len()))
        }

    }

}

entity!(TileForNationExpand: Tile {
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<NeighborAndDirection>,
    lake_id: Option<IdRef>,
    culture: Option<String>,
    nation_id: Option<IdRef> = |_| Ok::<_,CommandError>(None),
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
});

entity!(TileForNationNormalize: Tile {
    grouping: Grouping,
    neighbors: Vec<NeighborAndDirection>,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>
});

entity!(TileForSubnations: Tile {
    fid: IdRef,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>,
    culture: Option<String>,
    population: i32
});

entity!(TileForSubnationExpand: Tile {
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32,
    elevation_scaled: i32,
    nation_id: Option<IdRef>,
    subnation_id: Option<IdRef> = |_| Ok::<_,CommandError>(None),
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
});

entity!(TileForEmptySubnations: Tile {
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32,
    nation_id: Option<IdRef>,
    subnation_id: Option<IdRef>,
    town_id: Option<IdRef>,
    population: i32,
    culture: Option<String>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
});

entity!(TileForSubnationNormalize: Tile {
    neighbors: Vec<NeighborAndDirection>,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>,
    subnation_id: Option<IdRef>
});

entity!(TileForCultureDissolve: Tile {
    culture: Option<String>,
    geometry: Polygon,
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32
});

impl TileWithGeometry for TileForCultureDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForCultureDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForCultureDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForBiomeDissolve: Tile {
    biome: String,
    geometry: Polygon,
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32
});

impl TileWithGeometry for TileForBiomeDissolve {

    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForBiomeDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForBiomeDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForNationDissolve: Tile {
    nation_id: Option<IdRef>,
    geometry: Polygon,
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32
});

impl TileWithGeometry for TileForNationDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForNationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForNationDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForSubnationDissolve: Tile {
    subnation_id: Option<IdRef>,
    geometry: Polygon,
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32
});

impl TileWithGeometry for TileForSubnationDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForSubnationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForSubnationDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}


impl TileLayer<'_,'_> {


    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TileSchema,TileFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn try_entity_by_id<'this, Data: Entity<TileSchema> + TryFrom<TileFeature<'this>,Error=CommandError>>(&'this mut self, fid: &IdRef) -> Result<Data,CommandError> {
        self.try_feature_by_id(fid)?.try_into()
    }

    pub(crate) fn add_tile(&mut self, tile: NewTileSite) -> Result<(),CommandError> {
        // tiles are initialized with incomplete definitions in the table. It is a user error to access fields which haven't been assigned yet by running an algorithm before required algorithms are completed.

        let (x,y) = tile.site.to_tuple();

        _ = self.add_feature_with_geometry(tile.geometry,&[
                TileSchema::FIELD_SITE_X,
                TileSchema::FIELD_SITE_Y,
                TileSchema::FIELD_EDGE,
                TileSchema::FIELD_ELEVATION,
                TileSchema::FIELD_ELEVATION_SCALED,
                TileSchema::FIELD_GROUPING,
            ],&[
                x.to_field_value()?,
                y.to_field_value()?,
                tile.edge.to_field_value()?,
                // initial tiles start with 0 elevation, terrain commands will edit this...
                0.0.to_field_value()?, // FUTURE: Watch that this type stays correct
                // and scaled elevation starts with 20.
                20.to_field_value()?, // FUTURE: Watch that this type stays correct
                // tiles are continent by default until someone samples some ocean.
                Grouping::Continent.to_field_value()?
            ])?;
        Ok(())

    }

    pub(crate) fn get_layer_size(&self) -> Result<(f64,f64),CommandError> {
        let extent = self.layer.get_extent()?;
        let width = extent.MaxX - extent.MinX;
        let height = extent.MaxY - extent.MinY;
        Ok((width,height))
    }

    /// Gets average tile area in "square degrees" by dividing the width * height of the map by the number of tiles.
    pub(crate) fn estimate_average_tile_area(&self) -> Result<f64,CommandError> {
        let (width,height) = self.get_layer_size()?;
        let tiles = self.feature_count();
        Ok((width*height)/tiles as f64)
    }

    pub(crate) fn get_extent(&self) -> Result<Extent,CommandError> {
        let result = self.layer.get_extent()?;
        Ok(Extent::new(result.MinX, result.MinY, result.MaxX, result.MaxY))

    }

    // This is for when you want to generate the water fill in a second step, so you can verify the flow first.
    // It's a function here because it's used in a command, which I want to be as simple as possible.
    pub(crate) fn get_index_and_queue_for_water_fill<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<WaterFlowResult,CommandError> {

        let mut lake_queue = Vec::new();

        let tile_map = self.read_features().into_entities_index_for_each::<_,TileForWaterFill,_>(|fid,tile| {
            if tile.water_accumulation > 0.0 {
                lake_queue.push((fid.clone(),tile.water_accumulation));
            }

            Ok(())
        },progress)?;


        Ok(WaterFlowResult {
            tile_map,
            lake_queue
        })
        

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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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


layer!(River["rivers"]: MultiLineString {
    // clippy doesn't understand why I'm using 'from_*' here.
    #[get(allow(clippy::wrong_self_convention))] #[get(allow(dead_code))] #[set(allow(dead_code))] from_tile_id: IdRef,
    #[get(allow(clippy::wrong_self_convention))] #[get(allow(dead_code))] #[set(allow(dead_code))] from_type: RiverSegmentFrom,
    #[get(allow(clippy::wrong_self_convention))] #[get(allow(dead_code))] #[set(allow(dead_code))] from_flow: f64,
    #[get(allow(dead_code))] #[set(allow(dead_code))] to_tile_id: Neighbor,
    #[get(allow(dead_code))] #[set(allow(dead_code))] to_type: RiverSegmentTo,
    #[get(allow(dead_code))] #[set(allow(dead_code))] to_flow: f64,
});


impl RiverLayer<'_,'_> {

    pub(crate) fn add_segment(&mut self, new_river: &NewRiver, lines: Vec<Vec<Coordinates>>) -> Result<IdRef,CommandError> {
        let lines = lines.into_iter().map(|line| {
            LineString::from_vertices(line.into_iter().map(|p| p.to_tuple()))
        });
        let geometry = MultiLineString::from_lines(lines)?;
        self.add_struct(new_river, Some(geometry))
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

layer!(Lake["lakes"]: MultiPolygon {
    #[get(allow(dead_code))] #[set(allow(dead_code))] elevation: f64,
    #[set(allow(dead_code))] type_: LakeType,
    #[get(allow(dead_code))] #[set(allow(dead_code))] flow: f64,
    #[set(allow(dead_code))] size: i32,
    #[get(allow(dead_code))] #[set(allow(dead_code))] temperature: f64,
    #[get(allow(dead_code))] #[set(allow(dead_code))] evaporation: f64,
});

entity!(LakeForBiomes: Lake {
    type_: LakeType
});

entity!(LakeForPopulation: Lake {
    type_: LakeType
});

entity!(LakeForCultureGen: Lake {
    size: i32
});

entity!(LakeForTownPopulation: Lake {
    size: i32
});


impl LakeLayer<'_,'_> {

    pub(crate) fn add_lake(&mut self, lake: &NewLake, geometry: MultiPolygon) -> Result<IdRef,CommandError> {
        self.add_struct(lake, Some(geometry))
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<LakeSchema,LakeFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }



}

#[derive(Clone,PartialEq,Debug)]
pub(crate) enum BiomeCriteria {
    Matrix(Vec<(usize,usize)>), // moisture band, temperature band
    Wetland,
    Glacier,
    Ocean
}

impl DocumentedFieldType for (usize,usize) {

    fn get_field_type_documentation() -> FieldTypeDocumentation {
        FieldTypeDocumentation { 
            name: "Unsigned Integer Pair".to_owned(), 
            description: "A pair of unsigned integers in parentheses.".to_owned(), 
            storage_type: field_type_to_name(OGRFieldType::OFTString), 
            syntax: "(<integer>,<integer>)".to_owned(),
            sub_types: Vec::new()
        }
    }
}

impl_documentation_for_tagged_enum!{
    /// Criteria for how the biome is to be mapped to the world based on generated climate data.
    BiomeCriteria {
        /// This biome should be used for glacier -- only one is allowed
        Glacier,
        /// The biome should be placed in the following locations in the moisture and temperature matrix -- coordinates must not be used for another biome
        Matrix(list: Vec<(usize,usize)>),
        /// The biome should be used for ocean -- only one is allowed
        Ocean,
        /// The biome should be used for wetland -- only one is allowed
        Wetland,
    }
}

impl TypedField for BiomeCriteria {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string_by_name(field_name)?, field_id)?)
    }

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(field_name, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}



impl_simple_serde_tagged_enum!{
    BiomeCriteria {
        Glacier,
        Matrix(list: Vec<(usize,usize)>),
        Ocean,
        Wetland,
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

struct BiomeDefault {
    name: &'static str,
    habitability: i32,
    criteria: BiomeCriteria,
    movement_cost: i32,
    supports_nomadic: bool,
    supports_hunting: bool,
    color: &'static str,
}
        

pub(crate) struct BiomeMatrix {
    pub(crate) matrix: [[String; 26]; 5],
    pub(crate) ocean: String,
    pub(crate) glacier: String,
    pub(crate) wetland: String
}

layer!(Biome["biomes"]: MultiPolygon {
    #[set(allow(dead_code))] name: String,
    #[set(allow(dead_code))] habitability: i32,
    #[set(allow(dead_code))] criteria: BiomeCriteria,
    #[set(allow(dead_code))] movement_cost: i32,
    #[set(allow(dead_code))] supports_nomadic: bool,
    #[set(allow(dead_code))] supports_hunting: bool,
    #[set(allow(dead_code))] color: Rgb<u8>,
});

impl Entity<BiomeSchema> for NewBiome {

}

impl TryFrom<BiomeFeature<'_>> for NewBiome {

    type Error = CommandError;

    fn try_from(value: BiomeFeature) -> Result<Self,Self::Error> {
        Ok(Self {
            name: value.name()?,
            habitability: value.habitability()?,
            criteria: value.criteria()?,
            movement_cost: value.movement_cost()?,
            supports_nomadic: value.supports_nomadic()?,
            supports_hunting: value.supports_hunting()?,
            color: value.color()?,
        })
    }
}



impl<'feature> NamedFeature<'feature,BiomeSchema> for BiomeFeature<'feature> {
    fn get_name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

impl BiomeSchema {

    pub(crate) const OCEAN: &str = "Ocean";
    pub(crate) const HOT_DESERT: &str = "Hot desert";
    pub(crate) const COLD_DESERT: &str = "Cold desert";
    pub(crate) const SAVANNA: &str = "Savanna";
    pub(crate) const GRASSLAND: &str = "Grassland";
    pub(crate) const TROPICAL_SEASONAL_FOREST: &str = "Tropical seasonal forest";
    pub(crate) const TEMPERATE_DECIDUOUS_FOREST: &str = "Temperate deciduous forest";
    pub(crate) const TROPICAL_RAINFOREST: &str = "Tropical rainforest";
    pub(crate) const TEMPERATE_RAINFOREST: &str = "Temperate rainforest";
    pub(crate) const TAIGA: &str = "Taiga";
    pub(crate) const TUNDRA: &str = "Tundra";
    pub(crate) const GLACIER: &str = "Glacier";
    pub(crate) const WETLAND: &str = "Wetland";
    
    const DEFAULT_BIOMES: [BiomeDefault; 13] = [ // name, index, habitability, supports_nomadic, supports_hunting
        BiomeDefault { name: Self::OCEAN, habitability: 0, criteria: BiomeCriteria::Ocean, movement_cost: 10, supports_nomadic: false, supports_hunting: false, color: "#1F78B4"},
        BiomeDefault { name: Self::HOT_DESERT, habitability: 4, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 200, supports_nomadic: true, supports_hunting: false, color: "#FBE79F"},
        BiomeDefault { name: Self::COLD_DESERT, habitability: 10, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 150, supports_nomadic: true, supports_hunting: false, color: "#B5B887"},
        BiomeDefault { name: Self::SAVANNA, habitability: 22, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 60, supports_nomadic: false, supports_hunting: true, color: "#D2D082"},
        BiomeDefault { name: Self::GRASSLAND, habitability: 30, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 50, supports_nomadic: true, supports_hunting: false, color: "#C8D68F"},
        BiomeDefault { name: Self::TROPICAL_SEASONAL_FOREST, habitability: 50, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 70, supports_nomadic: false, supports_hunting: false, color: "#B6D95D"},
        BiomeDefault { name: Self::TEMPERATE_DECIDUOUS_FOREST, habitability: 100, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 70, supports_nomadic: false, supports_hunting: true, color: "#29BC56"},
        BiomeDefault { name: Self::TROPICAL_RAINFOREST, habitability: 80, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 80, supports_nomadic: false, supports_hunting: false, color: "#7DCB35"},
        BiomeDefault { name: Self::TEMPERATE_RAINFOREST, habitability: 90, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 90, supports_nomadic: false, supports_hunting: true, color: "#409C43"},
        BiomeDefault { name: Self::TAIGA, habitability: 12, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 200, supports_nomadic: false, supports_hunting: true, color: "#4B6B32"},
        BiomeDefault { name: Self::TUNDRA, habitability: 4, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 1000, supports_nomadic: false, supports_hunting: true, color: "#96784B"},
        BiomeDefault { name: Self::GLACIER, habitability: 0, criteria: BiomeCriteria::Glacier, movement_cost: 5000, supports_nomadic: false, supports_hunting: false, color: "#D5E7EB"},
        BiomeDefault { name: Self::WETLAND, habitability: 12, criteria: BiomeCriteria::Wetland, movement_cost: 150, supports_nomadic: false, supports_hunting: true, color: "#0B9131"},
    ];

    //these constants make the default matrix easier to read.
    const HDT: &str = Self::HOT_DESERT;
    const CDT: &str = Self::COLD_DESERT;
    const SAV: &str = Self::SAVANNA;
    const GRA: &str = Self::GRASSLAND;
    const TRF: &str = Self::TROPICAL_SEASONAL_FOREST;
    const TEF: &str = Self::TEMPERATE_DECIDUOUS_FOREST;
    const TRR: &str = Self::TROPICAL_RAINFOREST;
    const TER: &str = Self::TEMPERATE_RAINFOREST;
    const TAI: &str = Self::TAIGA;
    const TUN: &str = Self::TUNDRA;

    const DEFAULT_MATRIX: [[&str; 26]; 5] = [
        // hot ↔ cold [>19°C; <-4°C]; dry ↕ wet
        [Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::TUN],
        [Self::SAV, Self::SAV, Self::SAV, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRR, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN]
    ];

    pub(crate) fn get_default_biomes() -> Vec<NewBiome> {
        let mut matrix_criteria = HashMap::new();
        // map the matrix numbers to biome names
        for (moisture,row) in Self::DEFAULT_MATRIX.iter().enumerate() {
            for (temperature,id) in row.iter().enumerate() {
                match matrix_criteria.get_mut(id) {
                    None => {
                        _ = matrix_criteria.insert(id,vec![(moisture,temperature)]);
                    },
                    Some(entry) => entry.push((moisture,temperature)),
                }
            }

        }

        // now insert the matrix numbers into the output biomes criteria fields and return the biome entities.
        Self::DEFAULT_BIOMES.iter().map(|default| {
            let criteria = if let BiomeCriteria::Matrix(_) = default.criteria {
                BiomeCriteria::Matrix(matrix_criteria.get(&default.name).expect("Someone messed up the default biome constants.").clone())
            } else {
                default.criteria.clone()
            };
            NewBiome {
                name: (*default.name).to_owned(),
                habitability: default.habitability,
                criteria,
                movement_cost: default.movement_cost,
                supports_nomadic: default.supports_nomadic,
                supports_hunting: default.supports_hunting,
                color: string_to_color(default.color).expect("Someone messed up the biome color constants?")
            }

        }).collect()

    }

    pub(crate) fn build_matrix_from_biomes(biomes: &[NewBiome]) -> Result<BiomeMatrix,CommandError> {
        let mut matrix: [[String; 26]; 5] = Default::default();
        let mut wetland = None;
        let mut glacier = None;
        let mut ocean = None;
        for biome in biomes {
            match &biome.criteria {
                BiomeCriteria::Matrix(list) => {
                    for (moist,temp) in list {
                        let (moist,temp) = (*moist,*temp);
                        if matrix[moist][temp].is_empty() {
                            matrix[moist][temp] = biome.name.clone()

                        } else {
                            return Err(CommandError::DuplicateBiomeMatrixSlot(moist,temp))
                        }
                    }
                },
                BiomeCriteria::Wetland => if wetland.is_some() {
                    return Err(CommandError::DuplicateWetlandBiome)
                } else {
                    wetland = Some(biome.name.clone())
                },
                BiomeCriteria::Glacier => if glacier.is_some() {
                    return Err(CommandError::DuplicateGlacierBiome)
                } else {
                    glacier = Some(biome.name.clone())
                },
                BiomeCriteria::Ocean => if ocean.is_some() {
                    return Err(CommandError::DuplicateOceanBiome)
                } else {
                    ocean = Some(biome.name.clone())
                }
            }

        }
        // check for missing data
        let wetland = wetland.ok_or_else(|| CommandError::MissingWetlandBiome)?;
        let glacier = glacier.ok_or_else(|| CommandError::MissingGlacierBiome)?;
        let ocean = ocean.ok_or_else(|| CommandError::MissingOceanBiome)?;
        for (moisture,moisture_dimension) in matrix.iter().enumerate() {
            for (temperature,temperature_dimension) in moisture_dimension.iter().enumerate() {
                if temperature_dimension.is_empty() {
                    return Err(CommandError::MissingBiomeMatrixSlot(moisture,temperature))
                }
            }
        }
        Ok(BiomeMatrix { 
            matrix, 
            ocean, 
            glacier, 
            wetland 
        })
    }

}

entity!(BiomeForPopulation: Biome {
    name: String,
    habitability: i32
});

impl NamedEntity<BiomeSchema> for BiomeForPopulation {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureGen: Biome {
    name: String,
    supports_nomadic: bool,
    supports_hunting: bool
});

impl NamedEntity<BiomeSchema> for BiomeForCultureGen {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureExpand: Biome {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForCultureExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForNationExpand: Biome {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForNationExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForDissolve: Biome {
    fid: IdRef,
    name: String
});

impl NamedEntity<BiomeSchema> for BiomeForDissolve {
    fn name(&self) -> &str {
        &self.name
    }
}

impl BiomeLayer<'_,'_> {

    pub(crate) fn add_biome(&mut self, biome: &NewBiome) -> Result<IdRef,CommandError> {
        self.add_struct(biome, None)

    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<BiomeSchema,BiomeFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    pub(crate) fn get_matrix<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<BiomeMatrix,CommandError> {
        let result = self.read_features().into_entities_vec(progress)?;
    
        BiomeSchema::build_matrix_from_biomes(&result)
    
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

    fn set_field(&self, feature: &Feature, field_name: &str) -> Result<(),CommandError> {
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

layer!(Culture["cultures"]: MultiPolygon {
    #[set(allow(dead_code))] name: String,
    #[set(allow(dead_code))] namer: String,
    #[set(allow(dead_code))] type_: CultureType,
    #[set(allow(dead_code))] expansionism: f64,
    #[set(allow(dead_code))] center_tile_id: IdRef,
    #[get(allow(dead_code))] #[set(allow(dead_code))] color: Rgb<u8>,
});

impl<'feature> NamedFeature<'feature,CultureSchema> for CultureFeature<'feature> {
    fn get_name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

impl CultureSchema {

}

pub(crate) trait CultureWithNamer {

    fn namer(&self) -> &str;

    fn get_namer<'namers, Culture: CultureWithNamer>(culture: Option<&Culture>, namers: &'namers mut NamerSet) -> Result<&'namers mut Namer, CommandError> {
        let namer = namers.get_mut(culture.map(CultureWithNamer::namer))?;
        Ok(namer)
    }
    
}

pub(crate) trait CultureWithType {

    fn type_(&self) -> &CultureType;
}

// needs to be hashable in order to fit into a priority queue
entity!(#[derive(Hash,Eq,PartialEq)] CultureForPlacement: Culture {
    name: String,
    center_tile_id: IdRef,
    type_: CultureType,
    expansionism: OrderedFloat<f64> = |feature: &CultureFeature| Ok::<_,CommandError>(OrderedFloat::from(feature.expansionism()?))
});

entity!(CultureForTowns: Culture {
    name: String,
    namer: String
});

impl NamedEntity<CultureSchema> for CultureForTowns {
    fn name(&self) -> &str {
        &self.name
    }
}

impl CultureWithNamer for CultureForTowns {
    fn namer(&self) -> &str {
        &self.namer
    }
}

entity!(CultureForNations: Culture {
    name: String,
    namer: String,
    type_: CultureType
});

impl NamedEntity<CultureSchema> for CultureForNations {
    fn name(&self) -> &str {
        &self.name
    }
}

impl CultureWithNamer for CultureForNations {
    fn namer(&self) -> &str {
        &self.namer
    }
}

impl CultureWithType for CultureForNations {
    fn type_(&self) -> &CultureType {
        &self.type_
    }
}


entity!(CultureForDissolve: Culture {
    fid: IdRef,
    name: String
});

impl NamedEntity<CultureSchema> for CultureForDissolve {
    fn name(&self) -> &str {
        &self.name
    }
}


impl CultureLayer<'_,'_> {

    pub(crate) fn add_culture(&mut self, culture: &NewCulture) -> Result<IdRef,CommandError> {
        self.add_struct(culture, None)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<CultureSchema,CultureFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }


}

layer!(Town["towns"]: Point {
    #[set(allow(dead_code))] name: String,
    #[set(allow(dead_code))] culture: Option<String>,
    #[set(allow(dead_code))] is_capital: bool,
    #[set(allow(dead_code))] tile_id: IdRef,
    #[get(allow(dead_code))] #[set(allow(dead_code))] grouping_id: IdRef, 
    #[get(allow(dead_code))] population: i32,
    #[get(allow(dead_code))] is_port: bool,
});

impl TownFeature<'_> {

    pub(crate) fn move_to(&mut self, new_location: &Coordinates) -> Result<(),CommandError> {
        Ok(self.feature.set_geometry(new_location.create_geometry()?.into())?)
    }

}

entity!(TownForPopulation: Town {
    fid: IdRef,
    is_capital: bool,
    tile_id: IdRef
});

entity!(TownForNations: Town {
    fid: IdRef,
    is_capital: bool,
    culture: Option<String>,
    tile_id: IdRef
});

entity!(TownForNationNormalize: Town {
    is_capital: bool
});

entity!(TownForSubnations: Town {
    name: String
});

entity!(TownForEmptySubnations: Town {
    name: String
});

impl TownLayer<'_,'_> {

    pub(crate) fn add_town(&mut self, town: &NewTown, geometry: Point) -> Result<IdRef,CommandError> {
        self.add_struct(town, Some(geometry))
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TownSchema,TownFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    
}


layer!(Nation["nations"]: MultiPolygon {
    #[set(allow(dead_code))] name: String,
    #[set(allow(dead_code))] culture: Option<String>,
    #[set(allow(dead_code))] center_tile_id: IdRef, 
    #[set(allow(dead_code))] type_: CultureType,
    #[set(allow(dead_code))] expansionism: f64,
    #[set(allow(dead_code))] capital_town_id: IdRef,
    #[set(allow(dead_code))] color: Rgb<u8>,
});

impl<'feature> NamedFeature<'feature,NationSchema> for NationFeature<'feature> {
    fn get_name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

// needs to be hashable in order to fit into a priority queue
entity!(#[derive(Hash,Eq,PartialEq)] NationForPlacement: Nation {
    fid: IdRef,
    name: String,
    center_tile_id: IdRef,
    type_: CultureType,
    expansionism: OrderedFloat<f64> = |feature: &NationFeature| Ok::<_,CommandError>(OrderedFloat::from(feature.expansionism()?))
});

entity!(NationForSubnations: Nation {
    fid: IdRef,
    capital_town_id: IdRef,
    color: Rgb<u8>
});

entity!(NationForEmptySubnations: Nation {
    fid: IdRef,
    color: Rgb<u8>,
    culture: Option<String>
});

entity!(NationForSubnationColors: Nation {
    color: Rgb<u8>,
    subnation_count: usize = |_| Ok::<_,CommandError>(0) // to be filled in by algorithm
});

impl NationLayer<'_,'_> {

    pub(crate) fn add_nation(&mut self, nation: &NewNation) -> Result<IdRef,CommandError> {
        self.add_struct(nation, None)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<NationSchema,NationFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    

}

layer!(Subnation["subnations"]: MultiPolygon {
    #[set(allow(dead_code))] name: String,
    #[get(allow(dead_code))] #[set(allow(dead_code))] culture: Option<String>,
    #[set(allow(dead_code))] center_tile_id: IdRef,
    #[get(allow(dead_code))] #[set(allow(dead_code))] type_: CultureType,
    #[set(allow(dead_code))] seat_town_id: Option<IdRef>, 
    #[set(allow(dead_code))] nation_id: IdRef, 
    #[get(allow(dead_code))] #[set(allow(dead_code))] color: Rgb<u8>,
});

impl<'feature> NamedFeature<'feature,SubnationSchema> for SubnationFeature<'feature> {
    fn get_name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

entity!(#[derive(Hash,Eq,PartialEq)] SubnationForPlacement: Subnation {
    fid: IdRef,
    center_tile_id: IdRef,
    nation_id: IdRef
});

entity!(SubnationForNormalize: Subnation {
    center_tile_id: IdRef,
    seat_town_id: Option<IdRef>
});

entity!(SubnationForColors: Subnation {
    fid: IdRef,
    nation_id: IdRef
});

impl SubnationLayer<'_,'_> {

    pub(crate) fn add_subnation(&mut self, subnation: &NewSubnation) -> Result<IdRef,CommandError> {
        self.add_struct(subnation, None)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<SubnationSchema,SubnationFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }


}

layer!(Coastline["coastlines"]: Polygon  {
});

impl CoastlineLayer<'_,'_> {

    pub(crate) fn add_land_mass(&mut self, geometry: Polygon) -> Result<IdRef, CommandError> {
        self.add_struct(&NewCoastline {  }, Some(geometry))
    }

}

layer!(Ocean["oceans"]: Polygon {
});

impl OceanLayer<'_,'_> {

    pub(crate) fn add_ocean(&mut self, geometry: Polygon) -> Result<IdRef, CommandError> {
        self.add_struct(&NewOcean {  }, Some(geometry))
    }

}

/*
// Uncomment this stuff if you need to add a line layer for playing around with something.
feature!(Line["lines"]: LineString {
});

pub(crate) type LineLayer<'layer,'feature> = MapLayer<'layer,'feature,LineSchema,LineFeature<'feature>>;

impl LineLayer<'_,'_> {

     pub(crate) fn add_line(&mut self, line: &Vec<Point>) -> Result<u64,CommandError> {
        let geometry = crate::utils::create_line(line)?;
        self.add_feature(geometry, &[], &[])
    }
}
*/

layer!(Property["properties"]: NoGeometry {
    #[set(allow(dead_code))] name: String,
    value: String,
});

impl PropertySchema {
    const PROP_ELEVATION_LIMITS: &str = "elevation-limits";

}

pub(crate) struct ElevationLimits {
    pub(crate) min_elevation: f64,
    pub(crate) max_elevation: f64
}

impl ElevationLimits {

    pub(crate) fn new(min_elevation: f64, max_elevation: f64) -> Result<Self,CommandError> {
        if max_elevation < 0.0 {
            Err(CommandError::MaxElevationMustBePositive(max_elevation))
            // FUTURE: or should it? What if they want to create an underwater world? That won't be possible until we allow mermaid-like cultures, however,
            // and I'm not sure how "biomes" work down there.
        } else if min_elevation >= max_elevation {
            // it doesn't necessarily have to be negative, however.
            Err(CommandError::MinElevationMustBeLess(min_elevation,max_elevation))
        } else {
            Ok(Self {
                min_elevation,
                max_elevation,
            })
        }
    }
}

impl From<&ElevationLimits> for String {

    fn from(value: &ElevationLimits) -> Self {
        // store as tuple for simplicity
        (value.min_elevation,value.max_elevation).write_to_string()
    }
}


impl TryFrom<String> for ElevationLimits {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // store as tuple for simplicity
        let input: (f64,f64) = Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidPropertyValue(PropertySchema::PROP_ELEVATION_LIMITS.to_owned(),value.clone(),format!("{e}")))?;
        Ok(Self {
            min_elevation: input.0,
            max_elevation: input.1,
        })
    }
}

impl PropertyLayer<'_,'_> {

    fn get_property(&mut self, name: &str) -> Result<String,CommandError> {
        for feature in TypedFeatureIterator::<PropertySchema,PropertyFeature>::from(self.layer.features()) {
            if feature.name()? == name {
                return feature.value()
            }
        }
        Err(CommandError::PropertyNotSet(name.to_owned()))

    }

    pub(crate) fn get_elevation_limits(&mut self) -> Result<ElevationLimits,CommandError> {
        self.get_property(PropertySchema::PROP_ELEVATION_LIMITS)?.try_into()
    }

    fn set_property(&mut self, name: &str, value: &str) -> Result<IdRef,CommandError> {
        let mut found = None;
        for feature in TypedFeatureIterator::<PropertySchema,PropertyFeature>::from(self.layer.features()) {
            if feature.name()? == name {
                found = Some(feature.fid()?);
                break;
            }
        }
        if let Some(found) = found {
            let mut feature = self.try_feature_by_id(&found)?;
            feature.set_value(&value.to_owned())?;
            self.update_feature(feature)?;
            Ok(found)
        } else {
            self.add_struct(&NewProperty { 
                name: name.to_owned(), 
                value: value.to_owned() 
            }, None)
   
        }
    }

    pub(crate) fn set_elevation_limits(&mut self, value: &ElevationLimits) -> Result<IdRef,CommandError> {
        self.set_property(PropertySchema::PROP_ELEVATION_LIMITS, &Into::<String>::into(value))
    }


}


pub(crate) struct WorldMap {
    path: PathBuf,
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &str = "GPKG";

    fn new(dataset: Dataset, path: PathBuf) -> Self {
        Self { 
            path, 
            dataset 
        }
    }

    fn open_dataset<FilePath: AsRef<Path>>(path: FilePath) -> Result<Dataset, CommandError> {
        Ok(Dataset::open_ex(&path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?)
    }

    pub(crate) fn edit<FilePath: AsRef<Path> + Into<PathBuf>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Self::open_dataset(&path)?,path.into()))
    }

    pub(crate) fn create_or_edit<FilePath: AsRef<Path> + Into<PathBuf>>(path: FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(&path)?;
            Ok(Self::new(dataset,path.into()))
        }

    }

    pub(crate) fn reedit(self) -> Result<Self,CommandError> {
        // This function is necessary to work around a bug in big-bang that reminds me of days long before rust and I don't want to investigate further.
        self.dataset.close()?;
        Self::edit(self.path)
    }

    pub(crate) fn with_transaction<ResultType, Callback: FnOnce(&mut WorldMapTransaction) -> Result<ResultType,CommandError>>(&mut self, callback: Callback) -> Result<ResultType,CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        match callback(&mut transaction) {
            Ok(result) => {
                transaction.dataset.commit()?;
                Ok(result)
            },
            Err(err) => {
                transaction.dataset.rollback()?;
                Err(err)
            },
        }

    }

    pub(crate) fn save<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {
        progress.start_unknown_endpoint(|| "Saving map."); 
        self.dataset.flush_cache()?;
        progress.finish(|| "Map saved."); 
        Ok(())
    }

    pub(crate) fn points_layer(&self) -> Result<PointLayer,CommandError> {
        PointLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn tiles_layer(&self) -> Result<TileLayer,CommandError> {
        TileLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn biomes_layer(&self) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn cultures_layer(&self) -> Result<CultureLayer, CommandError> {
        CultureLayer::open_from_dataset(&self.dataset)
    }



 

}

pub(crate) struct WorldMapTransaction<'data_life> {
    dataset: Transaction<'data_life>
}

impl<'impl_life> WorldMapTransaction<'impl_life> {

    fn new(dataset: Transaction<'impl_life>) -> Self {
        Self {
            dataset
        }
    }

    pub(crate) fn create_points_layer(&mut self, overwrite: bool) -> Result<PointLayer,CommandError> {
        PointLayer::create_from_dataset(&mut self.dataset, overwrite)       

    }

    pub(crate) fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TriangleLayer,CommandError> {
        TriangleLayer::create_from_dataset(&mut self.dataset, overwrite)

    }

    pub(crate) fn edit_triangles_layer(&self) -> Result<TriangleLayer, CommandError> {
        TriangleLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_tile_layer(&mut self, overwrite: &OverwriteTilesArg) -> Result<TileLayer,CommandError> {
        TileLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_tiles)

    }

    pub(crate) fn create_rivers_layer(&mut self, overwrite: &OverwriteRiversArg) -> Result<RiverLayer,CommandError> {
        RiverLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_rivers)

    }

    pub (crate) fn create_lakes_layer(&mut self, overwrite_layer: &OverwriteLakesArg) -> Result<LakeLayer,CommandError> {
        LakeLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_lakes)
    }

    pub (crate) fn edit_lakes_layer(&mut self) -> Result<LakeLayer,CommandError> {
        LakeLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn edit_tile_layer(&mut self) -> Result<TileLayer,CommandError> {
        TileLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_biomes_layer(&mut self, overwrite: &OverwriteBiomesArg) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_biomes)
    }

    pub(crate) fn edit_biomes_layer(&mut self) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_cultures_layer(&mut self, overwrite: &OverwriteCulturesArg) -> Result<CultureLayer,CommandError> {
        CultureLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_cultures)
    }

    pub(crate) fn edit_cultures_layer(&mut self) -> Result<CultureLayer,CommandError> {
        CultureLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_towns_layer(&mut self, overwrite_layer: &OverwriteTownsArg) -> Result<TownLayer,CommandError> {
        TownLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_towns)
    }

    pub(crate) fn edit_towns_layer(&mut self) -> Result<TownLayer,CommandError> {
        TownLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_nations_layer(&mut self, overwrite_layer: &OverwriteNationsArg) -> Result<NationLayer,CommandError> {
        NationLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_nations)
    }

    pub(crate) fn edit_nations_layer(&mut self) -> Result<NationLayer,CommandError> {
        NationLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_subnations_layer(&mut self, overwrite_layer: &OverwriteSubnationsArg) -> Result<SubnationLayer,CommandError> {
        SubnationLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_subnations)
    }

    pub(crate) fn edit_subnations_layer(&mut self) -> Result<SubnationLayer,CommandError> {
        SubnationLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_coastline_layer(&mut self, overwrite_coastline: &OverwriteCoastlineArg) -> Result<CoastlineLayer,CommandError> {
        CoastlineLayer::create_from_dataset(&mut self.dataset, overwrite_coastline.overwrite_coastline)
    }

    pub(crate) fn create_ocean_layer(&mut self, overwrite_ocean: &OverwriteOceanArg) -> Result<OceanLayer,CommandError> {
        OceanLayer::create_from_dataset(&mut self.dataset, overwrite_ocean.overwrite_ocean)
    }

    /* Uncomment this to add a line layer for playing around with ideas.
     pub(crate) fn create_lines_layer(&mut self, overwrite: bool) -> Result<LineLayer,CommandError> {
        Ok(LineLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }
    */

    pub(crate) fn create_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        PropertyLayer::create_from_dataset(&mut self.dataset,true)
    }

    pub(crate) fn edit_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        PropertyLayer::open_from_dataset(&self.dataset)
    }

}
