use std::path::Path;
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::Feature;
use gdal::vector::FeatureIterator;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::utils::LayerGeometryIterator;
use crate::utils::Point;
use crate::utils::create_line;
use crate::errors::MissingErrorToOption;
use crate::utils::ToTitleCase;


// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.

// FUTURE: Another problem with the gdal crate is the lifetimes. Feature, for example, only requires the lifetimes because it holds a reference to 
// a field definition pointer, which is never used except in the constructor. Once the feature is created, this reference could easily be forgotten. Layer is
// a little more complex, it holds a phantom value of the type of a reference to its dataset. On the one hand, it also doesn't do anything with it at all,
// on the other this reference might keep it from outliving it's dataset reference. Which, I guess, is the same with Feature, so maybe that's what they're 
// doing. I just wish there was another way, as it would make the TypedFeature stuff I'm trying to do below work better. However, if that were built into
// the gdal crate, maybe it would be better.

macro_rules! feature_conv {
    (id_list_to_string@ $value: ident) => {
        $value.iter().map(|fid| format!("{}",fid)).collect::<Vec<String>>().join(",")
    };
    (neighbor_directions_to_string@ $value: ident) => {
        $value.iter().map(|(fid,dir)| format!("{}:{}",fid,dir)).collect::<Vec<String>>().join(",")
    };
}

macro_rules! feature_get_field_type {
    (f64) => {
        f64
    };
    (i64) => {
        i64
    };
    (i32) => {
        i32
    };
    (bool) => {
        bool
    };
    (option_f64) => {
        Option<f64> // this is the same because everything's an option, the option tag only means it can accept options
    };
    (option_i64) => {
        Option<i64> // this is the same because everything's an option, the option tag only means it can accept options
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
    (biome_criteria) => {
        BiomeCriteria
    };
    (lake_type) => {
        LakeType
    };
    (option_lake_type) => {
        Option<LakeType> // this is the same because everything's an option, the option tag only means it can accept options
    }
}

macro_rules! feature_set_field_type {
    (f64) => {
        f64
    };
    (option_f64) => {
        Option<f64>
    };
    (i64) => {
        i64
    };
    (option_i64) => {
        Option<i64>
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
    (biome_criteria) => {
        &BiomeCriteria
    };
    (lake_type) => {
        &LakeType
    };
    (option_lake_type) => {
        Option<&LakeType>
    }
}

macro_rules! feature_get_field {
    ($self: ident f64 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_double_by_name($field)?.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))?)
    };
    ($self: ident option_f64 $feature_name: literal $prop: ident $field: path) => {
        // see above for getfieldtype option_f64
        Ok($self.feature.field_as_double_by_name($field)?)
    };
    ($self: ident i64 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer64_by_name($field)?.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))?)
    };
    ($self: ident option_i64 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer64_by_name($field)?)
    };
    ($self: ident i32 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))?)
    };
    ($self: ident option_i32 $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?)
    };
    ($self: ident bool $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))? != 0)
    };
    ($self: ident neighbor_directions $feature_name: literal $prop: ident $field: path) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name($field)? {
            Ok(neighbors.split(',').filter_map(|a| {
                let mut a = a.splitn(2, ':');
                if let Some(neighbor) = a.next().map(|n| n.parse().ok()).flatten() {
                    if let Some(direction) = a.next().map(|d| d.parse().ok()).flatten() {
                        if direction >= 0 {
                            Some((neighbor,direction))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
                
            }).collect())
        } else {
            Ok(Vec::new())
        }

    };
    ($self: ident id_list $feature_name: literal $prop: ident $field: path) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name($field)? {
            Ok(neighbors.split(',').filter_map(|a| {
                a.parse().ok()
            }).collect())
        } else {
            Ok(Vec::new())
        }

    };
    ($self: ident river_segment_from $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(RiverSegmentFrom::try_from(value)?)
        } else {
            Err(CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))
        }

    };
    ($self: ident river_segment_to $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(RiverSegmentTo::try_from(value)?)
        } else {
            Err(CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))
        }

    };
    ($self: ident string $feature_name: literal $prop: ident $field: path) => {
        Ok($self.feature.field_as_string_by_name($field)?.ok_or_else(|| CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))?)
    };
    ($self: ident biome_criteria $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(BiomeCriteria::try_from(value)?)
        } else {
            Err(CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))
        }

    };
    ($self: ident lake_type $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(LakeType::try_from(value)?)
        } else {
            Err(CommandError::MissingField(concat!($feature_name,".",stringify!($prop))))
        }

    };
    ($self: ident option_lake_type $feature_name: literal $prop: ident $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            if value == "" {
                // we're storing null strings as empty for now.
                Ok(None)
            } else {
                Ok(Some(LakeType::try_from(value)?))
            }
        } else {
            Ok(None)
        }

    };
}

