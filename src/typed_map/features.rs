use std::collections::HashMap;

use gdal::vector::Feature;
use gdal::vector::FeatureIterator;
use indexmap::IndexMap;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator as _;
use crate::utils::title_case::ToTitleCase as _;
use crate::typed_map::fields::IdRef;
use crate::typed_map::entities::Entity;
use crate::typed_map::entities::EntityIndex;
use crate::typed_map::entities::EntityIterator;
use crate::typed_map::entities::EntityLookup;
use crate::typed_map::entities::NamedEntity;
use crate::typed_map::schema::Schema;
use core::marker::PhantomData;


pub(crate) trait TypedFeature<'data_life,SchemaType: Schema>: From<Feature<'data_life>>  {

    fn fid(&self) -> Result<IdRef,CommandError>;

    fn into_feature(self) -> Feature<'data_life>;

    fn geometry(&self) -> Result<SchemaType::Geometry,CommandError>;

    fn set_geometry(&mut self, value: SchemaType::Geometry) -> Result<(),CommandError>;


}

pub(crate) trait NamedFeature<'data_life,SchemaType: Schema>: TypedFeature<'data_life,SchemaType> {

    fn get_name(&self) -> Result<String,CommandError>;

}



pub(crate) struct TypedFeatureIterator<'data_life, SchemaType: Schema, Feature: TypedFeature<'data_life,SchemaType>> {
    features: FeatureIterator<'data_life>,
    _phantom_feature: PhantomData<Feature>,
    _phantom_schema: PhantomData<SchemaType>
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
            _phantom_feature: PhantomData,
            _phantom_schema: PhantomData
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
