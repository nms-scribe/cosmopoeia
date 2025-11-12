use std::collections::HashSet;

use angular_units::Deg;
use gdal::vector::LayerAccess as _;

use crate::algorithms::water_flow::WaterFlowResult;
use crate::entity;
use crate::errors::CommandError;
use crate::geometry::Polygon;
use crate::layer;
use crate::progress::ProgressObserver;
use crate::utils::coordinates::Coordinates;
use crate::utils::edge::Edge;
use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use crate::world_map::biome_layer::BiomeForCultureGen;
use crate::world_map::biome_layer::BiomeSchema;
use crate::world_map::fields::Grouping;
use crate::typed_map::fields::IdRef;
use crate::world_map::fields::Neighbor;
use crate::world_map::fields::NeighborAndDirection;
use crate::typed_map::entities::Entity;
use crate::typed_map::entities::EntityIndex;
use crate::typed_map::entities::EntityLookup;
use crate::typed_map::features::TypedFeature as _;
use crate::typed_map::features::TypedFeatureIterator;
use crate::typed_map::fields::TypedField as _;
use crate::world_map::water_layers::LakeForCultureGen;
use crate::world_map::water_layers::LakeSchema;

layer!(#[hide_add(true)] #[hide_doc(false)] Tile["tiles"]: Polygon {
    /// longitude of the node point for the tile's voronoi
    #[set(allow(dead_code))] site_x: f64,
    /// latitude of the node point for the tile's voronoi
    #[set(allow(dead_code))] site_y: f64,
    /// calculated area based on shape of world (this may not be the same as the area calculated by GDAL)
    #[set(allow(dead_code))] area: f64,
    /// elevation in meters of the node point for the tile's voronoi
    elevation: f64,
    // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
    // If I ever get rid of those algorithms, this field can go away.
    /// elevation scaled into a value from 0 to 100, where 20 is sea-level.
    elevation_scaled: i32,
    /// Indicates whether the tile is part of the ocean, an island, a continent, a lake, and maybe others.
    grouping: Grouping,
    /// A unique id for each grouping. These id's do not map to other tables, but will tell when tiles are in the same group. Use lake_id to link to the lake table.
    // NOTE: This isn't an IdRef, but let's store it that way anyway
    grouping_id: IdRef,
    /// average annual temperature of tile in imaginary units
    temperature: f64,
    /// roughly estimated average wind direction for tile
    wind: Deg<f64>,
    /// average annual precipitation of tile in imaginary units
    precipitation: f64,
    /// amount of water flow through tile in imaginary units
    water_flow: f64,
    /// amount of water accumulating (because it couldn't flow on) in imaginary units
    water_accumulation: f64,
    /// if the tile is in a lake, this is the id of the lake in the lakes layer
    lake_id: Option<IdRef>,
    /// id of neighboring tile which water flows to
    flow_to: Vec<Neighbor>,
    /// shortest distance in number of tiles to an ocean or lake shoreline. This will be positive on land and negative inside a water body.
    shore_distance: i32,
    /// If this is a land tile neighboring a water body, this is the id of the closest tile
    harbor_tile_id: Option<Neighbor>,
    /// if this is a land tile neighboring a water body, this is the number of neighbor tiles that are water
    water_count: Option<i32>,
    /// The biome for this tile
    biome: String,
    /// the factor used to generate population numbers, along with the area of the tile
    habitability: f64,
    /// base population of the cell outside of the towns.
    population: i32,
    /// The name of the culture assigned to this tile, unless wild
    culture: Option<String>,
    /// if the tile has a town, this is the id of the town in the towns layer
    town_id: Option<IdRef>, 
    /// if the tile is part of a nation, this is the id of the nation which controls it
    nation_id: Option<IdRef>,
    /// if the tile is part of a subnation, this is the id of the nation which controls it
    subnation_id: Option<IdRef>,
    /// If this tile is an outlet from a lake, this is the neighbor from which the water is flowing.
    outlet_from: Option<Neighbor>,
    /// A list of all tile neighbors and their angular directions (tile_id:direction)
    neighbors: Vec<NeighborAndDirection>,
    /// A value indicating whether the tile is on the edge of the map
    #[set(allow(dead_code))] edge: Option<Edge>,

});