macro_rules! feature_set_field {
    ($self: ident $value: ident f64 $field: path) => {
        Ok($self.feature.set_field_double($field, $value)?)
    };
    ($self: ident $value: ident option_f64 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_double($field, value)?)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            Ok($self.feature.set_field_double($field,f64::NAN)?)
        }
    };
    ($self: ident $value: ident i32 $field: path) => {
        Ok($self.feature.set_field_integer($field, $value)?)
    };
    ($self: ident $value: ident option_i32 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_integer($field, value)?)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            Ok($self.feature.set_field_double($field,f64::NAN)?)
        }
    };
    ($self: ident $value: ident i64 $field: path) => {
        Ok($self.feature.set_field_integer64($field, $value)?)
    };
    ($self: ident $value: ident option_i64 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_integer64($field, value)?)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            Ok($self.feature.set_field_double($field,f64::NAN)?)
        }
    };
    ($self: ident $value: ident bool $field: path) => {
        Ok($self.feature.set_field_integer($field, $value.into())?)
    };
    ($self: ident $value: ident neighbor_directions $field: path) => {{
        let neighbors = feature_conv!(neighbor_directions_to_string@ $value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    ($self: ident $value: ident id_list $field: path) => {{
        let neighbors = feature_conv!(id_list_to_string@ $value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    ($self: ident $value: ident river_segment_from $field: path) => {{
        Ok($self.feature.set_field_string($field, $value.into())?)
    }};
    ($self: ident $value: ident river_segment_to $field: path) => {{
        Ok($self.feature.set_field_string($field, $value.into())?)
    }};
    ($self: ident $value: ident string $field: path) => {{
        Ok($self.feature.set_field_string($field, $value)?)
    }};
    ($self: ident $value: ident biome_criteria $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident lake_type $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    ($self: ident $value: ident option_lake_type $field: path) => {{
        if let Some(value) = $value {
            Ok($self.feature.set_field_string($field, &Into::<String>::into(value))?)
        } else {
            // There's no unsetfield, and unfortunately if I use the tricks above for numbers, it doesn't work for strings
            // so this is the best I can do.
            // FUTURE: I've put in a feature request to gdal crate.
            Ok($self.feature.set_field_string($field,"")?)
        }        
    }};
}

macro_rules! feature_to_value {
    ($prop: ident f64) => {
        FieldValue::RealValue($prop)
    };
    ($prop: ident i32) => {
        FieldValue::IntegerValue($prop)
    };
    ($prop: ident bool) => {
        FieldValue::IntegerValue($prop.into())
    };
    ($prop: ident option_f64) => {
        if let Some(value) = $prop {
            FieldValue::RealValue(value)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            FieldValue::RealValue(f64::NAN)
        }
    };
    ($prop: ident option_i32) => {
        if let Some(value) = $prop {
            FieldValue::IntegerValue(value)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            FieldValue::RealValue(f64::NAN)
        }
    };
    ($prop: ident option_i64) => {
        if let Some(value) = $prop {
            FieldValue::Integer64Value(value)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            FieldValue::RealValue(f64::NAN)
        }
    };
    ($prop: ident id_list) => {
        FieldValue::StringValue(feature_conv!(id_list_to_string@ $prop))
    };
    ($prop: ident neighbor_directions) => {
        FieldValue::StringValue(feature_conv!(neighbor_directions_to_string@ $prop))
    };
    ($prop: ident i64) => {
        FieldValue::Integer64Value($prop)
    };
    ($prop: ident river_segment_from) => {{
        FieldValue::StringValue(Into::<&str>::into($prop).to_owned())
    }};
    ($prop: ident river_segment_to) => {{
        FieldValue::StringValue(Into::<&str>::into($prop).to_owned())
    }};
    ($prop: ident string) => {{
        FieldValue::StringValue($prop.to_owned())
    }};
    ($prop: ident biome_criteria) => {{
        FieldValue::StringValue(Into::<String>::into($prop))
    }};
    ($prop: ident lake_type) => {{
        FieldValue::StringValue(Into::<String>::into($prop))
    }};
    ($prop: ident option_lake_type) => {
        if let Some(value) = $prop {
            FieldValue::StringValue(Into::<String>::into(value))
        } else {
            // There's no unsetfield, and unfortunately if I use the tricks above for numbers, it doesn't work for strings
            // so this is the best I can do.
            // FUTURE: I've put in a feature request to gdal crate.
            FieldValue::StringValue("".to_owned())
        }
    };

}

pub(crate) trait TypedFeature<'data_life>: From<Feature<'data_life>>  {

    const GEOMETRY_TYPE: OGRwkbGeometryType::Type;

    const LAYER_NAME: &'static str;

    fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)];

    fn fid(&self) -> Result<u64,CommandError>;

    fn into_feature(self) -> Feature<'data_life>;

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

macro_rules! feature {
    ($struct_name:ident $iterator: ident $layer_name: literal $geometry_type: ident $(fid: #[$fid_attr: meta])? $(geometry: #[$geometry_attr: meta])? $(to_field_names_values: #[$to_values_attr: meta])? {$(
        $(#[$get_attr: meta])* $prop: ident 
        $(#[$set_attr: meta])* $set_prop: ident 
        $prop_type: ident 
        $field: ident 
        $name: literal 
        $field_type: path;
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

        impl<'impl_life> TypedFeature<'impl_life> for $struct_name<'impl_life> {

            const GEOMETRY_TYPE: OGRwkbGeometryType::Type = OGRwkbGeometryType::$geometry_type;

            const LAYER_NAME: &'static str = $layer_name;

            fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)] {
                &Self::FIELD_DEFS
            }

            // fid field
            fn fid(&self) -> Result<u64,CommandError> {
                self.feature.fid().ok_or_else(|| CommandError::MissingField(concat!($layer_name,".","fid")))
            }

            fn into_feature(self) -> Feature<'impl_life> {
                self.feature
            }

        }
        
        impl $struct_name<'_> {

            // constant field names
            $(pub(crate) const $field: &str = $name;)*

            // field definitions
            const FIELD_DEFS: [(&str,OGRFieldType::Type); feature_count_fields!($($field),*)] = [
                $((Self::$field,$field_type)),*
            ];
    
            // geometry field
            $(#[$geometry_attr])? pub(crate) fn geometry(&self) -> Option<&Geometry> {
                self.feature.geometry()
            }
    
            // feature initializer function
            $(#[$to_values_attr])? pub(crate) fn to_field_names_values($($prop: feature_set_field_type!($prop_type)),*) -> ([&'static str; feature_count_fields!($($field),*)],[FieldValue; feature_count_fields!($($field),*)]) {
                ([
                    $(Self::$field),*
                ],[
                    $(feature_to_value!($prop $prop_type)),*
                ])
    
            }
        
            // property functions
            $(
                $(#[$get_attr])* pub(crate) fn $prop(&self) -> Result<feature_get_field_type!($prop_type),CommandError> {
                    feature_get_field!(self $prop_type $layer_name $prop Self::$field)
                }
        
                $(#[$set_attr])* pub(crate) fn $set_prop(&mut self, value: feature_set_field_type!($prop_type)) -> Result<(),CommandError> {
                    feature_set_field!(self value $prop_type Self::$field)
                }            
        
            )*
        }

    };
}


pub(crate) struct TypedFeatureIterator<'data_life, Feature: TypedFeature<'data_life>> {
    features: FeatureIterator<'data_life>,
    _phantom: std::marker::PhantomData<Feature>
}

impl<'impl_life, Feature: TypedFeature<'impl_life>> Iterator for TypedFeatureIterator<'impl_life, Feature> {
    type Item = Feature;

    fn next(&mut self) -> Option<Self::Item> {
        self.features.next().map(Feature::from)
    }
}

impl<'impl_life, Feature: TypedFeature<'impl_life>> From<FeatureIterator<'impl_life>> for TypedFeatureIterator<'impl_life, Feature> {
    fn from(features: FeatureIterator<'impl_life>) -> Self {
        Self {
            features,
            _phantom: std::marker::PhantomData::default()
        }
    }
}

impl<'impl_life, Feature: TypedFeature<'impl_life>> TypedFeatureIterator<'impl_life, Feature> {

    pub(crate) fn to_entities_vec<Progress: ProgressObserver, Data: Entity<'impl_life,Feature>>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        progress.start(|| (format!("Reading {}.",Feature::LAYER_NAME),self.features.size_hint().1));
        let mut result = Vec::new();
        for (i,entity) in self.enumerate() {
            result.push(Data::try_from(entity)?);
            progress.update(|| i);
        }
        progress.finish(|| format!("{} read.",Feature::LAYER_NAME.to_title_case()));
        Ok(result)
    }

    pub(crate) fn to_entities_index<Progress: ProgressObserver, Data: Entity<'impl_life,Feature>>(&mut self, progress: &mut Progress) -> Result<HashMap<u64,Data>,CommandError> {

        progress.start(|| (format!("Indexing {}.",Feature::LAYER_NAME),self.features.size_hint().1));
        let mut result = HashMap::new();
        for (i,feature) in self.enumerate() {
            let fid = feature.fid()?;
            let entity = Data::try_from(feature)?;
 
            result.insert(fid,entity);
            progress.update(|| i);
        }
        progress.finish(|| format!("{} indexed.",Feature::LAYER_NAME.to_title_case()));
        Ok(result)
    }


}


