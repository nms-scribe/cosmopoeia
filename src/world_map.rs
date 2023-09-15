use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::hash::Hash;
use std::collections::HashSet;
use std::collections::hash_map::IntoIter;

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
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::Feature;
use gdal::vector::FeatureIterator;
use gdal::Transaction;
use ordered_float::OrderedFloat;
use ordered_float::NotNan;
use indexmap::IndexMap;
// rename these imports in case I want to use these from hash_map sometime.
use indexmap::map::Keys as IndexKeys;
use indexmap::map::Iter as IndexIter;
use indexmap::map::IterMut as IndexIterMut;
use indexmap::map::IntoIter as IndexIntoIter;
use serde::Serialize;
use serde::Deserialize;
use ron::to_string as to_ron_string;
use ron::from_str as from_ron_str;
use paste::paste;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::utils::LayerGeometryIterator;
use crate::utils::Point;
use crate::utils::Extent;
use crate::utils::create_line;
use crate::utils::title_case::ToTitleCase;
use crate::gdal_fixes::FeatureFix;
use crate::algorithms::naming::NamerSet;
use crate::algorithms::naming::Namer;
use crate::algorithms::naming::LoadedNamers;


// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.

// FUTURE: Another problem with the gdal crate is the lifetimes. Feature, for example, only requires the lifetimes because it holds a reference to 
// a field definition pointer, which is never used except in the constructor. Once the feature is created, this reference could easily be forgotten. Layer is
// a little more complex, it holds a phantom value of the type of a reference to its dataset. On the one hand, it also doesn't do anything with it at all,
// on the other this reference might keep it from outliving it's dataset reference. Which, I guess, is the same with Feature, so maybe that's what they're 
// doing. I just wish there was another way, as it would make the TypedFeature stuff I'm trying to do below work better. However, if that were built into
// the gdal crate, maybe it would be better.

fn id_list_to_string(value: &Vec<u64>) -> String {
    to_ron_string(value).unwrap() // it's a basic enum, there shouldn't be any reason why this would fail
}

fn string_to_id_list(value: String) -> Result<Vec<u64>,CommandError> {
    from_ron_str(&value).map_err(|_| CommandError::InvalidValueForIdList(value))   
}

fn neighbor_directions_to_string(value: &Vec<(u64,i32)>) -> String {
    to_ron_string(value).unwrap() // it's a basic enum, there shouldn't be any reason why this would fail
}

fn string_to_neighbor_directions(value: String) -> Result<Vec<(u64,i32)>,CommandError> {
    from_ron_str(&value).map_err(|_| CommandError::InvalidValueForNeighborDirections(value))   
}

fn id_ref_to_string(value: &u64) -> String {
    to_ron_string(value).unwrap() // it's an f64, what's it going to do?
}

fn string_to_id_ref(value: String) -> Result<u64,CommandError> {
    from_ron_str(&value).map_err(|_| CommandError::InvalidValueForIdRef(value))
}

macro_rules! feature_get_field_type {
    (f64) => {
        f64
    };
    (i64) => {
        i64
    };
    (id_ref) => {
        u64
    };
    (i32) => {
        i32
    };
    (bool) => {
        bool
    };
    (option_i64) => {
        Option<i64> // this is the same because everything's an option, the option tag only means it can accept options
    };
    (option_id_ref) => {
        Option<u64>
    };
    (option_i32) => {
        Option<i32> // this is the same because everything's an option, the option tag only means it can accept options
    };
    (neighbor_directions) => {
        Vec<(u64,i32)>
    };
    (id_list) => {
        Vec<u64>
    };
    (river_segment_from) => {
        RiverSegmentFrom
    };
    (river_segment_to) => {
        RiverSegmentTo
    };
    (string) => {
        String
    };
    (option_string) => {
        Option<String>
    };
    (biome_criteria) => {
        BiomeCriteria
    };
    (lake_type) => {
        LakeType
    };
    (grouping) => {
        Grouping
    };
    (culture_type) => {
        CultureType
    };
}

macro_rules! feature_set_field_type {
    (f64) => {
        f64
    };
    (i64) => { // TODO: Remove?
        i64
    };
    (option_i64) => {
        Option<i64>
    };
    (id_ref) => {
        u64
    };
    (option_id_ref) => {
        Option<u64>
    };
    (i32) => {
        i32
    };
    (option_i32) => {
        Option<i32>
    };
    (bool) => {
        bool
    };
    (neighbor_directions) => {
        &Vec<(u64,i32)>
    };
    (id_list) => {
        &Vec<u64>
    };
    (river_segment_from) => {
        &RiverSegmentFrom
    };
    (river_segment_to) => {
        &RiverSegmentTo
    };
    (string) => {
        &str
    };
    (option_string) => {
        Option<&str>
    };
    (biome_criteria) => {
        &BiomeCriteria
    };
    (lake_type) => {
        &LakeType
    };
    (grouping) => {
        &Grouping
    };
    (culture_type) => {
        &CultureType
    };
}

macro_rules! feature_get_required {
    ($feature_name: literal $prop: ident $value: expr ) => {
        $value.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))
    };
}

