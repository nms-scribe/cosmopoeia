use gdal::Dataset;
use gdal::LayerOptions;
use gdal::spatial_ref::SpatialRef;
use gdal::vector::FieldValue;
use gdal::vector::Layer;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType;

use crate::errors::CommandError;
use crate::gdal_fixes::FeatureFix;
use crate::geometry::GDALGeometryWrapper;
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::fields::FieldDocumentation;
use crate::typed_map::schema::Schema;

pub(crate) struct LayerDocumentation {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) geometry: String,
    pub(crate) fields: Vec<FieldDocumentation>
}

#[macro_export]
macro_rules! count_ids {
    () => {
        0
    };
    ($prop: ident) => {
        1
    };
    ($prop: ident, $($props: ident),+) => {
        $($crate::count_ids!($props)+)+ $crate::count_ids!($prop)
    };
}

#[macro_export]
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

#[macro_export]
macro_rules! layer {
    ($(#[hide_add($hide_add: literal)])? $(#[hide_read($hide_read: literal)])? $(#[hide_doc($hide_doc: literal)])? $(#[doc = $layer_doc_attr: literal])? $name: ident [$layer_name: literal]: $geometry_type: ident {$(
        $(#[doc = $field_doc_attr: literal])? $(#[get($get_attr: meta)])* $(#[set($set_attr: meta)])* $prop: ident: $prop_type: ty
    ),*$(,)?}) => {

        paste::paste!{
            pub(crate) struct [<$name Feature>]<'data_life> {

                feature: gdal::vector::Feature<'data_life>
            }

        }
    
        paste::paste!{
            impl<'impl_life> From<gdal::vector::Feature<'impl_life>> for [<$name Feature>]<'impl_life> {
    
                fn from(feature: gdal::vector::Feature<'impl_life>) -> Self {
                    Self {
                        feature
                    }
                }
            }

        }

        paste::paste!{
            $(#[doc = $layer_doc_attr])?
            pub(crate) struct [<$name Schema>];
        }

        paste::paste!{
            impl [<$name Schema>] {
                // constant field names
                paste::paste!{
                    $(pub(crate) const [<FIELD_ $prop:snake:upper>]: &'static str = stringify!($prop);)*
                }

                // field definitions
                const FIELD_DEFS: [(&'static str,gdal::vector::OGRFieldType::Type); $crate::count_ids!($($prop),*)] = [
                    $((paste::paste!{Self::[<FIELD_ $prop:snake:upper>]},<$prop_type as $crate::typed_map::fields::TypedField>::STORAGE_TYPE)),*
                ];


            }
        }


        paste::paste!{
            impl $crate::typed_map::schema::Schema for [<$name Schema>] {

                type Geometry = $geometry_type;

                const LAYER_NAME: &'static str = $layer_name;

                fn get_field_defs() -> &'static [(&'static str,gdal::vector::OGRFieldType::Type)] {
                    &Self::FIELD_DEFS
                }


            }
        }

        paste::paste!{

            impl<'impl_life> $crate::typed_map::features::TypedFeature<'impl_life,[<$name Schema>]> for [<$name Feature>]<'impl_life> {

                // fid field
                fn fid(&self) -> Result<IdRef,CommandError> {
                    Ok(IdRef::new(self.feature.fid().ok_or_else(|| CommandError::MissingField(concat!($layer_name,".","fid")))?))
                }

                fn into_feature(self) -> gdal::vector::Feature<'impl_life> {
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
        
        paste::paste!{
        
            impl [<$name Feature>]<'_> {

                // property functions
                $(
                    paste::paste!{
                        $(#[doc = $field_doc_attr])?
                        $(#[$get_attr])* pub(crate) fn $prop(&self) -> Result<$prop_type,CommandError> {
                            <$prop_type as $crate::typed_map::fields::TypedField>::get_field(&self.feature,[<$name Schema>]::[<FIELD_ $prop:snake:upper>],concat!($layer_name,".",stringify!($prop)))
                        }
                    }
        
                    paste::paste!{
                        $(#[doc = $field_doc_attr])?
                        $(#[$set_attr])* pub(crate) fn [<set_ $prop>](&mut self, value: &$prop_type) -> Result<(),CommandError> {
                            $crate::typed_map::fields::TypedField::set_field(value,&self.feature,[<$name Schema>]::[<FIELD_ $prop:snake:upper>])
                        }            
    
                    }
        
                )*

            }

        }

        paste::paste!{

            $crate::hide_item!{$(hide_add $hide_add)?,
                pub(crate) struct [<New $name>] {
                    $(
                        pub(crate) $prop: $prop_type
                    ),*
                }
            }
        }

        paste::paste!{
            pub(crate) type [<$name Layer>]<'layer,'feature> = $crate::typed_map::layers::MapLayer<'layer,'feature,[<$name Schema>],[<$name Feature>]<'feature>>;

            impl [<$name Layer>]<'_,'_> {

                $crate::hide_item!{$(hide_add $hide_add)?,
                    // I've marked entity as possibly not used because some calls have no fields and it won't be assigned.          
                    fn add_struct(&mut self, _entity: &[<New $name>], geometry: Option<<[<$name Schema>] as $crate::typed_map::schema::Schema>::Geometry>) -> Result<IdRef,CommandError> {
                        let field_names = [
                            $(paste::paste!{
                                [<$name Schema>]::[<FIELD_ $prop:snake:upper>]
                            }),*
                        ];
                        let field_values = [
                            $($crate::typed_map::fields::TypedField::to_field_value(&_entity.$prop)?),*
                        ];
                        if let Some(geometry) = geometry {
                            self.add_feature_with_geometry(geometry, &field_names, &field_values)
                        } else {
                            self.add_feature_without_geometry(&field_names, &field_values)
                        }

                    }
                }

                $crate::hide_item!{$(hide_read $hide_read)?,
                    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
                    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<[<$name Schema>],[<$name Feature>]> {
                        TypedFeatureIterator::from(self.layer.features())
                    }
                }


            }

        }

        paste::paste!{
            $crate::hide_item!{$(hide_doc $hide_doc)?,
                pub(crate) fn [<document_ $name:snake _layer>]() -> Result<$crate::typed_map::layers::LayerDocumentation,CommandError> {
                    Ok($crate::typed_map::layers::LayerDocumentation {
                        name: $layer_name.to_owned(),
                        description: concat!("",$($layer_doc_attr: literal)?).trim_start().to_owned(),
                        geometry: stringify!($geometry_type).to_owned(),
                        fields: vec![
                            $(
                                $crate::typed_map::fields::FieldDocumentation {
                                    name: stringify!($prop).to_owned(),
                                    description: concat!("",$($field_doc_attr)?).trim_start().to_owned(),
                                    field_type: <$prop_type as $crate::typed_map::fields::DocumentedFieldType>::get_field_type_documentation()
                                }
                            ),*
            
                        ],
                    })            

                }
            }
        }

    };
}

pub(crate) struct MapLayer<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> {
    pub(crate) layer: Layer<'layer>,
    _phantom_feature: core::marker::PhantomData<&'feature Feature>,
    _phantom_schema: core::marker::PhantomData<SchemaType>
}

impl<'layer, 'feature, SchemaType: Schema, Feature: TypedFeature<'feature, SchemaType>> MapLayer<'layer,'feature,SchemaType,Feature> {


    pub(crate) fn create_from_dataset(dataset: &'layer mut Dataset, overwrite: bool) -> Result<Self,CommandError> {

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

    pub(crate) fn open_from_dataset(dataset: &'layer Dataset) -> Result<Self,CommandError> {
        
        let layer = dataset.layer_by_name(SchemaType::LAYER_NAME)?;
        Ok(Self {
            layer,
            _phantom_feature: core::marker::PhantomData,
            _phantom_schema: core::marker::PhantomData
        })

    }

    pub(crate) fn try_feature_by_id(&'feature self, fid: &IdRef) -> Result<Feature,CommandError> {
        self.layer.feature(fid.to_inner()).ok_or_else(|| CommandError::MissingFeature(SchemaType::LAYER_NAME,fid.clone())).map(Feature::from)
    }


    pub(crate) fn update_feature(&self, feature: Feature) -> Result<(),CommandError> {
        Ok(self.layer.set_feature(feature.into_feature())?)
    }

    pub(crate) fn feature_count(&self) -> usize {
        self.layer.feature_count() as usize
    }

    pub(crate) fn add_feature_with_geometry(&mut self, geometry: SchemaType::Geometry, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<IdRef,CommandError> {
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

    pub(crate) fn add_feature_without_geometry(&mut self, field_names: &[&str], field_values: &[Option<FieldValue>]) -> Result<IdRef,CommandError> {
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