pub(crate) trait Entity<'trait_life, Feature: TypedFeature<'trait_life>>: TryFrom<Feature,Error=CommandError> {

}

pub(crate) struct EntityIterator<'data_life, Feature: TypedFeature<'data_life>, Data: Entity<'data_life,Feature>> {
    features: TypedFeatureIterator<'data_life,Feature>,
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'impl_life,Feature: TypedFeature<'impl_life>, Data: Entity<'impl_life,Feature>> Iterator for EntityIterator<'impl_life,Feature,Data> {
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
}

impl<'impl_life,Feature: TypedFeature<'impl_life>, Data: Entity<'impl_life,Feature>> From<TypedFeatureIterator<'impl_life,Feature>> for EntityIterator<'impl_life,Feature,Data> {
    fn from(features: TypedFeatureIterator<'impl_life,Feature>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}

#[macro_export]
macro_rules! entity_field_assign {
    ($feature: ident geometry $type: ty) => {
        $feature.geometry().cloned().ok_or_else(|| CommandError::MissingGeometry)?
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
macro_rules! entity_field_def {
    ($type: ty [$function: expr]) => {
        $type
    };
    ($type: ty) => {
        $type
    };
}

#[macro_export]
macro_rules! entity {
    ($name: ident $feature: ident {$($field: ident: $type: ty $(= $function: expr)?),*}) => {
        #[derive(Clone)]
        pub(crate) struct $name {
            $(
                pub(crate) $field: crate::entity_field_def!($type $([$function])?)
            ),*
        }

        impl<'impl_life> Entity<'impl_life, $feature<'impl_life>> for $name {

        }

        impl TryFrom<$feature<'_>> for $name {

            type Error = CommandError;

            fn try_from(value: $feature) -> Result<Self,Self::Error> {
                crate::entity_from_data!($name value, $($field: $type $(= $function)?),*)
            }
        }

    };
}

pub(crate) struct MapLayer<'data_life, Feature: TypedFeature<'data_life>> {
    layer: Layer<'data_life>,
    _phantom: std::marker::PhantomData<Feature>
}

impl<'impl_life, Feature: TypedFeature<'impl_life>> MapLayer<'impl_life,Feature> {


    fn create_from_dataset(dataset: &'impl_life mut Dataset, overwrite: bool) -> Result<Self,CommandError> {

        let layer = dataset.create_layer(LayerOptions {
            name: Feature::LAYER_NAME,
            ty: Feature::GEOMETRY_TYPE,
            options: if overwrite { 
                Some(&["OVERWRITE=YES"])
            } else {
                None
            },
            ..Default::default()
        })?;
        layer.create_defn_fields(Feature::get_field_defs())?;
        
        Ok(Self {
            layer,
            _phantom: Default::default()
        })
    }

    fn open_from_dataset(dataset: &'impl_life Dataset) -> Result<Self,CommandError> {
        
        let layer = dataset.layer_by_name(Feature::LAYER_NAME)?;
        Ok(Self {
            layer,
            _phantom: Default::default()
        })

    }
    
    // FUTURE: I wish I could get rid of the lifetime thingie...
    pub(crate) fn feature_by_id(&'impl_life self, fid: &u64) -> Option<Feature> {
        self.layer.feature(*fid).map(Feature::from)
    }

    pub(crate) fn try_feature_by_id(&'impl_life self, fid: &u64) -> Result<Feature,CommandError> {
        self.layer.feature(*fid).ok_or_else(|| CommandError::MissingFeature(Feature::LAYER_NAME,*fid)).map(Feature::from)
    }


    pub(crate) fn update_feature(&self, feature: Feature) -> Result<(),CommandError> {
        Ok(self.layer.set_feature(feature.into_feature())?)
    }

    // FUTURE: It would be nice if we could set the filter and retrieve the features all at once. But then I have to implement drop.
    pub(crate) fn set_spatial_filter_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y)
    }

    pub(crate) fn clear_spatial_filter(&mut self) {
        self.layer.clear_spatial_filter()
    }

    pub(crate) fn feature_count(&self) -> usize {
        self.layer.feature_count() as usize
    }

    pub(crate) fn read_geometries(&mut self) -> LayerGeometryIterator {
        LayerGeometryIterator::new(&mut self.layer)
    }

    fn add_feature(&mut self, geometry: Geometry, field_names: &[&str], field_values: &[FieldValue]) -> Result<(),CommandError> {
        // I dug out the source to get this. I wanted to be able to return the feature being created.
        let mut feature = gdal::vector::Feature::new(self.layer.defn())?;
        feature.set_geometry(geometry)?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            feature.set_field(&field, value)?;
        }
        feature.create(&self.layer)?;
        Ok(())
    }


    fn add_feature_without_geometry(&mut self, field_names: &[&str], field_values: &[FieldValue]) -> Result<(),CommandError> {
        // This function is used for lookup tables, like biomes.

        // I had to dig into the source to get this stuff...
        let feature = gdal::vector::Feature::new(self.layer.defn())?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            feature.set_field(field, value)?;
        }
        feature.create(&self.layer)?;
        Ok(())

    }
}


