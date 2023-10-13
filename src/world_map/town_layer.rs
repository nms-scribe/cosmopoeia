use gdal::vector::LayerAccess;

use crate::entity;
use crate::errors::CommandError;
use crate::geometry::Point;
use crate::layer;
use crate::utils::coordinates::Coordinates; // renamed so it doesn't conflict with geometry::Point, which is more important that it keep this name.
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;

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


}