macro_rules! feature_get_field {
    ($self: ident f64 $feature_name: literal $prop: ident $field: path) => {
        Ok(feature_get_required!($feature_name $prop $self.feature.field_as_double_by_name($field)?)?)
    };
    ($self: ident i64 $feature_name: literal $prop: ident $field: path) => {
        Ok(feature_get_required!($feature_name $prop $self.feature.field_as_integer64_by_name($field)?))
    };
    ($self: ident id_ref $feature_name: literal $prop: ident $field: path) => {
        Ok(string_to_id_ref(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident option_i64 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer64_by_name($field)?)
    };
    ($self: ident option_id_ref $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_string_by_name($field)?.map(|a| string_to_id_ref(a)).transpose()?)
    };
    ($self: ident i32 $feature_name: literal $prop: ident $field: path) => {
        Ok(feature_get_required!($feature_name $prop $self.feature.field_as_integer_by_name($field)?)?)
    };
    ($self: ident option_i32 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?)
    };
    ($self: ident bool $feature_name: literal $prop: ident $field: path) => {
        Ok(feature_get_required!($feature_name $prop $self.feature.field_as_integer_by_name($field)?)? != 0)
    };
    ($self: ident neighbor_directions $feature_name: literal $prop: ident $field: path) => {
        Ok(string_to_neighbor_directions(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident id_list $feature_name: literal $prop: ident $field: path) => {
        Ok(string_to_id_list(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident river_segment_from $feature_name: literal $prop: ident $field: path) => {
        Ok(RiverSegmentFrom::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident river_segment_to $feature_name: literal $prop: ident $field: path) => {
        Ok(RiverSegmentTo::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident string $feature_name: literal $prop: ident $field: path) => {
        Ok(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)
    };
    ($self: ident option_string $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            if value == "" {
                // we're storing null strings as empty for now.
                Ok(None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    };
    ($self: ident biome_criteria $feature_name: literal $prop: ident $field: path) => {
        Ok(BiomeCriteria::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident lake_type $feature_name: literal $prop: ident $field: path) => {
        Ok(LakeType::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident grouping $feature_name: literal $prop: ident $field: path) => {
        Ok(Grouping::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
    ($self: ident culture_type $feature_name: literal $prop: ident $field: path) => {
        Ok(CultureType::try_from(feature_get_required!($feature_name $prop $self.feature.field_as_string_by_name($field)?)?)?)
    };
}

macro_rules! feature_set_field {
    ($self: ident $value: ident f64 $field: path) => {
        // The NotNan thing verifies that the value is not NaN, which would be treated as null.
        // This can help me catch math problems early...
        Ok($self.feature.set_field_double($field, NotNan::try_from($value)?.into_inner())?)
    };
    ($self: ident $value: ident i32 $field: path) => {
        Ok($self.feature.set_field_integer($field, $value)?)
    };
    ($self: ident $value: ident option_i32 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_integer($field, value)?)
        } else {
            Ok($self.feature.set_field_null($field)?)
        }
    };
    ($self: ident $value: ident i64 $field: path) => {
        Ok($self.feature.set_field_integer64($field, $value)?)
    };
    ($self: ident $value: ident id_ref $field: path) => {
        Ok($self.feature.set_field_string($field, &id_ref_to_string(&$value))?)
    };
    ($self: ident $value: ident option_id_ref $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_string($field, &id_ref_to_string(&value))?)
        } else {
            Ok($self.feature.set_field_null($field)?)
        }
    };
    ($self: ident $value: ident option_i64 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_integer64($field, value)?)
        } else {
            Ok($self.feature.set_field_null($field)?)
        }
    };
    ($self: ident $value: ident bool $field: path) => {
        Ok($self.feature.set_field_integer($field, $value.into())?)
    };
    ($self: ident $value: ident neighbor_directions $field: path) => {{
        let neighbors = neighbor_directions_to_string($value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    ($self: ident $value: ident id_list $field: path) => {{
        let neighbors = id_list_to_string($value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    ($self: ident $value: ident river_segment_from $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident river_segment_to $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident string $field: path) => {{
        Ok($self.feature.set_field_string($field, $value)?)
    }};
    ($self: ident $value: ident option_string $field: path) => {{
        if let Some(value) = $value {
            Ok($self.feature.set_field_string($field, value)?)
        } else {
            Ok($self.feature.set_field_null($field)?)
        }        
    }};
    ($self: ident $value: ident biome_criteria $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident lake_type $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident grouping $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident culture_type $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
}

macro_rules! feature_to_value {
    ($prop: expr; f64) => {
        Some(FieldValue::RealValue(NotNan::<f64>::try_from($prop)?.into_inner()))
    };
    ($prop: expr; i32) => {
        Some(FieldValue::IntegerValue($prop))
    };
    ($prop: expr; bool) => {
        Some(FieldValue::IntegerValue($prop.into()))
    };
    ($prop: expr; option_i32) => {
        if let Some(value) = $prop {
            Some(FieldValue::IntegerValue(value))
        } else {
            None
        }
    };
    ($prop: expr; option_i64) => {
        if let Some(value) = $prop {
            Some(FieldValue::Integer64Value(value))
        } else {
            None
        }
    };
    ($prop: expr; option_id_ref) => {
        if let Some(value) = $prop {
            Some(FieldValue::StringValue(id_ref_to_string(&value)))
        } else {
            None
        }
    };
    ($prop: expr; id_list) => {
        Some(FieldValue::StringValue(id_list_to_string($prop)))
    };
    ($prop: expr; neighbor_directions) => {
        Some(FieldValue::StringValue(neighbor_directions_to_string($prop)))
    };
    ($prop: expr; i64) => {
        Some(FieldValue::Integer64Value($prop))
    };
    ($prop: expr; id_ref) => {
        // store id_ref as a string so I can use u64, as fields only support i64
        Some(FieldValue::StringValue(id_ref_to_string(&$prop)))
    };
    ($prop: expr; river_segment_from) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};
    ($prop: expr; river_segment_to) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};
    ($prop: expr; string) => {{
        Some(FieldValue::StringValue($prop.to_owned()))
    }};
    ($prop: expr; option_string) => {{
        if let Some(value) = $prop {
            Some(FieldValue::StringValue(value.to_owned()))
        } else {
            None
        }
    }};
    ($prop: expr; biome_criteria) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};
    ($prop: expr; lake_type) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};
    ($prop: expr; grouping) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};
    ($prop: expr; culture_type) => {{
        Some(FieldValue::StringValue(Into::<String>::into($prop)))
    }};

}

pub(crate) trait Schema {

    const GEOMETRY_TYPE: OGRwkbGeometryType::Type;

    const LAYER_NAME: &'static str;

    fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)];

}



pub(crate) trait TypedFeature<'data_life,SchemaType: Schema>: From<Feature<'data_life>>  {

    fn fid(&self) -> Result<u64,CommandError>;

    fn into_feature(self) -> Feature<'data_life>;

    fn geometry(&self) -> Result<&Geometry,CommandError>;

    fn set_geometry(&mut self, geometry: Geometry) -> Result<(),CommandError>;

}

pub(crate) trait NamedFeature<'data_life,SchemaType: Schema>: TypedFeature<'data_life,SchemaType> {

    fn name(&self) -> Result<String,CommandError>;

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

macro_rules! get_field_type_for_prop_type {
    (f64) => {
        OGRFieldType::OFTReal
    };
    (i32) => {
        OGRFieldType::OFTInteger
    };
    (grouping) => {
        OGRFieldType::OFTString
    };
    (id_ref) => {
        OGRFieldType::OFTString
    };
    (option_id_ref) => {
        OGRFieldType::OFTString
    };
    (id_list) => {
        OGRFieldType::OFTString
    };
    (option_i32) => {
        OGRFieldType::OFTInteger
    };
    (string) => {
        OGRFieldType::OFTString
    };
    (option_string) => {
        OGRFieldType::OFTString
    };
    (option_i64) => {
        OGRFieldType::OFTInteger64
    };
    (neighbor_directions) => {
        OGRFieldType::OFTString
    };
    (i64) => {
        OGRFieldType::OFTInteger64
    };
    (river_segment_from) => {
        OGRFieldType::OFTString
    };
    (river_segment_to) => {
        OGRFieldType::OFTString
    };
    (lake_type) => {
        OGRFieldType::OFTString
    };
    (biome_criteria) => {
        OGRFieldType::OFTString
    };
    (bool) => {
        OGRFieldType::OFTInteger
    };
    (culture_type) => {
        OGRFieldType::OFTString
    }
}

macro_rules! feature {
    ($struct_name:ident $schema_name: ident $layer_name: literal $geometry_type: ident $(to_field_names_values: #[$to_values_attr: meta])? {$(
        $(#[$get_attr: meta])* $prop: ident $(#[$set_attr: meta])*
        $prop_type: ident 
        $field: ident 
        $name: literal;
    )*}) => {

        pub(crate) struct $struct_name<'data_life> {

            feature: Feature<'data_life>
        }
        
        impl<'impl_life> From<Feature<'impl_life>> for $struct_name<'impl_life> {
        
            fn from(feature: Feature<'impl_life>) -> Self {
                Self {
                    feature
                }
            }
        }

        pub(crate) struct $schema_name {

        }

        impl $schema_name {
            // constant field names
            $(pub(crate) const $field: &str = stringify!($prop);)*

            // field definitions
            const FIELD_DEFS: [(&str,OGRFieldType::Type); feature_count_fields!($($field),*)] = [
                $((Self::$field,get_field_type_for_prop_type!($prop_type))),*
            ];


        }

        impl Schema for $schema_name {

            const GEOMETRY_TYPE: OGRwkbGeometryType::Type = OGRwkbGeometryType::$geometry_type;

            const LAYER_NAME: &'static str = $layer_name;

            fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)] {
                &Self::FIELD_DEFS
            }


        }

        impl<'impl_life> TypedFeature<'impl_life,$schema_name> for $struct_name<'impl_life> {

            // fid field
            fn fid(&self) -> Result<u64,CommandError> {
                self.feature.fid().ok_or_else(|| CommandError::MissingField(concat!($layer_name,".","fid")))
            }

            fn into_feature(self) -> Feature<'impl_life> {
                self.feature
            }

            fn geometry(&self) -> Result<&Geometry,CommandError> { 
                self.feature.geometry().ok_or_else(|| CommandError::MissingGeometry($layer_name))
            }
    
            fn set_geometry(&mut self, geometry: Geometry) -> Result<(),CommandError> { 
                Ok(self.feature.set_geometry(geometry)?)
            }
        }
        
        
        impl $struct_name<'_> {

            // feature initializer function
            $(#[$to_values_attr])? pub(crate) fn to_field_names_values($($prop: feature_set_field_type!($prop_type)),*) -> Result<([&'static str; feature_count_fields!($($field),*)],[Option<FieldValue>; feature_count_fields!($($field),*)]),CommandError> {
                Ok(([
                    $($schema_name::$field),*
                ],[
                    $(feature_to_value!($prop; $prop_type)),*
                ]))
    
            }
        
            // property functions
            $(
                $(#[$get_attr])* pub(crate) fn $prop(&self) -> Result<feature_get_field_type!($prop_type),CommandError> {
                    feature_get_field!(self $prop_type $layer_name $prop $schema_name::$field)
                }
        
                paste!{
                    $(#[$set_attr])* pub(crate) fn [<set_ $prop>](&mut self, value: feature_set_field_type!($prop_type)) -> Result<(),CommandError> {
                        feature_set_field!(self value $prop_type $schema_name::$field)
                    }            
    
                }
        
            )*
        }

        

    };
}




pub(crate) struct TypedFeatureIterator<'data_life, SchemaType: Schema, Feature: TypedFeature<'data_life,SchemaType>> {
    features: FeatureIterator<'data_life>,
    _phantom_feature: std::marker::PhantomData<Feature>,
    _phantom_schema: std::marker::PhantomData<SchemaType>
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
            _phantom_feature: Default::default(),
            _phantom_schema: Default::default()
        }
    }
}

impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>> TypedFeatureIterator<'impl_life, SchemaType, Feature> {

    pub(crate) fn to_entities_vec<'local, Progress: ProgressObserver, Data: TryFrom<Feature,Error=CommandError>>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        let mut result = Vec::new();
        for entity in self.watch(progress,format!("Reading {}.",SchemaType::LAYER_NAME),format!("{} read.",SchemaType::LAYER_NAME.to_title_case())) {
            result.push(Data::try_from(entity)?);
        }
        Ok(result)
    }

    pub(crate) fn into_entities<Data: Entity<SchemaType>>(self) -> EntityIterator<'impl_life, SchemaType, Feature, Data> {
        self.into()
    }


    pub(crate) fn to_entities_index<Progress: ProgressObserver, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>>(&mut self, progress: &mut Progress) -> Result<EntityIndex<SchemaType,Data>,CommandError> {

        self.to_entities_index_for_each(|_,_| Ok(()), progress)

    }

    pub(crate) fn to_entities_index_for_each<Progress: ProgressObserver, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>, Callback: FnMut(&u64,&Data) -> Result<(),CommandError>>(&mut self, mut callback: Callback, progress: &mut Progress) -> Result<EntityIndex<SchemaType,Data>,CommandError> {

        let mut result = IndexMap::new();
        for feature in self.watch(progress,format!("Indexing {}.",SchemaType::LAYER_NAME),format!("{} indexed.",SchemaType::LAYER_NAME.to_title_case())) {
            let fid = feature.fid()?;
            let entity = Data::try_from(feature)?;

            callback(&fid,&entity)?;
    
            result.insert(fid,entity);
        }

        Ok(EntityIndex::from(result))
    }

    pub(crate) fn to_named_entities_index<'local, Progress: ProgressObserver, Data: NamedEntity<SchemaType> + TryFrom<Feature,Error=CommandError>>(&'local mut self, progress: &mut Progress) -> Result<EntityLookup<SchemaType, Data>,CommandError> {
        let mut result = HashMap::new();

        for feature in self.watch(progress,format!("Indexing {}.",SchemaType::LAYER_NAME),format!("{} indexed.",SchemaType::LAYER_NAME.to_title_case())) {
            let entity = Data::try_from(feature)?;
            let name = entity.name().to_owned();
            result.insert(name, entity);
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
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'impl_life, SchemaType: Schema, Feature: TypedFeature<'impl_life,SchemaType>, Data: Entity<SchemaType> + TryFrom<Feature,Error=CommandError>> Iterator for EntityIterator<'impl_life,SchemaType,Feature,Data> {
    type Item = Result<(u64,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from(feature)) {
                (Ok(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (Err(e), Ok(_)) => Some(Err(e)),
                (_, Err(e)) => Some(Err(e)),
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
            data: Default::default()
        }
    }
}

pub(crate) struct EntityIndex<SchemaType: Schema, EntityType: Entity<SchemaType>> {
    // I use an IndexMap instead of HashMap as it ensures that the map maintains an order when iterating.
    // This helps me get reproducible results with the same random seed.
    inner: IndexMap<u64,EntityType>,
    _phantom: std::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> EntityIndex<SchemaType,EntityType> {

    // NOTE: There is no 'insert' or 'new' function because this should be created with to_entities_index.

    fn from(mut inner: IndexMap<u64,EntityType>) -> Self {
        // I want to ensure that the tiles are sorted in insertion order (by fid). So do this here.
        // if there were an easy way to insert_sorted from the beginning, then I wouldn't need to do this.
        inner.sort_keys();
        Self {
            inner,
            _phantom: Default::default()
        }
    }

    pub(crate) fn try_get(&self, key: &u64) -> Result<&EntityType,CommandError> {
        self.inner.get(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, *key))
    }

    pub(crate) fn try_get_mut(&mut self, key: &u64) -> Result<&mut EntityType,CommandError> {
        self.inner.get_mut(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, *key))
    }

    pub(crate) fn try_remove(&mut self, key: &u64) -> Result<EntityType,CommandError> {
        self.inner.remove(key).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME, *key))
    }

    pub(crate) fn keys(&self) -> IndexKeys<'_, u64, EntityType> {
        self.inner.keys()
    }

    pub(crate) fn iter(&self) -> IndexIter<'_, u64, EntityType> {
        self.inner.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> IndexIterMut<'_, u64, EntityType> {
        self.inner.iter_mut()
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn maybe_get(&self, key: &u64) -> Option<&EntityType> {
        self.inner.get(key)
    }

}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> IntoIterator for EntityIndex<SchemaType,EntityType> {
    type Item = (u64,EntityType);

    type IntoIter = IndexIntoIter<u64,EntityType>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> FromIterator<(u64,EntityType)> for EntityIndex<SchemaType,EntityType> {

    fn from_iter<Iter: IntoIterator<Item = (u64,EntityType)>>(iter: Iter) -> Self {
        Self::from(IndexMap::from_iter(iter))
    }
}
        


pub(crate) struct EntityLookup<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> {
    inner: HashMap<String,EntityType>,
    _phantom: std::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> EntityLookup<SchemaType,EntityType> {

    fn from(inner: HashMap<String,EntityType>) -> Self {
        Self {
            inner,
            _phantom: Default::default()
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
    ($feature: ident geometry $type: ty) => {
        $feature.geometry()?.clone()
    };
    ($feature: ident fid $type: ty) => {
        $feature.fid()?
    };
    ($feature: ident $field: ident $type: ty) => {
        $feature.$field()?
    };
    ($feature: ident $field: ident $type: ty = $function: expr) => {
        $function(&$feature)?
    };
}

#[macro_export]
/// Used by `entity!` to generate an expression which tries to convert a feature into an entity.
macro_rules! entity_from_data {
    ($name: ident $feature: ident, $($field: ident: $type: ty $(= $function: expr)?),*) => {{
        Ok($name {
            $(
                $field: crate::entity_field_assign!($feature $field $type $(= $function)?)
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
    ($(#[$struct_attr: meta])* $name: ident $schema: ident $feature: ident {$($field: ident: $type: ty $(= $function: expr)?),*}) => {
        #[derive(Clone)]
        $(#[$struct_attr])* 
        pub(crate) struct $name {
            $(
                pub(crate) $field: crate::entity_field_def!($type $([$function])?)
            ),*
        }

        impl<'impl_life> Entity<$schema> for $name {

        }

        impl TryFrom<$feature<'_>> for $name {

            type Error = CommandError;

            fn try_from(value: $feature) -> Result<Self,Self::Error> {
                crate::entity_from_data!($name value, $($field: $type $(= $function)?),*)
            }
        }

    };
}

pub(crate) struct MapLayer<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> {
    layer: Layer<'layer>,
    _phantom_feature: std::marker::PhantomData<&'feature Feature>,
    _phantom_schema: std::marker::PhantomData<SchemaType>
}

impl<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> MapLayer<'layer,'feature,SchemaType,Feature> {


    fn create_from_dataset(dataset: &'layer mut Dataset, overwrite: bool) -> Result<Self,CommandError> {

        // 4326 is WGS 84, although this is a fictional world and isn't necessarily shaped like Earth.
        // That coordinate system just seems "safe" as far as other tools are expecting an Earth-shape.
        let srs = SpatialRef::from_epsg(4326)?;
        let layer = dataset.create_layer(LayerOptions {
            name: SchemaType::LAYER_NAME,
            ty: SchemaType::GEOMETRY_TYPE,
            srs: if SchemaType::GEOMETRY_TYPE == OGRwkbGeometryType::wkbNone {
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
            _phantom_feature: Default::default(),
            _phantom_schema: Default::default()
        })
    }

    fn open_from_dataset(dataset: &'layer Dataset) -> Result<Self,CommandError> {
        
        let layer = dataset.layer_by_name(SchemaType::LAYER_NAME)?;
        Ok(Self {
            layer,
            _phantom_feature: Default::default(),
            _phantom_schema: Default::default()
        })

    }

    // FUTURE: I wish I could get rid of the lifetime thingie...
    pub(crate) fn feature_by_id(&'feature self, fid: &u64) -> Option<Feature> {
        self.layer.feature(*fid).map(Feature::from)
    }

    pub(crate) fn try_feature_by_id(&'feature self, fid: &u64) -> Result<Feature,CommandError> {
        self.layer.feature(*fid).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME,*fid)).map(Feature::from)
    }


    pub(crate) fn update_feature(&self, feature: Feature) -> Result<(),CommandError> {
        Ok(self.layer.set_feature(feature.into_feature())?)
    }

    pub(crate) fn feature_count(&self) -> usize {
        self.layer.feature_count() as usize
    }

    pub(crate) fn read_geometries(&mut self) -> LayerGeometryIterator {
        LayerGeometryIterator::new(&mut self.layer)
    }

    fn add_feature(&mut self, geometry: Geometry, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<u64,CommandError> {
        // I dug out the source to get this. I wanted to be able to return the feature being created.
        let mut feature = gdal::vector::Feature::new(self.layer.defn())?;
        feature.set_geometry(geometry)?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            if let Some(value) = value {
                feature.set_field(&field, value)?;
            } else {
                feature.set_field_null(&field)?;
            }
        }
        feature.create(&self.layer)?;
        Ok(feature.fid().unwrap())
    }


    fn add_feature_without_geometry(&mut self, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<u64,CommandError> {
        // This function is used for lookup tables, like biomes.

        // I had to dig into the source to get this stuff...
        let feature = gdal::vector::Feature::new(self.layer.defn())?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            if let Some(value) = value {
                feature.set_field(&field, value)?;
            } else {
                feature.set_field_null(&field)?;
            }
        }
        feature.create(&self.layer)?;
        Ok(feature.fid().unwrap())

    }

}

feature!(PointFeature PointSchema "points" wkbPoint to_field_names_values: #[allow(dead_code)] {});
type PointsLayer<'layer,'feature> = MapLayer<'layer,'feature,PointSchema,PointFeature<'feature>>;


impl PointsLayer<'_,'_> {

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.add_feature(point,&[],&[])?;
        Ok(())
    
    }

}

feature!(TriangleFeature TriangleSchema "triangles" wkbPolygon to_field_names_values: #[allow(dead_code)] {});
type TrianglesLayer<'layer,'feature> = MapLayer<'layer,'feature,TriangleSchema,TriangleFeature<'feature>>;



impl TrianglesLayer<'_,'_> {

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.add_feature(geo,&[],&[])?;
        Ok(())

    }


}

#[derive(Clone,PartialEq,Serialize,Deserialize)]
pub(crate) enum Grouping {
    LakeIsland,
    Islet,
    Island,
    Continent,
    Lake,
    Ocean
}

impl Grouping {

    pub(crate) fn is_ocean(&self) -> bool {
        matches!(self,Grouping::Ocean)
    }

    pub(crate) fn is_water(&self) -> bool {
        matches!(self,Grouping::Ocean | Grouping::Lake)
    }


}


impl Into<String> for &Grouping {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // it's a basic enum, there shouldn't be any reason why this would fail
    }
}


impl TryFrom<String> for Grouping {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidValueForGroupingType(value))
    }
}

feature!(TileFeature TileSchema "tiles" wkbPolygon to_field_names_values: #[allow(dead_code)] {
    /// longitude of the node point for the tile's voronoi
    site_x #[allow(dead_code)] f64 FIELD_SITE_X "site_x";
    /// latitude of the node point for the tile's voronoi
    site_y #[allow(dead_code)] f64 FIELD_SITE_Y "site_y";
    /// elevation in meters of the node point for the tile's voronoi
    elevation f64 FIELD_ELEVATION "elevation";
    // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
    // If I ever get rid of those algorithms, this field can go away.
    /// elevation scaled into a value from 0 to 100, where 20 is sea-level.
    elevation_scaled i32 FIELD_ELEVATION_SCALED "elevation_scaled";
    /// Indicates whether the tile is part of the ocean, an island, a continent, a lake, and maybe others.
    grouping grouping FIELD_GROUPING "grouping";
    /// A unique id for each grouping. These id's do not map to other tables, but will tell when tiles are in the same group. Use lake_id to link to the lake table.
    // NOTE: This isn't an id_ref, but let's store it that way anyway
    grouping_id id_ref FIELD_GROUPING_ID "grouping_id";
    /// average annual temperature of tile in imaginary units
    temperature f64 FIELD_TEMPERATURE "temperature";
    /// roughly estimated average wind direction for tile
    wind i32 FIELD_WIND "wind_dir";
    /// average annual precipitation of tile in imaginary units
    precipitation f64 FIELD_PRECIPITATION "precipitation";
    /// amount of water flow through tile in imaginary units
    water_flow f64 FIELD_WATER_FLOW "water_flow";
    /// amount of water accumulating (because it couldn't flow on) in imaginary units
    water_accumulation f64 FIELD_WATER_ACCUMULATION "water_accum";
    /// if the tile is in a lake, this is the id of the lake in the lakes layer
    lake_id option_id_ref FIELD_LAKE_ID "lake_id";
    /// id of neighboring tile which water flows to
    flow_to id_list FIELD_FLOW_TO "flow_to";
    /// shortest distance in number of tiles to an ocean or lake shoreline. This will be positive on land and negative inside a water body.
    shore_distance i32 FIELD_SHORE_DISTANCE "shore_distance";
    /// If this is a land tile neighboring a water body, this is the id of the closest tile
    harbor_tile_id option_id_ref FIELD_HARBOR_TILE_ID "harbor_tile_id";
    /// if this is a land tile neighboring a water body, this is the number of neighbor tiles that are water
    water_count option_i32 FIELD_WATER_COUNT "water_count";
    /// The biome for this tile
    biome string FIELD_BIOME "biome";
    /// the factor used to generate population numbers, along with the area of the tile
    habitability f64 FIELD_HABITABILITY "habitability";
    /// base population of the cell outside of the towns.
    population i32 FIELD_POPULATION "population";
    /// The name of the culture assigned to this tile, unless wild
    culture option_string FIELD_CULTURE "culture";
    /// if the tile has a town, this is the id of the town in the towns layer
    town_id option_id_ref FIELD_TOWN_ID "town_id"; 
    /// if the tile is part of a nation, this is the id of the nation which controls it
    nation_id option_id_ref FIELD_NATION_ID "nation_id";
    /// if the tile is part of a subnation, this is the id of the nation which controls it
    subnation_id option_id_ref FIELD_SUBNATION_ID "subnation_id";
    // NOTE: This field should only ever have one value or none. However, as I have no way of setting None
    // on a u64 field (until gdal is updated to give me access to FieldSetNone), I'm going to use a vector
    // to store it. In any way, you never know when I might support outlet from multiple points.
    /// If this tile is an outlet from a lake, this is the tile ID from which the water is flowing.
    outlet_from id_list FIELD_OUTLET_FROM "outlet_from";
    /// A list of all tile neighbors and their angular directions (tile_id:direction)
    neighbors neighbor_directions FIELD_NEIGHBOR_TILES "neighbor_tiles";

});


impl TileFeature<'_> {

    pub(crate) fn site(&self) -> Result<Point,CommandError> {
        Ok(Point::try_from((self.site_x()?,self.site_y()?))?)
    }

}

pub(crate) trait TileWithNeighbors: Entity<TileSchema> {

    fn neighbors(&self) -> &Vec<(u64,i32)>;

}

pub(crate) trait TileWithElevation: Entity<TileSchema> {

    fn elevation(&self) -> &f64;

}

pub(crate) trait TileWithGeometry: Entity<TileSchema> {
    fn geometry(&self) -> &Geometry;
}

pub(crate) trait TileWithShoreDistance: Entity<TileSchema> {
    fn shore_distance(&self) -> &i32;
}

pub(crate) trait TileWithNeighborsElevation: TileWithNeighbors + TileWithElevation {

}

impl<T: TileWithNeighbors + TileWithElevation> TileWithNeighborsElevation for T {

}


entity!(NewTile TileSchema TileFeature {
    geometry: Geometry,
    site_x: f64, 
    site_y: f64
}); 

entity!(TileForCalcNeighbors TileSchema TileFeature {
    geometry: Geometry,
    site: Point,
    neighbor_set: HashSet<u64> = |_| Ok::<_,CommandError>(HashSet::new())
});

entity!(TileForTerrain TileSchema TileFeature {
    site: Point, 
    elevation: f64,
    grouping: Grouping, 
    neighbors: Vec<(u64,i32)>,
    // 'old' values so the algorithm can check if it's changed.
    old_elevation: f64 = |feature: &TileFeature| feature.elevation(),
    old_grouping: Grouping = |feature: &TileFeature| feature.grouping()
});

impl TileForTerrain {

    pub(crate) fn elevation_changed(&self) -> bool {
        self.elevation != self.old_elevation
    }

    pub(crate) fn grouping_changed(&self) -> bool {
        self.grouping != self.old_grouping
    }
}

entity!(TileForTemperatures TileSchema TileFeature {
    fid: u64, 
    site_y: f64, 
    elevation: f64, 
    grouping: Grouping
});

entity!(TileForWinds TileSchema TileFeature {
    fid: u64, 
    site_y: f64
});

entity!(TileForWaterflow TileSchema TileFeature {
    elevation: f64, 
    grouping: Grouping, 
    neighbors: Vec<(u64,i32)>,
    precipitation: f64,
    temperature: f64,
    water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
    water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    flow_to: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new())
});

impl TileWithNeighbors for TileForWaterflow {

    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }

}

impl TileWithElevation for TileForWaterflow {

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}

// Basically the same struct as WaterFlow, except that the fields are initialized differently. I can't
// just use a different function because it's based on a trait. I could take this one out
// of the macro and figure something out, but this is easier.
entity!(TileForWaterFill TileSchema TileFeature {
    elevation: f64, 
    grouping: Grouping, 
    neighbors: Vec<(u64,i32)>,
    water_flow: f64,
    water_accumulation: f64,
    flow_to: Vec<u64>,
    temperature: f64,
    outlet_from: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new()),
    lake_id: Option<u64> = |_| Ok::<_,CommandError>(None)
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
            outlet_from: Vec::new(),
            lake_id: None
        }
    }
}