feature!(PointFeature PointEntityIterator "points" wkbPoint geometry: #[allow(dead_code)] to_field_names_values: #[allow(dead_code)] {});
type PointsLayer<'data_life> = MapLayer<'data_life,PointFeature<'data_life>>;


impl PointsLayer<'_> {

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.add_feature(point,&[],&[])?;
        Ok(())
    
    }

}

feature!(TriangleFeature TriangleEntityIterator "triangles" wkbPolygon geometry: #[allow(dead_code)] to_field_names_values: #[allow(dead_code)] {});
type TrianglesLayer<'data_life> = MapLayer<'data_life,TriangleFeature<'data_life>>;



impl TrianglesLayer<'_> {

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.add_feature(geo,&[],&[])?;
        Ok(())

    }


}

feature!(TileFeature TileEntityIterator "tiles" wkbPolygon to_field_names_values: #[allow(dead_code)] {
    /// longitude of the node point for the tile's voronoi
    site_x #[allow(dead_code)] set_site_x f64 FIELD_SITE_X "site_x" OGRFieldType::OFTReal;
    /// latitude of the node point for the tile's voronoi
    site_y #[allow(dead_code)] set_site_y f64 FIELD_SITE_Y "site_y" OGRFieldType::OFTReal;
    /// elevation in meters of the node point for the tile's voronoi
    elevation set_elevation f64 FIELD_ELEVATION "elevation" OGRFieldType::OFTReal;
    // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
    // If I ever get rid of those algorithms, this field can go away.
    /// elevation scaled into a value from 0 to 100, where 20 is sea-level.
    elevation_scaled set_elevation_scaled i32 FIELD_ELEVATION_SCALED "elevation_scaled" OGRFieldType::OFTInteger;
    /// whether the tile is marked as ocean
    is_ocean set_is_ocean bool FIELD_IS_OCEAN "is_ocean" OGRFieldType::OFTInteger;
    /// average annual temperature of tile in imaginary units
    temperature set_temperature f64 FIELD_TEMPERATURE "temperature" OGRFieldType::OFTReal;
    /// roughly estimated average wind direction for tile
    wind set_wind i32 FIELD_WIND "wind_dir" OGRFieldType::OFTInteger;
    /// average annual precipitation of tile in imaginary units
    precipitation set_precipitation f64 FIELD_PRECIPITATION "precipitation" OGRFieldType::OFTReal;
    /// amount of water flow through tile in imaginary units
    #[allow(dead_code)] water_flow set_water_flow f64 FIELD_WATER_FLOW "water_flow" OGRFieldType::OFTReal;
    /// amount of water accumulating (because it couldn't flow on) in imaginary units
    #[allow(dead_code)] water_accumulation set_water_accumulation f64 FIELD_WATER_ACCUMULATION "water_accum" OGRFieldType::OFTReal;
    /// if the tile is in a lake, this is the elevation of the lake's surface, if not a lake then this will be null.
    #[allow(dead_code)] lake_elevation set_lake_elevation option_f64 FIELD_LAKE_ELEVATION "lake_elev" OGRFieldType::OFTReal;
    /// if the tile is in a lake, this is the type of lake it is, if not a lake then this will be blank or null.
    #[allow(dead_code)] lake_type set_lake_type option_lake_type FIELD_LAKE_LAKE_TYPE "lake_type" OGRFieldType::OFTString;
    /// id of neighboring tile which water flows to
    #[allow(dead_code)] flow_to set_flow_to id_list FIELD_FLOW_TO "flow_to" OGRFieldType::OFTString;
    /// shortest distance in number of tiles to an ocean or lake shoreline. This will be positive on land and negative inside a water body.
    #[allow(dead_code)] shore_distance set_shore_distance i32 FIELD_SHORE_DISTANCE "shore_distance" OGRFieldType::OFTInteger;
    /// If this is a land tile neighboring a water body, this is the id of the closest tile
    #[allow(dead_code)] closest_water set_closest_water option_i64 FIELD_CLOSEST_WATER "closest_water" OGRFieldType::OFTInteger64;
    /// if this is a land tile neighboring a water body, this is the number of neighbor tiles that are water
    #[allow(dead_code)] water_count set_water_count option_i32 FIELD_WATER_COUNT "water_count" OGRFieldType::OFTInteger;
    /// The biome for this tile
    #[allow(dead_code)] biome set_biome string FIELD_BIOME "biome" OGRFieldType::OFTString;
    /// the factor used to generate population numbers, along with the area of the tile
    #[allow(dead_code)] habitability set_habitability f64 FIELD_HABITABILITY "habitability" OGRFieldType::OFTReal;
    /// base population of the cell outside of the towns.
    #[allow(dead_code)] population set_population i32 FIELD_POPULATION "population" OGRFieldType::OFTInteger;
    // NOTE: This field should only ever have one value or none. However, as I have no way of setting None
    // on a u64 field (until gdal is updated to give me access to FieldSetNone), I'm going to use a vector
    // to store it. In any way, you never know when I might support outlet from multiple points.
    /// If this tile is an outlet from a lake, this is the tile ID from which the water is flowing.
    #[allow(dead_code)] outlet_from set_outlet_from id_list FIELD_OUTLET_FROM "outlet_from" OGRFieldType::OFTString;
    /// A list of all tile neighbors and their angular directions (tile_id:direction)
    neighbors set_neighbors neighbor_directions FIELD_NEIGHBOR_TILES "neighbor_tiles" OGRFieldType::OFTString;

});