impl TileFeature<'_> {

    pub(crate) fn site(&self) -> Result<Coordinates,CommandError> {
        Ok(Coordinates::try_from((self.site_x()?,self.site_y()?))?)
    }

}

pub(crate) trait TileWithNeighbors: Entity<TileSchema> {

    fn neighbors(&self) -> &Vec<NeighborAndDirection>;

}

pub(crate) trait TileWithGeometry: Entity<TileSchema> {
    fn geometry(&self) -> &Polygon;
}

pub(crate) trait TileWithShoreDistance: Entity<TileSchema> {
    fn shore_distance(&self) -> &i32;
}

entity!(NewTileSite: Tile {
    #[get=false] geometry: Polygon,
    site: Coordinates,
    #[get=false] edge: Option<Edge>,
    #[get=false] area: f64
});

impl NewTileSite {

    pub(crate) const fn new(geometry: Polygon,
        site: Coordinates,
        edge: Option<Edge>,
        area: f64) -> Self {
        Self { 
            geometry, 
            site, 
            edge, 
            area 
        }

    }
}

entity!(TileForCalcNeighbors: Tile {
    geometry: Polygon,
    edge: Option<Edge>,
    site: Coordinates,
    #[mut=true] neighbor_set: HashSet<IdRef> = |_| Ok::<_,CommandError>(HashSet::new()),
    #[mut=true] cross_neighbor_set: HashSet<IdRef> = |_| Ok::<_,CommandError>(HashSet::new())
});

entity!(TileForTerrain: Tile {
    site: Coordinates, 
    #[set=true] #[mut=true] elevation: f64,
    #[set=true] grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    // 'old' values so the algorithm can check if it's changed.
    #[get=false] old_elevation: f64 = TileFeature::elevation,
    #[get=false] old_grouping: Grouping = TileFeature::grouping
});

impl TileForTerrain {

    pub(crate) fn elevation_changed(&self) -> bool {
        (self.elevation - self.old_elevation).abs() > f64::EPSILON
    }

    pub(crate) fn grouping_changed(&self) -> bool {
        self.grouping != self.old_grouping
    }
}

entity!(TileForTemperatures: Tile {
    fid: IdRef, 
    site_y: f64, 
    elevation: f64, 
    grouping: Grouping
});

entity!(TileForWinds: Tile {
    fid: IdRef, 
    site_y: f64
});

entity!(TileForWaterflow: Tile {
    elevation: f64, 
    #[set=true] flow_to: Vec<Neighbor> = |_| Ok::<_,CommandError>(Vec::new()),
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    precipitation: f64, // not in TileForWaterFill
    #[get=false] temperature: f64,
    #[mut=true] water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    #[set=true] #[mut=true] water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
});

// Basically the same struct as WaterFlow, except that the fields are initialized differently. I can't
// just use a different function because it's based on a trait. I could take this one out
// of the macro and figure something out, but this is easier.
entity!(TileForWaterFill: Tile {
    elevation: f64, 
    flow_to: Vec<Neighbor>, // Initialized to blank in TileForWaterFlow
    grouping: Grouping, 
    #[set=true] lake_id: Option<IdRef> = |_| Ok::<_,CommandError>(None), // Not in TileForWaterFlow
    neighbors: Vec<NeighborAndDirection>,
    #[set=true] outlet_from: Option<Neighbor> = |_| Ok::<_,CommandError>(None), // Not in TileForWaterFlow
    temperature: f64,
    #[get=false] water_accumulation: f64,  // Initialized to blank in TileForWaterFlow
    #[mut=true] water_flow: f64,  // Initialized to blank in TileForWaterFlow
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
            outlet_from: None,
            lake_id: None
        }
    }
}

entity!(TileForRiverConnect: Tile {
    water_flow: f64,
    flow_to: Vec<Neighbor>,
    outlet_from: Option<Neighbor>
});

entity!(TileForWaterDistance: Tile {
    site: Coordinates,
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
    #[set=true] water_count: Option<i32> = |_| Ok::<_,CommandError>(None),
    #[set=true] closest_water_tile_id: Option<Neighbor> = |_| Ok::<_,CommandError>(None)
});

entity!(TileForGroupingCalc: Tile {
    grouping: Grouping,
    edge: Option<Edge>,
    lake_id: Option<IdRef>,
    neighbors: Vec<NeighborAndDirection>
});