impl TileWithNeighbors for TileForWaterFill {
    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }


}

impl TileWithElevation for TileForWaterFill {

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}


entity!(TileForRiverConnect TileSchema TileFeature {
    water_flow: f64,
    flow_to: Vec<u64>,
    outlet_from: Vec<u64>
});

entity!(TileForWaterDistance TileSchema TileFeature {
    site: Point,
    grouping: Grouping, 
    neighbors: Vec<(u64,i32)>,
    water_count: Option<i32> = |_| Ok::<_,CommandError>(None),
    closest_water_tile_id: Option<u64> = |_| Ok::<_,CommandError>(None)
});


entity!(TileForGroupingCalc TileSchema TileFeature {
    fid: u64,
    grouping: Grouping,
    lake_id: Option<u64>,
    neighbors: Vec<(u64,i32)>
});

entity!(TileForPopulation TileSchema TileFeature {
    water_flow: f64,
    elevation_scaled: i32,
    biome: String,
    shore_distance: i32,
    water_count: Option<i32>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
    harbor_tile_id: Option<u64>,
    lake_id: Option<u64>
});

entity!(TileForPopulationNeighbor TileSchema TileFeature {
    grouping: Grouping,
    lake_id: Option<u64>
});



entity!(TileForCultureGen TileSchema TileFeature {
    fid: u64,
    site: Point,
    population: i32,
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    water_count: Option<i32>,
    harbor_tile_id: Option<u64>,
    grouping: Grouping,
    water_flow: f64,
    temperature: f64

});

