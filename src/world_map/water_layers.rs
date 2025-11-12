use gdal::vector::LayerAccess as _;

use crate::entity;
use crate::errors::CommandError;
use crate::geometry::LineString;
use crate::geometry::MultiLineString;
use crate::geometry::MultiPolygon;
use crate::geometry::Polygon;
use crate::layer;
use crate::utils::coordinates::Coordinates;
use crate::typed_map::fields::IdRef;
use crate::world_map::fields::LakeType;
use crate::world_map::fields::Neighbor;
use crate::world_map::fields::RiverSegmentFrom;
use crate::world_map::fields::RiverSegmentTo;
use crate::typed_map::features::TypedFeatureIterator;

layer!(#[hide_read(true)] River["rivers"]: MultiLineString {
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


}


layer!(#[hide_read(true)] Coastline["coastlines"]: Polygon  {
});

impl CoastlineLayer<'_,'_> {

    pub(crate) fn add_land_mass(&mut self, geometry: Polygon) -> Result<IdRef, CommandError> {
        self.add_struct(&NewCoastline {  }, Some(geometry))
    }

}

layer!(#[hide_read(true)] Ocean["oceans"]: Polygon {
});

impl OceanLayer<'_,'_> {

    pub(crate) fn add_ocean(&mut self, geometry: Polygon) -> Result<IdRef, CommandError> {
        self.add_struct(&NewOcean {  }, Some(geometry))
    }

}