entity!(TileForPopulation: Tile {
    water_flow: f64,
    elevation_scaled: i32,
    biome: String,
    shore_distance: i32,
    water_count: Option<i32>,
    area: f64,
    harbor_tile_id: Option<Neighbor>,
    lake_id: Option<IdRef>
});

entity!(TileForPopulationNeighbor: Tile {
    grouping: Grouping,
    lake_id: Option<IdRef>
});

entity!(TileForCultureGen: Tile {
    #[get=false] fid: IdRef,
    #[get=false] site: Coordinates,
    population: i32,
    habitability: f64,
    #[get=false] shore_distance: i32,
    #[get=false] elevation_scaled: i32,
    #[get=false] biome: String,
    #[get=false] water_count: Option<i32>,
    #[get=false] harbor_tile_id: Option<Neighbor>,
    #[get=false] grouping: Grouping,
    #[get=false] water_flow: f64,
    #[get=false] temperature: f64

});

pub(crate) struct TileForCulturePrefSorting<'struct_life> { // NOT an entity because we add in data from other layers.
    fid: IdRef,
    site: Coordinates,
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: &'struct_life BiomeForCultureGen,
    water_count: Option<i32>,
    neighboring_lake_size: Option<i32>,
    grouping: Grouping,
    water_flow: f64,
    temperature: f64
}