pub(crate) struct TileForCulturePrefSorting<'struct_life> { // NOT an entity because we add in data from other layers.
    pub(crate) fid: u64,
    pub(crate) site: Point,
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

    pub(crate) fn from<'biomes>(tile: TileForCultureGen, tiles: &TilesLayer, biomes: &'biomes EntityLookup<BiomeSchema,BiomeForCultureGen>, lakes: &EntityIndex<LakeSchema,LakeForCultureGen>) -> Result<TileForCulturePrefSorting<'biomes>,CommandError> {
        let biome = biomes.try_get(&tile.biome)?;
        let neighboring_lake_size = if let Some(closest_water) = tile.harbor_tile_id {
            let closest_water = closest_water as u64;
            let closest_water = tiles.try_feature_by_id(&closest_water)?;
            if let Some(lake_id) = closest_water.lake_id()? {
                let lake_id = lake_id as u64;
                let lake = lakes.try_get(&lake_id)?;
                Some(lake.size)
            } else {
                None
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


entity!(TileForCultureExpand TileSchema TileFeature {
    population: i32,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<(u64,i32)>,
    lake_id: Option<u64>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry()?.area())
    },
    culture: Option<String> = |_| Ok::<_,CommandError>(None)

});

entity!(TileForTowns TileSchema TileFeature {
    fid: u64,
    habitability: f64,
    site: Point,
    culture: Option<String>,
    grouping_id: u64
});

entity!(TileForTownPopulation TileSchema TileFeature {
    fid: u64,
    geometry: Geometry,
    habitability: f64,
    site: Point,
    grouping_id: u64,
    harbor_tile_id: Option<u64>,
    water_count: Option<i32>,
    temperature: f64,
    lake_id: Option<u64>,
    water_flow: f64,
    grouping: Grouping
});

impl TileForTownPopulation {

    pub(crate) fn find_middle_point_between(&self, other: &Self) -> Result<Point,CommandError> {
        let self_ring = self.geometry.get_geometry(0);
        let other_ring = other.geometry.get_geometry(0);
        let other_vertices = other_ring.get_point_vec();
        let mut common_vertices = self_ring.get_point_vec();
        common_vertices.truncate(common_vertices.len() - 1); // remove the last point, which matches the first
        common_vertices.retain(|p| other_vertices.contains(p));
        if common_vertices.len() == 2 {
            let point1: Point = (common_vertices[0].0,common_vertices[0].1).try_into()?;
            let point2 = (common_vertices[1].0,common_vertices[1].1).try_into()?;
            Ok(point1.middle_point_between(&point2))
        } else {
            Err(CommandError::CantFindMiddlePoint(self.fid,other.fid,common_vertices.len()))
        }

    }
}

entity!(TileForNationExpand TileSchema TileFeature {
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<(u64,i32)>,
    lake_id: Option<u64>,
    culture: Option<String>,
    nation_id: Option<u64> = |_| Ok::<_,CommandError>(None)
});

entity!(TileForNationNormalize TileSchema TileFeature {
    grouping: Grouping,
    neighbors: Vec<(u64,i32)>,
    town_id: Option<u64>,
    nation_id: Option<u64>
});

entity!(TileForSubnations TileSchema TileFeature {
    fid: u64,
    town_id: Option<u64>,
    nation_id: Option<u64>,
    culture: Option<String>,
    population: i32
});

entity!(TileForSubnationExpand TileSchema TileFeature {
    neighbors: Vec<(u64,i32)>,
    grouping: Grouping,
    shore_distance: i32,
    elevation_scaled: i32,
    nation_id: Option<u64>,
    subnation_id: Option<u64> = |_| Ok::<_,CommandError>(None)
});

entity!(TileForEmptySubnations TileSchema TileFeature {
    neighbors: Vec<(u64,i32)>,
    shore_distance: i32,
    nation_id: Option<u64>,
    subnation_id: Option<u64>,
    grouping: Grouping,
    town_id: Option<u64>,
    population: i32,
    culture: Option<String>
});

entity!(TileForSubnationNormalize TileSchema TileFeature {
    neighbors: Vec<(u64,i32)>,
    town_id: Option<u64>,
    nation_id: Option<u64>,
    subnation_id: Option<u64>
});

entity!(TileForCultureDissolve TileSchema TileFeature {
    culture: Option<String>,
    geometry: Geometry,
    neighbors: Vec<(u64,i32)>,
    shore_distance: i32
});

impl TileWithGeometry for TileForCultureDissolve {
    fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForCultureDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForCultureDissolve {
    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }
}

entity!(TileForBiomeDissolve TileSchema TileFeature {
    biome: String,
    geometry: Geometry,
    neighbors: Vec<(u64,i32)>,
    shore_distance: i32
});

impl TileWithGeometry for TileForBiomeDissolve {
    fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForBiomeDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForBiomeDissolve {
    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }
}

entity!(TileForNationDissolve TileSchema TileFeature {
    nation_id: Option<u64>,
    geometry: Geometry,
    neighbors: Vec<(u64,i32)>,
    shore_distance: i32
});

impl TileWithGeometry for TileForNationDissolve {
    fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForNationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForNationDissolve {
    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }
}

entity!(TileForSubnationDissolve TileSchema TileFeature {
    subnation_id: Option<u64>,
    geometry: Geometry,
    neighbors: Vec<(u64,i32)>,
    shore_distance: i32
});

impl TileWithGeometry for TileForSubnationDissolve {
    fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForSubnationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForSubnationDissolve {
    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }
}



pub(crate) type TilesLayer<'layer,'feature> = MapLayer<'layer,'feature,TileSchema,TileFeature<'feature>>;

impl TilesLayer<'_,'_> {


    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TileSchema,TileFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn try_entity_by_id<'this, Data: Entity<TileSchema> + TryFrom<TileFeature<'this>,Error=CommandError>>(&'this mut self, fid: &u64) -> Result<Data,CommandError> {
        self.try_feature_by_id(&fid)?.try_into()
    }

    pub(crate) fn add_tile(&mut self, tile: NewTile) -> Result<(),CommandError> {

        self.add_feature(tile.geometry,&[
                TileSchema::FIELD_SITE_X,
                TileSchema::FIELD_SITE_Y,
                TileSchema::FIELD_ELEVATION,
                TileSchema::FIELD_ELEVATION_SCALED,
                TileSchema::FIELD_GROUPING,
            ],&[
                feature_to_value!(tile.site_x; f64),
                feature_to_value!(tile.site_y; f64),
                // initial tiles start with 0 elevation, terrain commands will edit this...
                feature_to_value!(0.0; f64), // FUTURE: Watch that this type stays correct
                // and scaled elevation starts with 20.
                feature_to_value!(20; i32), // FUTURE: Watch that this type stays correct
                // tiles are continent by default until someone samples some ocean.
                feature_to_value!(&Grouping::Continent; grouping)
            ])?;
        Ok(())

    }

    pub(crate) fn get_layer_size(&self) -> Result<(f64,f64),CommandError> {
        let extent = self.layer.get_extent()?;
        let width = extent.MaxX - extent.MinX;
        let height = extent.MaxY - extent.MinY;
        Ok((width,height))
    }

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
    pub(crate) fn get_index_and_queue_for_water_fill<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(EntityIndex<TileSchema,TileForWaterFill>,Vec<(u64,f64)>),CommandError> {

        let mut tile_queue = Vec::new();

        let tile_map = self.read_features().to_entities_index_for_each::<_,TileForWaterFill,_>(|fid,tile| {
            if tile.water_accumulation > 0.0 {
                tile_queue.push((*fid,tile.water_accumulation));
            }

            Ok(())
        },progress)?;


        Ok((tile_map,tile_queue))
        

    }


}


#[derive(Clone,Serialize,Deserialize)]
pub(crate) enum RiverSegmentFrom {
    Source,
    Lake,
    Branch,
    Continuing,
    BranchingLake,
    BranchingConfluence,
    Confluence,
}

impl TryFrom<String> for RiverSegmentFrom {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidValueForSegmentFrom(value))
    }
}

impl Into<String> for &RiverSegmentFrom {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // there shouldn't be any reason to have an error
    }
}

#[derive(Clone,Serialize,Deserialize)]
pub(crate) enum RiverSegmentTo {
    Mouth,
    Confluence,
    Continuing,
    Branch,
    BranchingConfluence,
}

impl TryFrom<String> for RiverSegmentTo {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidValueForSegmentTo(value))
    }
}

impl Into<String> for &RiverSegmentTo {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // there shouldn't be any reason to have an error
    }
}


feature!(RiverFeature RiverSchema "rivers" wkbLineString {
    from_tile_id #[allow(dead_code)] id_ref FIELD_FROM_TILE_ID "from_tile_id";
    from_type #[allow(dead_code)] river_segment_from FIELD_FROM_TYPE "from_type";
    from_flow #[allow(dead_code)] f64 FIELD_FROM_FLOW "from_flow";
    to_tile_id #[allow(dead_code)] id_ref FIELD_TO_TILE_ID "to_tile_id";
    to_type #[allow(dead_code)] river_segment_to FIELD_TO_TYPE "to_type";
    to_flow #[allow(dead_code)] f64 FIELD_TO_FLOW "to_flow";
});


entity!(NewRiver RiverSchema RiverFeature {
    from_tile_id: u64,
    from_type: RiverSegmentFrom,
    from_flow: f64,
    to_tile_id: u64,
    to_type: RiverSegmentTo,
    to_flow: f64,
    line: Vec<Point> = |_| Ok::<_,CommandError>(Vec::new())
});

pub(crate) type RiversLayer<'layer,'feature> = MapLayer<'layer,'feature,RiverSchema,RiverFeature<'feature>>;

impl RiversLayer<'_,'_> {

