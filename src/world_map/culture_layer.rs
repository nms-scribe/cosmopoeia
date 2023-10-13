use core::hash::Hash;

use gdal::vector::LayerAccess;
use ordered_float::OrderedFloat;
use prisma::Rgb;

use crate::algorithms::naming::Namer;
use crate::algorithms::naming::NamerSet;
use crate::entity;
use crate::errors::CommandError;
use crate::geometry::MultiPolygon;
use crate::layer;
use crate::world_map::fields::CultureType;
use crate::typed_map::fields::IdRef;
use crate::typed_map::entities::NamedEntity;
use crate::typed_map::features::NamedFeature;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;

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


}