impl TileForCulturePrefSorting<'_> {

    pub(crate) fn from<'biomes>(tile: TileForCultureGen, tiles: &TileLayer, biomes: &'biomes EntityLookup<BiomeSchema,BiomeForCultureGen>, lakes: &EntityIndex<LakeSchema,LakeForCultureGen>) -> Result<TileForCulturePrefSorting<'biomes>,CommandError> {
        let biome = biomes.try_get(&tile.biome)?;
        let neighboring_lake_size = if let Some(closest_water) = tile.harbor_tile_id {
            match closest_water {
                Neighbor::Tile(closest_water) | Neighbor::CrossMap(closest_water,_) => {
                    let closest_water = tiles.try_feature_by_id(&closest_water)?;
                    if let Some(lake_id) = closest_water.lake_id()? {
                        let lake = lakes.try_get(&lake_id)?;
                        Some(*lake.size())
                    } else {
                        None
                    }
   
                },
                Neighbor::OffMap(_) => unreachable!("Why on earth would the closest_water be off the map?"),
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
    
    pub(crate) const fn habitability(&self) -> f64 {
        self.habitability
    }
    
    pub(crate) const fn shore_distance(&self) -> i32 {
        self.shore_distance
    }
    
    pub(crate) const fn elevation_scaled(&self) -> i32 {
        self.elevation_scaled
    }
    
    pub(crate) const fn temperature(&self) -> f64 {
        self.temperature
    }
    
    pub(crate) const fn biome(&self) -> &BiomeForCultureGen {
        self.biome
    }
    
    pub(crate) const fn water_count(&self) -> Option<i32> {
        self.water_count
    }
    
    pub(crate) const fn neighboring_lake_size(&self) -> Option<i32> {
        self.neighboring_lake_size
    }
    
    pub(crate) const fn site(&self) -> &Coordinates {
        &self.site
    }
    
    pub(crate) const fn fid(&self) -> &IdRef {
        &self.fid
    }
    
    pub(crate) const fn grouping(&self) -> &Grouping {
        &self.grouping
    }
    
    pub(crate) const fn water_flow(&self) -> f64 {
        self.water_flow
    }
}

entity!(TileForCultureExpand: Tile {
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<NeighborAndDirection>,
    lake_id: Option<IdRef>,
    area: f64,
    #[set=true] culture: Option<String> = |_| Ok::<_,CommandError>(None)

});

entity!(TileForTowns: Tile {
    fid: IdRef,
    habitability: f64,
    site: Coordinates,
    culture: Option<String>,
    grouping_id: IdRef
});

entity!(TileForTownPopulation: Tile {
    #[get=false] fid: IdRef,
    #[get=false] geometry: Polygon,
    habitability: f64,
    site: Coordinates,
    grouping_id: IdRef,
    harbor_tile_id: Option<Neighbor>,
    water_count: Option<i32>,
    temperature: f64,
    lake_id: Option<IdRef>,
    water_flow: f64,
    grouping: Grouping
});

impl TileForTownPopulation {

    pub(crate) fn find_middle_point_between(&self, other: &Self, shape: &WorldShape) -> Result<Coordinates,CommandError> {
        let self_ring = self.geometry.get_ring(0)?;
        let other_ring = other.geometry.get_ring(0)?;
        let other_vertices: Vec<_> = other_ring.into_iter().collect();
        let mut common_vertices: Vec<_> = self_ring.into_iter().collect();
        common_vertices.truncate(common_vertices.len() - 1); // remove the last point, which matches the first
        common_vertices.retain(|p| other_vertices.contains(p));
        if common_vertices.len() == 2 {
            let point1: Coordinates = (common_vertices[0].0,common_vertices[0].1).try_into()?;
            let point2 = (common_vertices[1].0,common_vertices[1].1).try_into()?;
            Ok(point1.shaped_middle_point_between(&point2, shape)?)
        } else {
            Err(CommandError::CantFindMiddlePoint(self.fid.clone(),other.fid.clone(),common_vertices.len()))
        }

    }

    pub(crate) fn find_middle_point_on_edge(&self, edge: &Edge, extent: &Extent, shape: &WorldShape) -> Result<Coordinates,CommandError> {
        let self_ring = self.geometry.get_ring(0)?;
        let mut common_vertices: Vec<_> = self_ring.into_iter().collect();
        common_vertices.truncate(common_vertices.len() - 1); // remove the last point, which matches the first
        common_vertices.retain(|p| edge.contains(p,extent));
        if common_vertices.len() == 2 {
            // NOTE: There will be a problem in cases where the edge is NE,NW,SE,SW, as there are likely going to 
            // be 3 points at least. However, this shouldn't happen since there shouldn't be any cross-map tiles in
            // those directions.
            let point1: Coordinates = (common_vertices[0].0,common_vertices[0].1).try_into()?;
            let point2 = (common_vertices[1].0,common_vertices[1].1).try_into()?;
            Ok(point1.shaped_middle_point_between(&point2,shape)?)
        } else {
            Err(CommandError::CantFindMiddlePointOnEdge(self.fid.clone(),edge.clone(),common_vertices.len()))
        }

    }

}

entity!(TileForNationExpand: Tile {
    habitability: f64,
    shore_distance: i32,
    elevation_scaled: i32,
    biome: String,
    grouping: Grouping,
    water_flow: f64,
    neighbors: Vec<NeighborAndDirection>,
    lake_id: Option<IdRef>,
    culture: Option<String>,
    #[set=true] nation_id: Option<IdRef> = |_| Ok::<_,CommandError>(None),
    area: f64,
});

entity!(TileForNationNormalize: Tile {
    grouping: Grouping,
    neighbors: Vec<NeighborAndDirection>,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>
});

entity!(TileForSubnations: Tile {
    fid: IdRef,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>,
    culture: Option<String>,
    population: i32
});

entity!(TileForSubnationExpand: Tile {
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32,
    elevation_scaled: i32,
    nation_id: Option<IdRef>,
    #[set=true] subnation_id: Option<IdRef> = |_| Ok::<_,CommandError>(None),
    area: f64,
});

entity!(TileForEmptySubnations: Tile {
    neighbors: Vec<NeighborAndDirection>,
    shore_distance: i32,
    nation_id: Option<IdRef>,
    subnation_id: Option<IdRef>,
    town_id: Option<IdRef>,
    population: i32,
    culture: Option<String>,
    area: f64,
});

entity!(TileForSubnationNormalize: Tile {
    neighbors: Vec<NeighborAndDirection>,
    town_id: Option<IdRef>,
    nation_id: Option<IdRef>,
    subnation_id: Option<IdRef>
});

entity!(TileForCultureDissolve: Tile {
    culture: Option<String>,
    #[get=false] geometry: Polygon,
    #[get=false] neighbors: Vec<NeighborAndDirection>,
    #[get=false] shore_distance: i32
});

impl TileWithGeometry for TileForCultureDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForCultureDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForCultureDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForBiomeDissolve: Tile {
    biome: String,
    #[get=false] geometry: Polygon,
    #[get=false] neighbors: Vec<NeighborAndDirection>,
    #[get=false] shore_distance: i32
});