    pub(crate) fn add_segment(&mut self, segment: &NewRiver) -> Result<u64,CommandError> {
        let geometry = create_line(&segment.line)?;
        let (field_names,field_values) = RiverFeature::to_field_names_values(
            segment.from_tile_id, 
            &segment.from_type, 
            segment.from_flow, 
            segment.to_tile_id, 
            &segment.to_type,
            segment.to_flow)?;
        self.add_feature(geometry, &field_names, &field_values)
    }

}

#[derive(Clone,Serialize,Deserialize)]
pub(crate) enum LakeType {
    Fresh,
    Salt,
    Frozen,
    Pluvial, // lake forms intermittently, it's also salty
    Dry,
    Marsh,
}


impl Into<String> for &LakeType {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // there shouldn't be any reason to have an error
    }
}

impl TryFrom<String> for LakeType {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidValueForLakeType(value))
    }
}

feature!(LakeFeature LakeSchema "lakes" wkbMultiPolygon {
    #[allow(dead_code)] elevation #[allow(dead_code)] f64 FIELD_ELEVATION "elevation";
    type_ #[allow(dead_code)] lake_type FIELD_TYPE "type";
    #[allow(dead_code)] flow #[allow(dead_code)] f64 FIELD_FLOW "flow";
    size #[allow(dead_code)] i32 FIELD_SIZE "size";
    #[allow(dead_code)] temperature #[allow(dead_code)] f64 FIELD_TEMPERATURE "temperature";
    #[allow(dead_code)] evaporation #[allow(dead_code)] f64 FIELD_EVAPORATION "evaporation";
});

entity!(LakeForBiomes LakeSchema LakeFeature {
    type_: LakeType
});

entity!(LakeForPopulation LakeSchema LakeFeature {
    type_: LakeType
});

entity!(LakeForCultureGen LakeSchema LakeFeature {
    size: i32
});

entity!(LakeForTownPopulation LakeSchema LakeFeature {
    size: i32
});



#[derive(Clone)]
pub(crate) struct NewLake {
    pub(crate) elevation: f64,
    pub(crate) type_: LakeType,
    pub(crate) flow: f64,
    pub(crate) size: i32,
    pub(crate) temperature: f64,
    pub(crate) evaporation: f64,
    pub(crate) geometry: Geometry,
}


pub(crate) type LakesLayer<'layer,'feature> = MapLayer<'layer,'feature,LakeSchema,LakeFeature<'feature>>;

impl LakesLayer<'_,'_> {

    pub(crate) fn add_lake(&mut self, lake: NewLake) -> Result<u64,CommandError> {
        let (field_names,field_values) = LakeFeature::to_field_names_values(
            lake.elevation,
            &lake.type_,
            lake.flow,
            lake.size,
            lake.temperature,
            lake.evaporation
        )?;
        self.add_feature(lake.geometry, &field_names, &field_values)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<LakeSchema,LakeFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }



}

#[derive(Clone,Serialize,Deserialize)]
pub(crate) enum BiomeCriteria {
    Matrix(Vec<(usize,usize)>), // moisture band, temperature band
    Wetland,
    Glacier,
    Ocean
}

impl TryFrom<String> for BiomeCriteria {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidBiomeMatrixValue(value))
    }
}

impl Into<String> for &BiomeCriteria {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // there shouldn't be any reason to have an error
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

feature!(BiomeFeature BiomeSchema "biomes" wkbMultiPolygon {
    name #[allow(dead_code)] string FIELD_NAME "name";
    habitability #[allow(dead_code)] i32 FIELD_HABITABILITY "habitability";
    criteria #[allow(dead_code)] biome_criteria FIELD_CRITERIA "criteria";
    movement_cost #[allow(dead_code)] i32 FIELD_MOVEMENT_COST "movement_cost";
    supports_nomadic #[allow(dead_code)] bool FIELD_NOMADIC "supp_nomadic";
    supports_hunting #[allow(dead_code)] bool FIELD_HUNTING "supp_hunting";
    color #[allow(dead_code)] string FIELD_COLOR "color";
});

impl<'feature> NamedFeature<'feature,BiomeSchema> for BiomeFeature<'feature> {
    fn name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

impl BiomeFeature<'_> {

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
        for (moisture,row) in Self::DEFAULT_MATRIX.iter().enumerate() {
            for (temperature,id) in row.iter().enumerate() {
                match matrix_criteria.entry(id) {
                    Vacant(entry) => {
                        entry.insert(vec![(moisture,temperature)]);
                    },
                    Occupied(mut entry) => entry.get_mut().push((moisture,temperature)),
                }
            }

        }

        Self::DEFAULT_BIOMES.iter().map(|default| {
            let criteria = if let BiomeCriteria::Matrix(_) = default.criteria {
                BiomeCriteria::Matrix(matrix_criteria.get(&default.name).unwrap().clone())
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
                color: default.color.to_owned()
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
                        if matrix[moist][temp] != "" {
                            Err(CommandError::DuplicateBiomeMatrixSlot(moist,temp))?
                        } else {
                            matrix[moist][temp] = biome.name.clone()

                        }
                    }
                },
                BiomeCriteria::Wetland => if wetland.is_some() {
                    Err(CommandError::DuplicateWetlandBiome)?
                } else {
                    wetland = Some(biome.name.clone())
                },
                BiomeCriteria::Glacier => if glacier.is_some() {
                    Err(CommandError::DuplicateGlacierBiome)?
                } else {
                    glacier = Some(biome.name.clone())
                },
                BiomeCriteria::Ocean => if ocean.is_some() {
                    Err(CommandError::DuplicateOceanBiome)?
                } else {
                    ocean = Some(biome.name.clone())
                }
            }

        }
        let wetland = wetland.ok_or_else(|| CommandError::MissingWetlandBiome)?;
        let glacier = glacier.ok_or_else(|| CommandError::MissingGlacierBiome)?;
        let ocean = ocean.ok_or_else(|| CommandError::MissingOceanBiome)?;
        for moisture in 0..matrix.len() {
            for temperature in 0..matrix[moisture].len() {
                if matrix[moisture][temperature] == "" {
                    return Err(CommandError::MissingBiomeMatrixSlot(moisture,temperature))
                }
            }
        }
        Ok(BiomeMatrix {
            matrix,
            glacier,
            ocean,
            wetland,
        })
    }

}

entity!(NewBiome BiomeSchema BiomeFeature {
    name: String,
    habitability: i32,
    criteria: BiomeCriteria,
    movement_cost: i32,
    supports_nomadic: bool,
    supports_hunting: bool,
    color: String
});

entity!(BiomeForPopulation BiomeSchema BiomeFeature {
    name: String,
    habitability: i32
});

impl NamedEntity<BiomeSchema> for BiomeForPopulation {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureGen BiomeSchema BiomeFeature {
    name: String,
    supports_nomadic: bool,
    supports_hunting: bool
});

impl NamedEntity<BiomeSchema> for BiomeForCultureGen {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureExpand BiomeSchema BiomeFeature {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForCultureExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForNationExpand BiomeSchema BiomeFeature {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForNationExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForDissolve BiomeSchema BiomeFeature {
    fid: u64,
    name: String
});

impl NamedEntity<BiomeSchema> for BiomeForDissolve {
    fn name(&self) -> &str {
        &self.name
    }
}


pub(crate) type BiomeLayer<'layer,'feature> = MapLayer<'layer,'feature,BiomeSchema,BiomeFeature<'feature>>;

impl BiomeLayer<'_,'_> {

    pub(crate) fn add_biome(&mut self, biome: &NewBiome) -> Result<u64,CommandError> {

        let (field_names,field_values) = BiomeFeature::to_field_names_values(
            &biome.name,biome.habitability,&biome.criteria,biome.movement_cost,biome.supports_nomadic,biome.supports_hunting,&biome.color)?;
        self.add_feature_without_geometry(&field_names, &field_values)

    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<BiomeSchema,BiomeFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    pub(crate) fn get_matrix<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<BiomeMatrix,CommandError> {
        let result = self.read_features().to_entities_vec(progress)?;
    
        BiomeFeature::build_matrix_from_biomes(&result)
    
    }

    pub(crate) fn build_lookup<'local, Progress: ProgressObserver, Data: NamedEntity<BiomeSchema> + TryFrom<BiomeFeature<'local>,Error=CommandError>>(&'local mut self, progress: &mut Progress) -> Result<EntityLookup<BiomeSchema, Data>,CommandError> {
        let mut result = HashMap::new();

        for entity in self.read_features().into_entities::<Data>().watch(progress,"Indexing biomes.","Biomes indexed.") {
            let (_,entity) = entity?;
            let name = entity.name().to_owned();
            result.insert(name, entity);
        }

        Ok(EntityLookup::from(result))

    }


}

#[derive(Clone,Hash,Eq,PartialEq,Serialize,Deserialize)]
pub(crate) enum CultureType {
    Generic,
    Lake,
    Naval,
    River,
    Nomadic,
    Hunting,
    Highland
}


impl Into<String> for &CultureType {

    fn into(self) -> String {
        to_ron_string(self).unwrap() // there shouldn't be any reason to have an error
    }
}


impl TryFrom<String> for CultureType {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        from_ron_str(&value).map_err(|_| CommandError::InvalidValueForCultureType(value))
    }
}

feature!(CultureFeature CultureSchema "cultures" wkbMultiPolygon {
    name #[allow(dead_code)] string FIELD_NAME "name";
    namer #[allow(dead_code)] string FIELD_NAMER "namer";
    type_ #[allow(dead_code)] culture_type FIELD_TYPE "type";
    expansionism #[allow(dead_code)] f64 FIELD_EXPANSIONISM "expansionism";
    center_tile_id #[allow(dead_code)] id_ref FIELD_CENTER_TILE_ID "center_tile_id";
    color #[allow(dead_code)] string FIELD_COLOR "color";
});

impl<'feature> NamedFeature<'feature,CultureSchema> for CultureFeature<'feature> {
    fn name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

impl CultureSchema {