impl TileFeature<'_> {

    pub(crate) fn site(&self) -> Result<Point,CommandError> {
        Ok(Point::try_from((self.site_x()?,self.site_y()?))?)
    }

}


pub(crate) trait TileEntityWithNeighborsElevation {

    fn neighbors(&self) -> &Vec<(u64,i32)>;

    fn elevation(&self) -> &f64;
}


entity!(NewTileEntity TileFeature {
    geometry: Geometry,
    site_x: f64, 
    site_y: f64
}); 
entity!(TileEntitySite TileFeature {
    fid: u64, 
    site_x: f64, 
    site_y: f64
});
entity!(TileEntityLatElevOcean TileFeature {
    fid: u64, 
    site_y: f64, 
    elevation: f64, 
    is_ocean: bool
});
entity!(TileEntityLat TileFeature {
    fid: u64, 
    site_y: f64
});
entity!(TileEntityForWaterFlow TileFeature {
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    precipitation: f64,
    temperature: f64,
    water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
    water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    flow_to: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new())
});

impl TileEntityWithNeighborsElevation for TileEntityForWaterFlow {

    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}

// Basically the same struct as WaterFlow, except that the fields are initialized differently. I can't
// just use a different function because it's based on a trait. I could take this one out
// of the macro and figure something out, but this is easier.
entity!(TileEntityForWaterFill TileFeature {
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    water_flow: f64,
    water_accumulation: f64,
    flow_to: Vec<u64>,
    temperature: f64,
    outlet_from: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new()),
    lake_id: Option<usize> = |_| Ok::<_,CommandError>(None)
});

entity!(TileEntityForRiverConnect TileFeature {
    water_flow: f64,
    flow_to: Vec<u64>,
    outlet_from: Vec<u64>
});

impl From<TileEntityForWaterFlow> for TileEntityForWaterFill {

    fn from(value: TileEntityForWaterFlow) -> Self {
        Self {
            elevation: value.elevation,
            temperature: value.temperature,
            is_ocean: value.is_ocean,
            neighbors: value.neighbors,
            water_flow: value.water_flow,
            water_accumulation: value.water_accumulation,
            flow_to: value.flow_to,
            outlet_from: Vec::new(),
            lake_id: None
        }
    }
}