impl TileWithGeometry for TileForBiomeDissolve {

    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForBiomeDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForBiomeDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForNationDissolve: Tile {
    nation_id: Option<IdRef>,
    #[get=false] geometry: Polygon,
    #[get=false] neighbors: Vec<NeighborAndDirection>,
    #[get=false] shore_distance: i32
});

impl TileWithGeometry for TileForNationDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForNationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForNationDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

entity!(TileForSubnationDissolve: Tile {
    subnation_id: Option<IdRef>,
    #[get=false] geometry: Polygon,
    #[get=false] neighbors: Vec<NeighborAndDirection>,
    #[get=false] shore_distance: i32
});

impl TileWithGeometry for TileForSubnationDissolve {
    fn geometry(&self) -> &Polygon {
        &self.geometry
    }
}

impl TileWithShoreDistance for TileForSubnationDissolve {
    fn shore_distance(&self) -> &i32 {
        &self.shore_distance
    }
}

impl TileWithNeighbors for TileForSubnationDissolve {
    fn neighbors(&self) -> &Vec<NeighborAndDirection> {
        &self.neighbors
    }
}

impl TileLayer<'_,'_> {


    // FUTURE: If I can ever get around the lifetime bounds, this should be in the main MapLayer struct.
    // FUTURE: It would also be nice to get rid of the lifetimes
    pub(crate) fn try_entity_by_id<'this, Data: Entity<TileSchema> + TryFrom<TileFeature<'this>,Error=CommandError>>(&'this self, fid: &IdRef) -> Result<Data,CommandError> {
        self.try_feature_by_id(fid)?.try_into()
    }

    pub(crate) fn add_tile(&self, tile: NewTileSite) -> Result<(),CommandError> {
        // tiles are initialized with incomplete definitions in the table. It is a user error to access fields which haven't been assigned yet by running an algorithm before required algorithms are completed.

        let (x,y) = tile.site.to_tuple();

        _ = self.add_feature_with_geometry(tile.geometry,&[
                TileSchema::FIELD_SITE_X,
                TileSchema::FIELD_SITE_Y,
                TileSchema::FIELD_AREA,
                TileSchema::FIELD_EDGE,
                TileSchema::FIELD_ELEVATION,
                TileSchema::FIELD_ELEVATION_SCALED,
                TileSchema::FIELD_GROUPING,
            ],&[
                x.to_field_value()?,
                y.to_field_value()?,
                tile.area.to_field_value()?,
                tile.edge.to_field_value()?,
                // initial tiles start with 0 elevation, terrain commands will edit this...
                0.0.to_field_value()?, // FUTURE: Watch that this type stays correct
                // and scaled elevation starts with 20.
                20.to_field_value()?, // FUTURE: Watch that this type stays correct
                // tiles are continent by default until someone samples some ocean.
                Grouping::Continent.to_field_value()?
            ])?;
        Ok(())

    }

    pub(crate) fn get_layer_size(&self) -> Result<(f64,f64),CommandError> {
        let extent = self.layer().get_extent()?;
        let width = extent.MaxX - extent.MinX;
        let height = extent.MaxY - extent.MinY;
        Ok((width,height))
    }

    /// Gets average tile area in "square degrees" by dividing the width * height of the map by the number of tiles.
    pub(crate) fn estimate_average_tile_area(&self, world_shape: &WorldShape) -> Result<f64,CommandError> {
        let extent = self.get_extent()?;
        let tiles = self.feature_count();
        let result = extent.shaped_area(world_shape)/tiles as f64;
        Ok(result)
    }

    pub(crate) fn get_extent(&self) -> Result<Extent,CommandError> {
        let result = self.layer().get_extent()?;
        Ok(result.into())

    }

    // This is for when you want to generate the water fill in a second step, so you can verify the flow first.
    // It's a function here because it's used in a command, which I want to be as simple as possible.
    pub(crate) fn get_index_and_queue_for_water_fill<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<WaterFlowResult,CommandError> {

        let mut lake_queue = Vec::new();

        let tile_map = self.read_features().into_entities_index_for_each::<_,TileForWaterFill,_>(|fid,tile| {
            if tile.water_accumulation > 0.0 {
                lake_queue.push((fid.clone(),tile.water_accumulation));
            }

            Ok(())
        },progress)?;


        Ok(WaterFlowResult {
            tile_map,
            lake_queue
        })
    

    }


}

