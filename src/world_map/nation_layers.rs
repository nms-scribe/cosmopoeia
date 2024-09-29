use core::hash::Hash;

use gdal::vector::LayerAccess;
use ordered_float::OrderedFloat;
use prisma::Rgb;

use crate::entity;
use crate::errors::CommandError;
use crate::geometry::MultiPolygon;
use crate::layer;
use crate::world_map::fields::CultureType;
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::NamedFeature;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;

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
    #[get=false] name: String,
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
    #[mut=true] subnation_count: usize = |_| Ok::<_,CommandError>(0) // to be filled in by algorithm
});

impl NationLayer<'_,'_> {

    pub(crate) fn add_nation(&mut self, nation: &NewNation) -> Result<IdRef,CommandError> {
        self.add_struct(nation, None)
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

impl SubnationForPlacement {

    pub(crate) const fn new(fid: IdRef, center_tile_id: IdRef, nation_id: IdRef) -> Self {
        Self {
            fid,
            center_tile_id,
            nation_id
        }
        
    }
}

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


}