entity!(TileEntityForWaterDistance TileFeature {
    site: Point,
    is_ocean: bool, 
    lake_elevation: Option<f64>,
    neighbors: Vec<(u64,i32)>
});

entity!(TileEntityForWaterDistanceNeighbor TileFeature {
    site: Point,
    is_ocean: bool, 
    lake_elevation: Option<f64>
});

entity!(TileEntityForWaterDistanceOuter TileFeature {
    is_ocean: bool, 
    lake_elevation: Option<f64>,
    neighbors: Vec<(u64,i32)>,
    shore_distance: Option<i32> = |feature: &TileFeature| feature.shore_distance().missing_to_option()
});

entity!(TileEntityForWaterDistanceOuterNeighbor TileFeature {
    shore_distance: Option<i32> = |feature: &TileFeature| feature.shore_distance().missing_to_option()
});

entity!(TileForPopulation TileFeature {
    water_flow: f64,
    elevation_scaled: i32,
    biome: String,
    shore_distance: i32,
    water_count: Option<i32>,
    area: f64 = |feature: &TileFeature| {
        Ok::<_,CommandError>(feature.geometry().map(|g| g.area()).unwrap_or_else(|| 0.0))
    },
    closest_water: Option<i64>,
    lake_type: Option<LakeType>
});

entity!(TileForPopulationNeighbor TileFeature {
    is_ocean: bool,
    lake_type: Option<LakeType>
});

// entities containing is_ocean, lake_elevation, site, neighbors, and new values tile_distance, water_tile, and water_count, all set to None.


impl TileEntityWithNeighborsElevation for TileEntityForWaterFill {

    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}

pub(crate) type TilesLayer<'data_life> = MapLayer<'data_life,TileFeature<'data_life>>;

impl TilesLayer<'_> {


    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TileFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn read_entities<'local, Data: Entity<'local,TileFeature<'local>>>(&'local mut self) -> EntityIterator<'local,TileFeature<'local>, Data> {
        self.read_features().into()
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn try_entity_by_id<'local, Data: Entity<'local,TileFeature<'local>>>(&'local mut self, fid: &u64) -> Result<Data,CommandError> {
        self.try_feature_by_id(&fid)?.try_into()
    }

    pub(crate) fn add_tile(&mut self, tile: NewTileEntity) -> Result<(),CommandError> {

        self.add_feature(tile.geometry,&[
                TileFeature::FIELD_SITE_X,
                TileFeature::FIELD_SITE_Y,
            ],&[
                FieldValue::RealValue(tile.site_x),
                FieldValue::RealValue(tile.site_y),
            ])?;
        Ok(())

    }

    pub(crate) fn estimate_average_tile_area(&self) -> Result<f64,CommandError> {
        let extent = self.layer.get_extent()?;
        let width = extent.MaxX - extent.MinX;
        let height = extent.MaxY - extent.MinY;
        let tiles = self.feature_count();
        Ok((width*height)/tiles as f64)
    }

   // This is for when you want to generate the water fill in a second step, so you can verify the flow first.
   pub(crate) fn get_index_and_queue_for_water_fill<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(HashMap<u64,TileEntityForWaterFill>,Vec<(u64,f64)>),CommandError> {

        let mut tile_map = HashMap::new();
        let mut tile_queue = Vec::new();

        progress.start_known_endpoint(|| ("Indexing data.",self.feature_count() as usize));

        for (i,data) in self.read_entities::<TileEntityForWaterFill>().enumerate() {
            let (fid,entity) = data?;
            if entity.water_accumulation > 0.0 {
                tile_queue.push((fid,entity.water_accumulation));
            }
            tile_map.insert(fid, entity);
            progress.update(|| i);

        }
        progress.finish(|| "Data indexed.");

        Ok((tile_map,tile_queue))
        

    }


}


#[derive(Clone)]
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
        match value.to_lowercase().as_str() {
            "source" => Ok(Self::Source),
            "lake" => Ok(Self::Lake),
            "branch" => Ok(Self::Branch),
            "continuing" => Ok(Self::Continuing),
            "lake-branch" => Ok(Self::BranchingLake),
            "branch-confluence" => Ok(Self::BranchingConfluence),
            "confluence" => Ok(Self::Confluence),
            a => Err(CommandError::InvalidValueForSegmentFrom(a.to_owned()))
        }
    }
}

impl Into<&str> for &RiverSegmentFrom {

    fn into(self) -> &'static str {
        match self {
            RiverSegmentFrom::Source => "source",
            RiverSegmentFrom::Lake => "lake",
            RiverSegmentFrom::Branch => "branch",
            RiverSegmentFrom::Continuing => "continuing",
            RiverSegmentFrom::BranchingLake => "lake-branch",
            RiverSegmentFrom::BranchingConfluence => "branch-confluence",
            RiverSegmentFrom::Confluence => "confluence",
        }
    }
}

#[derive(Clone)]
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
        match value.to_lowercase().as_str() {
            "mouth" => Ok(Self::Mouth),
            "confluence" => Ok(Self::Confluence),
            "continuing" => Ok(Self::Continuing),
            "branch" => Ok(Self::Branch),
            "branch-confluence" => Ok(Self::BranchingConfluence),
            a => Err(CommandError::InvalidValueForSegmentTo(a.to_owned()))
        }
    }
}

impl Into<&str> for &RiverSegmentTo {