    pub(crate) fn get_lookup_and_namers<CultureEntity, Progress>(namers: NamerSet, default_namer: String, target: &WorldMap, progress: &mut Progress) -> Result<(EntityLookup<CultureSchema, CultureEntity>, LoadedNamers), CommandError> 
    where 
    CultureEntity: NamedEntity<CultureSchema> + for<'feature> TryFrom<CultureFeature<'feature>,Error=CommandError> + CultureWithNamer, Progress: ProgressObserver {
        progress.announce("Preparing namers");
        let mut cultures_layer = target.cultures_layer()?;
        let (culture_lookup,loaded_namers) = {
            let mut result = HashMap::new();
            let mut load_namers = HashSet::new();

            for entity in cultures_layer.read_features().into_entities::<CultureEntity>().watch(progress,"Indexing biomes.","Biomes indexed.") {
                let (_,entity) = entity?;
                let name = entity.name().to_owned();
                load_namers.insert(entity.namer().to_owned());
                result.insert(name, entity);
            }

            let loaded_namers = namers.into_loaded(load_namers, default_namer, progress)?;

            (EntityLookup::from(result),loaded_namers)
        };
        Ok((culture_lookup, loaded_namers))
    }

}

pub(crate) trait CultureWithNamer {

    fn namer(&self) -> &str;

    fn get_namer<'namers, Culture: CultureWithNamer>(culture: Option<&Culture>, namers: &'namers mut LoadedNamers) -> Result<&'namers mut Namer, CommandError> {
        let namer = namers.get_mut(culture.map(|culture| culture.namer()))?;
        Ok(namer)
    }
    
}

pub(crate) trait CultureWithType {

    fn type_(&self) -> &CultureType;
}


entity!(NewCulture CultureSchema CultureFeature {
    name: String,
    namer: String,
    type_: CultureType,
    expansionism: f64,
    center_tile_id: u64,
    color: String
});

// needs to be hashable in order to fit into a priority queue
entity!(#[derive(Hash,Eq,PartialEq)] CultureForPlacement CultureSchema CultureFeature {
    name: String,
    center_tile_id: u64,
    type_: CultureType,
    expansionism: OrderedFloat<f64> = |feature: &CultureFeature| Ok::<_,CommandError>(OrderedFloat::from(feature.expansionism()?))
});

entity!(CultureForTowns CultureSchema CultureFeature {
    name: String,
    namer: String
});

impl<'impl_life> NamedEntity<CultureSchema> for CultureForTowns {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<'impl_life> CultureWithNamer for CultureForTowns {
    fn namer(&self) -> &str {
        &self.namer
    }
}

entity!(CultureForNations CultureSchema CultureFeature {
    name: String,
    namer: String,
    type_: CultureType
});

impl<'impl_life> NamedEntity<CultureSchema> for CultureForNations {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<'impl_life> CultureWithNamer for CultureForNations {
    fn namer(&self) -> &str {
        &self.namer
    }
}

impl<'impl_life> CultureWithType for CultureForNations {
    fn type_(&self) -> &CultureType {
        &self.type_
    }
}


entity!(CultureForDissolve CultureSchema CultureFeature {
    fid: u64,
    name: String
});

impl<'impl_life> NamedEntity<CultureSchema> for CultureForDissolve {
    fn name(&self) -> &str {
        &self.name
    }
}





pub(crate) type CultureLayer<'layer,'feature> = MapLayer<'layer,'feature,CultureSchema,CultureFeature<'feature>>;


impl CultureLayer<'_,'_> {

    pub(crate) fn add_culture(&mut self, culture: &NewCulture) -> Result<u64,CommandError> {

        let (field_names,field_values) = CultureFeature::to_field_names_values(
            &culture.name,&culture.namer,&culture.type_,culture.expansionism,culture.center_tile_id,&culture.color)?;
        self.add_feature_without_geometry(&field_names, &field_values)

    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<CultureSchema,CultureFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }


}

feature!(TownFeature TownSchema "towns" wkbPoint {
    name #[allow(dead_code)] string FIELD_NAME "name";
    culture #[allow(dead_code)] option_string FIELD_CULTURE "culture";
    is_capital #[allow(dead_code)] bool FIELD_IS_CAPITAL "is_capital";
    tile_id #[allow(dead_code)] id_ref FIELD_TILE_ID "tile_id";
    grouping_id #[allow(dead_code)] id_ref FIELD_GROUPING_ID "grouping_id"; 
    #[allow(dead_code)] population i32 FIELD_POPULATION "population";
    #[allow(dead_code)] is_port bool FIELD_IS_PORT "is_port";
});

impl TownFeature<'_> {

    pub(crate) fn move_to(&mut self, new_location: Point) -> Result<(),CommandError> {
        Ok(self.feature.set_geometry(new_location.create_geometry()?)?)
    }

}

entity!(NewTown TownSchema TownFeature {
    geometry: Geometry,
    name: String,
    culture: Option<String>,
    is_capital: bool,
    tile_id: u64,
    grouping_id: u64
});

entity!(TownForPopulation TownSchema TownFeature {
    fid: u64,
    is_capital: bool,
    tile_id: u64
});

entity!(TownForNations TownSchema TownFeature {
    fid: u64,
    is_capital: bool,
    culture: Option<String>,
    tile_id: u64
});

entity!(TownForNationNormalize TownSchema TownFeature {
    is_capital: bool
});

entity!(TownForSubnations TownSchema TownFeature {
    name: String
});

entity!(TownForEmptySubnations TownSchema TownFeature {
    name: String
});

pub(crate) type TownLayer<'layer,'feature> = MapLayer<'layer,'feature,TownSchema,TownFeature<'feature>>;

impl TownLayer<'_,'_> {

    pub(crate) fn add_town(&mut self, town: NewTown) -> Result<u64,CommandError> {
        let (field_names,field_values) = TownFeature::to_field_names_values(
            &town.name,
            town.culture.as_deref(),
            town.is_capital,
            town.tile_id,
            town.grouping_id,
            0,
            false
        )?;
        self.add_feature(town.geometry, &field_names, &field_values)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TownSchema,TownFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    
}


feature!(NationFeature NationSchema "nations" wkbMultiPolygon {
    name #[allow(dead_code)] string FIELD_NAME "name";
    culture #[allow(dead_code)] option_string FIELD_CULTURE "culture";
    center_tile_id #[allow(dead_code)] id_ref FIELD_CENTER_TILE_ID "center_tile_id"; 
    type_ #[allow(dead_code)] culture_type FIELD_TYPE "type";
    expansionism #[allow(dead_code)] f64 FIELD_EXPANSIONISM "expansionism";
    capital_town_id #[allow(dead_code)] id_ref FIELD_CAPITAL_TOWN_ID "capital_town_id";
    color #[allow(dead_code)] string FIELD_COLOR "color";
});

impl<'feature> NamedFeature<'feature,NationSchema> for NationFeature<'feature> {
    fn name(&self) -> Result<String,CommandError> {
        self.name()
    }
}


entity!(NewNation NationSchema NationFeature {
    name: String,
    culture: Option<String>,
    center_tile_id: u64,
    type_: CultureType,
    expansionism: f64,
    capital_town_id: u64,
    color: String
});

// needs to be hashable in order to fit into a priority queue
entity!(#[derive(Hash,Eq,PartialEq)] NationForPlacement NationSchema NationFeature {
    fid: u64,
    name: String,
    center_tile_id: u64,
    type_: CultureType,
    expansionism: OrderedFloat<f64> = |feature: &NationFeature| Ok::<_,CommandError>(OrderedFloat::from(feature.expansionism()?))
});

entity!(NationForSubnations NationSchema NationFeature {
    fid: u64,
    capital_town_id: u64,
    color: String
});

entity!(NationForEmptySubnations NationSchema NationFeature {
    fid: u64,
    color: String,
    culture: Option<String>
});


pub(crate) type NationsLayer<'layer,'feature> = MapLayer<'layer,'feature,NationSchema,NationFeature<'feature>>;

impl NationsLayer<'_,'_> {

    pub(crate) fn add_nation(&mut self, nation: NewNation) -> Result<u64,CommandError> {
        let (field_names,field_values) = NationFeature::to_field_names_values(
            &nation.name,
            nation.culture.as_deref(),
            nation.center_tile_id,
            &nation.type_,
            nation.expansionism,
            nation.capital_town_id,
            &nation.color
        )?;
        self.add_feature_without_geometry(&field_names, &field_values)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<NationSchema,NationFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    

}

feature!(SubnationFeature SubnationSchema "subnations" wkbMultiPolygon {
    name #[allow(dead_code)] string FIELD_NAME "name";
    culture #[allow(dead_code)] option_string FIELD_CULTURE "culture";
    center_tile_id #[allow(dead_code)] id_ref FIELD_CENTER_TILE_ID "center_tile_id";
    type_ #[allow(dead_code)] culture_type FIELD_TYPE "type";
    seat_town_id #[allow(dead_code)] option_id_ref FIELD_SEAT_TOWN_ID "seat_town_id"; 
    nation_id #[allow(dead_code)] id_ref FIELD_NATION_ID "nation_id"; 
    color #[allow(dead_code)] string FIELD_COLOR "color";
});

impl<'feature> NamedFeature<'feature,SubnationSchema> for SubnationFeature<'feature> {
    fn name(&self) -> Result<String,CommandError> {
        self.name()
    }
}


