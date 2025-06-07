use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FormatResult;

use gdal::vector::Feature;
use gdal::vector::field_type_to_name;
use gdal::vector::FieldValue;
use gdal::vector::OGRFieldType;
use ordered_float::NotNan;

use crate::errors::CommandError;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Serialize;
use crate::utils::simple_serde::Deserializer;
use crate::utils::simple_serde::Serializer;

#[derive(Clone,PartialEq)]
pub(crate) struct FieldTypeDocumentation {
    name: String,
    description: String, // More detailed description for the format
    storage_type: String, // the concrete field type in the database
    syntax: String, // syntax for the format
    sub_types: Vec<FieldTypeDocumentation>

}

impl FieldTypeDocumentation {

    pub(crate) const fn new(name: String, description: String, storage_type: String, syntax: String, sub_types: Vec<Self>) -> Self {
        Self { 
            name, 
            description, 
            storage_type, 
            syntax, 
            sub_types 
        }
    }
    
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    
    pub(crate) fn description(&self) -> &str {
        &self.description
    }
    
    pub(crate) fn storage_type(&self) -> &str {
        &self.storage_type
    }
    
    pub(crate) fn syntax(&self) -> &str {
        &self.syntax
    }
    
    pub(crate) fn sub_types(&self) -> &[Self] {
        &self.sub_types
    }
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

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError>;

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
        result.push_str(variant);
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

#[macro_export]
macro_rules! impl_documentation_for_tagged_enum {

    (#[doc=$description: literal] $enum: ty {$(#[doc=$variant_description: literal] $variant: ident $(($($tuple_name: ident: $tuple_type: ty),*$(,)?))?),*$(,)?}) => {
        impl $crate::typed_map::fields::DocumentedFieldType for $enum {

            fn get_field_type_documentation() -> $crate::typed_map::fields::FieldTypeDocumentation {

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

                let (syntax,sub_types) = $crate::typed_map::fields::describe_variant_syntax(&[$((stringify!($variant),&[$(&[$(<$tuple_type>::get_field_type_documentation()),*])?])),*]);

                $crate::typed_map::fields::FieldTypeDocumentation::new(
                    stringify!($enum).to_owned(),
                    description,
                    field_type_to_name(<$enum>::STORAGE_TYPE), // TODO: Should take this from a TypedField trait instead
                    syntax,
                    sub_types
                )
            }
    
        }

        // This enforce that we have all of the same enum values
        impl $enum {

            #[allow(clippy::missing_const_for_fn)] // no it can't be a const function because it owns 'self' and anyway it's not intended to ever be called.
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

#[derive(PartialEq,Eq,Hash,PartialOrd,Ord,Clone,Debug)]
pub struct IdRef(u64);

impl IdRef {

    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    pub(crate) const fn to_inner(&self) -> u64 {
        self.0
    }

}

impl Display for IdRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
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
        Deserialize::read_from_str(&Self::get_required(feature.field_as_string(feature.field_index(field_name)?)?, field_id)?)
    }


    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(feature.field_index(field_name)?, &self.write_to_string())?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.write_to_string())))
    }

}

impl TypedField for Option<IdRef> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        feature.field_as_string(feature.field_index(field_name)?)?.map(|a| Deserialize::read_from_str(&a)).transpose()
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(value.set_field(feature,field_name)?)
        } else {
            Ok(feature.set_field_null(feature.field_index(field_name)?)?)
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

impl TypedField for String {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;

    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::get_required(feature.field_as_string(feature.field_index(field_name)?)?, field_id)
    }


    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_string(feature.field_index(field_name)?, self)?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::StringValue(self.clone())))
    }

}



impl TypedField for Option<String> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTString;


    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        if let Some(value) = feature.field_as_string(feature.field_index(field_name)?)? {
            Ok(Some(value))
        } else {
            Ok(None)
        }

    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(value.set_field(feature,field_name)?)
        } else {
            Ok(feature.set_field_null(feature.field_index(field_name)?)?)
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
        Ok(Self::get_required(feature.field_as_integer(feature.field_index(field_name)?)?, field_id)? != 0)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_integer(feature.field_index(field_name)?, (*self).into())?)

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



impl TypedField for f64 {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTReal;


    fn get_field(feature: &Feature, field_name: &str, field_id: &'static str) -> Result<Self,CommandError> {
        Self::get_required(feature.field_as_double(feature.field_index(field_name)?)?, field_id)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        // The NotNan thing verifies that the value is not NaN, which would be treated as null.
        // This can help me catch math problems early...
        Ok(feature.set_field_double(feature.field_index(field_name)?, NotNan::try_from(*self)?.into_inner())?)

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
        Self::get_required(feature.field_as_integer(feature.field_index(field_name)?)?, field_id)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        Ok(feature.set_field_integer(feature.field_index(field_name)?,*self)?)
    }

    fn to_field_value(&self) -> Result<Option<FieldValue>,CommandError> {
        Ok(Some(FieldValue::IntegerValue(*self)))
    }
    

}

impl TypedField for Option<i32> {

    const STORAGE_TYPE: OGRFieldType::Type = OGRFieldType::OFTInteger;


    
    fn get_field(feature: &Feature, field_name: &str, _: &'static str) -> Result<Self,CommandError> {
        Ok(feature.field_as_integer(feature.field_index(field_name)?)?)
    }

    fn set_field(&self, feature: &mut Feature, field_name: &str) -> Result<(),CommandError> {
        if let Some(value) = self {
            Ok(feature.set_field_integer(feature.field_index(field_name)?, *value)?)
        } else {
            Ok(feature.set_field_null(feature.field_index(field_name)?)?)
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
    name: String,
    description: String,
    field_type: FieldTypeDocumentation
}

impl FieldDocumentation {
    pub(crate) const fn new(name: String, description: String, field_type: FieldTypeDocumentation) -> Self {
        Self { 
            name, 
            description, 
            field_type 
        }
    }
    
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    
    pub(crate) fn description(&self) -> &str {
        &self.description
    }
    
    pub(crate) const fn field_type(&self) -> &FieldTypeDocumentation {
        &self.field_type
    }
}