    fn into(self) -> &'static str {
        match self {
            RiverSegmentTo::Mouth => "mouth",
            RiverSegmentTo::Confluence => "confluence",
            RiverSegmentTo::Continuing => "continuing",
            RiverSegmentTo::Branch => "branch",
            RiverSegmentTo::BranchingConfluence => "branch-confluence",
        }
    }
}


feature!(RiverFeature RiverEntityIterator "rivers" wkbLineString geometry: #[allow(dead_code)] {
    from_tile #[allow(dead_code)] set_from_tile i64 FIELD_FROM_TILE "from_tile" OGRFieldType::OFTInteger64;
    from_type #[allow(dead_code)] set_from_type river_segment_from FIELD_FROM_TYPE "from_type" OGRFieldType::OFTString;
    from_flow #[allow(dead_code)] set_from_flow f64 FIELD_FROM_FLOW "from_flow" OGRFieldType::OFTReal;
    to_tile #[allow(dead_code)] set_to_tile i64 FIELD_TO_TILE "to_tile" OGRFieldType::OFTInteger64;
    to_type #[allow(dead_code)] set_to_type river_segment_to FIELD_TO_TYPE "to_type" OGRFieldType::OFTString;
    to_flow #[allow(dead_code)] set_to_flow f64 FIELD_TO_FLOW "to_flow" OGRFieldType::OFTReal;
});


entity!(NewRiver RiverFeature {
    from_tile: i64,
    from_type: RiverSegmentFrom,
    from_flow: f64,
    to_tile: i64,
    to_type: RiverSegmentTo,
    to_flow: f64,
    line: Vec<Point> = |_| Ok::<_,CommandError>(Vec::new())
});

pub(crate) type RiversLayer<'data_life> = MapLayer<'data_life,RiverFeature<'data_life>>;

impl RiversLayer<'_> {

    pub(crate) fn add_segment(&mut self, segment: &NewRiver) -> Result<(),CommandError> {
        let geometry = create_line(&segment.line)?;
        let (field_names,field_values) = RiverFeature::to_field_names_values(
            segment.from_tile, 
            &segment.from_type, 
            segment.from_flow, 
            segment.to_tile, 
            &segment.to_type,
            segment.to_flow);
        self.add_feature(geometry, &field_names, &field_values)
    }

}

#[derive(Clone)]
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
        match self {
            LakeType::Fresh => "fresh",
            LakeType::Salt => "salt",
            LakeType::Frozen => "frozen",
            LakeType::Pluvial => "pluvial", 
            LakeType::Dry => "dry",
            LakeType::Marsh => "marsh"
        }.to_owned()
    }
}

impl TryFrom<String> for LakeType {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "fresh" => Ok(Self::Fresh),
            "salt" => Ok(Self::Salt),
            "frozen" => Ok(Self::Frozen),
            "pluvial" => Ok(Self::Pluvial),
            "dry" => Ok(Self::Dry),
            "marsh" => Ok(Self::Marsh),
            _ => Err(CommandError::InvalidValueForLakeType(value))
        }
    }
}

feature!(LakeFeature LakeEntityIterator "lakes" wkbMultiPolygon geometry: #[allow(dead_code)] {
    elevation #[allow(dead_code)] set_elevation f64 FIELD_ELEVATION "elevation" OGRFieldType::OFTReal;
    type_ #[allow(dead_code)] set_type lake_type FIELD_TYPE "type" OGRFieldType::OFTString;
    flow #[allow(dead_code)] set_flow f64 FIELD_FLOW "flow" OGRFieldType::OFTReal;
    size #[allow(dead_code)] set_size i32 FIELD_SIZE "size" OGRFieldType::OFTInteger64;
    temperature #[allow(dead_code)] set_temperature f64 FIELD_TEMPERATURE "temperature" OGRFieldType::OFTReal;
    evaporation #[allow(dead_code)] set_evaporation f64 FIELD_EVAPORATION "evaporation" OGRFieldType::OFTReal;
});


entity!(NewLake LakeFeature {
    elevation: f64,
    type_: LakeType,
    flow: f64,
    size: i32,
    temperature: f64,
    evaporation: f64,
    geometry: Geometry
});

pub(crate) type LakesLayer<'data_life> = MapLayer<'data_life,LakeFeature<'data_life>>;

impl LakesLayer<'_> {

    pub(crate) fn add_lake(&mut self, lake: NewLake) -> Result<(),CommandError> {
        let (field_names,field_values) = LakeFeature::to_field_names_values(
            lake.elevation,
            &lake.type_,
            lake.flow,
            lake.size,
            lake.temperature,
            lake.evaporation
        );
        self.add_feature(lake.geometry, &field_names, &field_values)
    }

}

#[derive(Clone)]
pub(crate) enum BiomeCriteria {
    Matrix(Vec<(usize,usize)>), // moisture band, temperature band
    Wetland,
    Glacier,
    Ocean
}

impl TryFrom<String> for BiomeCriteria {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "wetland" => Ok(Self::Wetland),
            "glacier" => Ok(Self::Glacier),
            "ocean" => Ok(Self::Ocean),
            list => {
                let mut result = Vec::new();
                for value in list.split(',') {
                    let value = value.splitn(2,':');
                    let mut value = value.map(str::parse).map(|a| a.map_err(|_| CommandError::InvalidBiomeMatrixValue(list.to_owned())));
                    let moisture = value.next().ok_or_else(|| CommandError::InvalidBiomeMatrixValue(list.to_owned()))??;
                    let temperature = value.next().ok_or_else(|| CommandError::InvalidBiomeMatrixValue(list.to_owned()))??;
                    result.push((moisture,temperature));
                }
                Ok(Self::Matrix(result))
            }
        }
    }
}