entity!(NewSubnation SubnationSchema SubnationFeature {
    name: String,
    culture: Option<String>,
    center_tile_id: u64,
    type_: CultureType,
    seat_town_id: Option<u64>,
    nation_id: u64,
    color: String
});


entity!(#[derive(Hash,Eq,PartialEq)] SubnationForPlacement SubnationSchema SubnationFeature {
    fid: u64,
    center_tile_id: u64,
    nation_id: u64
});

entity!(SubnationForNormalize SubnationSchema SubnationFeature {
    center_tile_id: u64,
    seat_town_id: Option<u64>
});


pub(crate) type SubnationsLayer<'layer,'feature> = MapLayer<'layer,'feature,SubnationSchema,SubnationFeature<'feature>>;

impl SubnationsLayer<'_,'_> {

    pub(crate) fn add_subnation(&mut self, subnation: NewSubnation) -> Result<u64,CommandError> {
        let (field_names,field_values) = SubnationFeature::to_field_names_values(
            &subnation.name,
            subnation.culture.as_deref(),
            subnation.center_tile_id,
            &subnation.type_,
            subnation.seat_town_id,
            subnation.nation_id,
            &subnation.color
        )?;
        self.add_feature_without_geometry(&field_names, &field_values)
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<SubnationSchema,SubnationFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }


}

feature!(CoastlineFeature CoastlineSchema "coastlines" wkbPolygon  {
});

pub(crate) type CoastlineLayer<'layer,'feature> = MapLayer<'layer,'feature,CoastlineSchema,CoastlineFeature<'feature>>;

impl CoastlineLayer<'_,'_> {

    pub(crate) fn add_land_mass(&mut self, geometry: Geometry) -> Result<u64, CommandError> {
        let (field_names,field_values) = CoastlineFeature::to_field_names_values()?;
        self.add_feature(geometry, &field_names, &field_values)
    }

}

feature!(OceanFeature OceanSchema "oceans" wkbPolygon {
});

pub(crate) type OceanLayer<'layer,'feature> = MapLayer<'layer,'feature,OceanSchema,OceanFeature<'feature>>;

impl OceanLayer<'_,'_> {

    pub(crate) fn add_ocean(&mut self, geometry: Geometry) -> Result<u64, CommandError> {
        let (field_names,field_values) = OceanFeature::to_field_names_values()?;
        self.add_feature(geometry, &field_names, &field_values)
    }

}

/*
// Uncomment this stuff if you need to add a line layer for playing around with something.
feature!(LineFeature LineSchema "lines" wkbLineString {
});

pub(crate) type LineLayer<'layer,'feature> = MapLayer<'layer,'feature,LineSchema,LineFeature<'feature>>;

impl LineLayer<'_,'_> {

     pub(crate) fn add_line(&mut self, line: &Vec<Point>) -> Result<u64,CommandError> {
        let geometry = crate::utils::create_line(line)?;
        self.add_feature(geometry, &[], &[])
    }
}
*/

feature!(PropertyFeature PropertySchema "properties" wkbNone {
    name #[allow(dead_code)] string FIELD_NAME "name";
    value string FIELD_VALUE "value";
});

pub(crate) type PropertyLayer<'layer,'feature> = MapLayer<'layer,'feature,PropertySchema,PropertyFeature<'feature>>;

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

impl Into<String> for &ElevationLimits {

    fn into(self) -> String {
        // store as tuple for simplicity
        to_ron_string(&(self.min_elevation,self.max_elevation)).unwrap() // there shouldn't be any reason to have an error
    }
}


impl TryFrom<String> for ElevationLimits {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // store as tuple for simplicity
        let input: (f64,f64) = from_ron_str(&value).map_err(|_| CommandError::InvalidPropertyValue(PropertySchema::PROP_ELEVATION_LIMITS.to_owned(),value.clone()))?;
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
                return Ok(feature.value()?)
            }
        }
        Err(CommandError::PropertyNotSet(name.to_owned()))

    }

    pub(crate) fn get_elevation_limits(&mut self) -> Result<ElevationLimits,CommandError> {
        self.get_property(PropertySchema::PROP_ELEVATION_LIMITS)?.try_into()
    }

    fn set_property(&mut self, name: &str, value: &str) -> Result<(),CommandError> {
        let mut found = None;
        for feature in TypedFeatureIterator::<PropertySchema,PropertyFeature>::from(self.layer.features()) {
            if feature.name()? == name {
                found = Some(feature.fid()?);
                break;
            }
        }
        if let Some(found) = found {
            let mut feature = self.try_feature_by_id(&found)?;
            feature.set_value(value)?;
            self.update_feature(feature)?;
        } else {
            let (fields,values) = PropertyFeature::to_field_names_values(name, value)?;
            self.add_feature_without_geometry(&fields,&values)?;
   
        }
        Ok(())
    }

    pub(crate) fn set_elevation_limits(&mut self, value: &ElevationLimits) -> Result<(),CommandError> {
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
            dataset,
            path
        }
    }

    fn open_dataset<FilePath: AsRef<Path>>(path: FilePath) -> Result<gdal::Dataset, CommandError> {
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

    pub(crate) fn points_layer(&self) -> Result<PointsLayer,CommandError> {
        PointsLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn triangles_layer(&self) -> Result<TrianglesLayer,CommandError> {
        TrianglesLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn tiles_layer(&self) -> Result<TilesLayer,CommandError> {
        TilesLayer::open_from_dataset(&self.dataset)
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

    pub(crate) fn create_points_layer(&mut self, overwrite: bool) -> Result<PointsLayer,CommandError> {
        Ok(PointsLayer::create_from_dataset(&mut self.dataset, overwrite)?)       

    }

    pub(crate) fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TrianglesLayer,CommandError> {
        Ok(TrianglesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn create_tile_layer(&mut self, overwrite: bool) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn create_rivers_layer(&mut self, overwrite: bool) -> Result<RiversLayer,CommandError> {
        Ok(RiversLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub (crate) fn create_lakes_layer(&mut self, overwrite_layer: bool) -> Result<LakesLayer,CommandError> {
        Ok(LakesLayer::create_from_dataset(&mut self.dataset, overwrite_layer)?)
    }

    pub (crate) fn edit_lakes_layer(&mut self) -> Result<LakesLayer,CommandError> {
        Ok(LakesLayer::open_from_dataset(&mut self.dataset)?)
    }

    pub(crate) fn edit_tile_layer(&mut self) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_biomes_layer(&mut self, overwrite: bool) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }

    pub(crate) fn edit_biomes_layer(&mut self) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_cultures_layer(&mut self, overwrite: bool) -> Result<CultureLayer,CommandError> {
        Ok(CultureLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }

    pub(crate) fn edit_cultures_layer(&mut self) -> Result<CultureLayer,CommandError> {
        Ok(CultureLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_towns_layer(&mut self, overwrite_layer: bool) -> Result<TownLayer,CommandError> {
        Ok(TownLayer::create_from_dataset(&mut self.dataset, overwrite_layer)?)
    }

    pub(crate) fn edit_towns_layer(&mut self) -> Result<TownLayer,CommandError> {
        Ok(TownLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_nations_layer(&mut self, overwrite_layer: bool) -> Result<NationsLayer,CommandError> {
        Ok(NationsLayer::create_from_dataset(&mut self.dataset, overwrite_layer)?)
    }

    pub(crate) fn edit_nations_layer(&mut self) -> Result<NationsLayer,CommandError> {
        Ok(NationsLayer::open_from_dataset(&mut self.dataset)?)
    }

    pub(crate) fn create_subnations_layer(&mut self, overwrite_layer: bool) -> Result<SubnationsLayer,CommandError> {
        Ok(SubnationsLayer::create_from_dataset(&mut self.dataset, overwrite_layer)?)
    }

    pub(crate) fn edit_subnations_layer(&mut self) -> Result<SubnationsLayer,CommandError> {
        Ok(SubnationsLayer::open_from_dataset(&mut self.dataset)?)
    }

    pub(crate) fn create_coastline_layer(&mut self, overwrite_coastline: bool) -> Result<CoastlineLayer,CommandError> {
        Ok(CoastlineLayer::create_from_dataset(&mut self.dataset, overwrite_coastline)?)
    }

    pub(crate) fn create_ocean_layer(&mut self, overwrite_ocean: bool) -> Result<OceanLayer,CommandError> {
        Ok(OceanLayer::create_from_dataset(&mut self.dataset, overwrite_ocean)?)
    }

    /* Uncomment this to add a line layer for playing around with ideas.
     pub(crate) fn create_lines_layer(&mut self, overwrite: bool) -> Result<LineLayer,CommandError> {
        Ok(LineLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }
    */

    pub(crate) fn create_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        Ok(PropertyLayer::create_from_dataset(&mut self.dataset,true)?)
    }

    pub(crate) fn edit_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        Ok(PropertyLayer::open_from_dataset(&mut self.dataset)?)
    }

}

