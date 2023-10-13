use std::collections::hash_map::IntoIter;
use std::collections::HashMap;

// rename these imports in case I want to use these from hash_map sometime.
use indexmap::IndexMap;
use indexmap::map::IntoIter as IndexIntoIter;
use indexmap::map::Iter as IndexIter;
use indexmap::map::IterMut as IndexIterMut;
use indexmap::map::Keys as IndexKeys;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;
use crate::typed_map::schema::Schema;


pub(crate) trait Entity<SchemaType: Schema> {

}

pub(crate) trait NamedEntity<SchemaType: Schema>: Entity<SchemaType> {
    fn name(&self) -> &str;
}

pub(crate) struct EntityIterator<'data_life, SchemaType: Schema, Feature: TypedFeature<'data_life,SchemaType>, Data: Entity<SchemaType>> {
    pub(crate) features: TypedFeatureIterator<'data_life,SchemaType,Feature>,
    pub(crate) data: core::marker::PhantomData<Data>
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
    pub(crate) inner: IndexMap<IdRef,EntityType>,
    pub(crate) _phantom: core::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: Entity<SchemaType>> EntityIndex<SchemaType,EntityType> {

    // NOTE: There is no 'insert' or 'new' function because this should be created with to_entities_index.

    pub(crate) fn from(mut inner: IndexMap<IdRef,EntityType>) -> Self {
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
    pub(crate) finish: Message,
    pub(crate) progress: &'progress mut Progress,
    pub(crate) inner: EntityIndex<SchemaType,EntityType>,
    pub(crate) popped: usize,
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
    pub(crate) inner: HashMap<String,EntityType>,
    pub(crate) _phantom: core::marker::PhantomData<SchemaType>
}

impl<SchemaType: Schema, EntityType: NamedEntity<SchemaType>> EntityLookup<SchemaType,EntityType> {

    pub(crate) const fn from(inner: HashMap<String,EntityType>) -> Self {
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
            impl $crate::typed_map::entities::Entity<[<$layer Schema>]> for $name {

            }


        }

        paste::paste!{
            impl TryFrom<[<$layer Feature>]<'_>> for $name {

                type Error = CommandError;

                fn try_from(value: [<$layer Feature>]) -> Result<Self,Self::Error> {
                    $crate::entity_from_data!($name value, $($field: $type $(= $function)?),*)
                }
            }

        }

    };
}