impl Into<String> for &BiomeCriteria {

    fn into(self) -> String {
        match self {
            BiomeCriteria::Wetland => "wetland".to_owned(),
            BiomeCriteria::Glacier => "glacier".to_owned(),
            BiomeCriteria::Ocean => "ocean".to_owned(),
            BiomeCriteria::Matrix(list) => {
                list.iter().map(|(moisture,temperature)| format!("{}:{}",moisture,temperature)).collect::<Vec<String>>().join(",")
            }
        }
    }
}

pub(crate) struct BiomeMatrix {
    pub(crate) matrix: [[String; 26]; 5],
    pub(crate) ocean: String,
    pub(crate) glacier: String,
    pub(crate) wetland: String
}

feature!(BiomeFeature BiomeEntityIterator "biomes" wkbNone geometry: #[allow(dead_code)] {
    #[allow(dead_code)] name #[allow(dead_code)] set_name string FIELD_NAME "name" OGRFieldType::OFTString;
    #[allow(dead_code)] habitability #[allow(dead_code)] set_habitability i32 FIELD_HABITABILITY "habitability" OGRFieldType::OFTInteger;
    #[allow(dead_code)] criteria #[allow(dead_code)] set_criteria biome_criteria FIELD_CRITERIA "criteria" OGRFieldType::OFTString;
});

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
    

    const DEFAULT_BIOMES: [(&str, i32, BiomeCriteria); 13] = [
        (Self::OCEAN,0,BiomeCriteria::Ocean),
        (Self::HOT_DESERT,4,BiomeCriteria::Matrix(vec![])),
        (Self::COLD_DESERT,10,BiomeCriteria::Matrix(vec![])),
        (Self::SAVANNA,22,BiomeCriteria::Matrix(vec![])),
        (Self::GRASSLAND,30,BiomeCriteria::Matrix(vec![])),
        (Self::TROPICAL_SEASONAL_FOREST,50,BiomeCriteria::Matrix(vec![])),
        (Self::TEMPERATE_DECIDUOUS_FOREST,100,BiomeCriteria::Matrix(vec![])),
        (Self::TROPICAL_RAINFOREST,80,BiomeCriteria::Matrix(vec![])),
        (Self::TEMPERATE_RAINFOREST,90,BiomeCriteria::Matrix(vec![])),
        (Self::TAIGA,12,BiomeCriteria::Matrix(vec![])),
        (Self::TUNDRA,4,BiomeCriteria::Matrix(vec![])),
        (Self::GLACIER,0,BiomeCriteria::Glacier),
        (Self::WETLAND,12,BiomeCriteria::Wetland),
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

        Self::DEFAULT_BIOMES.iter().map(|(name,habitability,criteria)| {
            let criteria = if let BiomeCriteria::Matrix(_) = criteria {
                BiomeCriteria::Matrix(matrix_criteria.get(name).unwrap().clone())
            } else {
                criteria.clone()
            };
            NewBiome {
                name: (*name).to_owned(),
                habitability: *habitability,
                criteria,
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

entity!(NewBiome BiomeFeature {
    name: String,
    habitability: i32,
    criteria: BiomeCriteria
});

entity!(BiomeData BiomeFeature {
    name: String,
    habitability: i32
});

pub(crate) type BiomeLayer<'data_life> = MapLayer<'data_life,BiomeFeature<'data_life>>;

impl BiomeLayer<'_> {

    pub(crate) fn add_biome(&mut self, biome: &NewBiome) -> Result<(),CommandError> {

        let (field_names,field_values) = BiomeFeature::to_field_names_values(
            &biome.name,biome.habitability,&biome.criteria);
        self.add_feature_without_geometry(&field_names, &field_values)

    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<BiomeFeature> {
        TypedFeatureIterator::from(self.layer.features())
    }

    pub(crate) fn get_matrix<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<BiomeMatrix,CommandError> {
        let result = self.read_features().to_entities_vec(progress)?;
    
        BiomeFeature::build_matrix_from_biomes(&result)
    
    }

    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn read_entities<'local, Data: Entity<'local,BiomeFeature<'local>>>(&'local mut self) -> EntityIterator<'local,BiomeFeature<'local>, Data> {
        self.read_features().into()
    }
    

}

pub(crate) struct WorldMap {
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &str = "GPKG";

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    #[allow(dead_code)] pub(crate) fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        let dataset = Dataset::open(path)?;
        Ok(Self::new(dataset))
    }


    pub(crate) fn edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open_ex(path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?))
    }

    pub(crate) fn create_or_edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(path)?;
            Ok(Self::new(dataset))
        }

    }

    pub(crate) fn with_transaction<ResultType, Callback: FnOnce(&mut WorldMapTransaction) -> Result<ResultType,CommandError>>(&mut self, callback: Callback) -> Result<ResultType,CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        let result = callback(&mut transaction)?;
        transaction.dataset.commit()?;    
        Ok(result)

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

    pub(crate) fn edit_tile_layer(&mut self) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_biomes_layer(&mut self, overwrite: bool) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }

    #[allow(dead_code)] pub(crate) fn edit_biomes_layer(&mut self) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::open_from_dataset(&mut self.dataset)?)

    }

}

