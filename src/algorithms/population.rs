use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;
use crate::errors::CommandError;
use crate::world_map::biome_layer::BiomeForPopulation;
use crate::typed_map::features::TypedFeature;
use crate::world_map::tile_layer::TileForPopulation;
use crate::world_map::tile_layer::TileForPopulationNeighbor;
use crate::world_map::fields::LakeType;
use crate::world_map::water_layers::LakeForPopulation;
use crate::commands::RiverThresholdArg;
use crate::world_map::fields::Neighbor;

pub(crate) fn generate_populations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, estuary_threshold: &RiverThresholdArg, progress: &mut Progress) -> Result<(),CommandError> {

    // This algorithm is almost the same as found in AFMG

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    // we need a lake information map
    let mut lakes_layer = target.edit_lakes_layer()?;

    let lake_map = lakes_layer.read_features().into_entities_index::<_,LakeForPopulation>(progress)?;

    // and a biome map
    let biome_map = target.edit_biomes_layer()?.read_features().into_named_entities_index::<_,BiomeForPopulation>(progress)?;

    let mut tiles = target.edit_tile_layer()?;

    let mut work_queue = Vec::new();
    let mut flow_sum = 0.0;
    let mut flow_max: f64 = 0.0;
    let mut area_sum = 0.0;

    for feature in tiles.read_features().watch(progress,"Indexing tiles.","Tiles indexed.") {
        let fid = feature.fid()?;
        let water_flow = feature.water_flow()?;
        flow_sum += water_flow;
        flow_max = flow_max.max(water_flow);
        area_sum += feature.geometry()?.shaped_area(&world_shape)?;
        work_queue.push(fid);

    }

    let flow_mean = flow_sum/work_queue.len() as f64;
    let area_mean = area_sum/work_queue.len() as f64;
    let flow_divisor = flow_max - flow_mean;

    let mut work_queue = work_queue.watch_queue(progress, "Calculating population.", "Population calculated.");
    while let Some(fid) = work_queue.pop() {
        let (habitability,population) = {
            let tile = tiles.try_entity_by_id::<TileForPopulation>(&fid)?; 
            let mut suitability = if tile.lake_id().is_some() {
                0.0
            } else {
                *biome_map.try_get(tile.biome())?.habitability() as f64
            };
            if suitability > 0.0 {
                if flow_mean > 0.0 {
                    suitability += ((tile.water_flow() - flow_mean)/flow_divisor).clamp(0.0,1.0) * 250.0; // big rivers are nice.
                }
                suitability -= (tile.elevation_scaled() - 50) as f64/5.0; // low elevation is preferred
                if tile.shore_distance() == &1 {
                    if tile.water_flow() > &estuary_threshold.river_threshold {
                        suitability += 15.0 // estuaries are liked
                    }
                    if let Some(water_cell) = tile.harbor_tile_id() {
                        match water_cell {
                            Neighbor::Tile(water_cell) | Neighbor::CrossMap(water_cell, _) => {
                                let water_cell = tiles.try_entity_by_id::<TileForPopulationNeighbor>(water_cell)?;
                                if let Some(lake_type) = water_cell.lake_id().as_ref().map(|id| lake_map.try_get(id)).transpose()?.map(LakeForPopulation::type_) {
                                    match lake_type {
                                        LakeType::Fresh => suitability += 30.0,
                                        LakeType::Salt => suitability += 10.0,
                                        LakeType::Frozen => suitability += 1.0,
                                        LakeType::Pluvial => suitability -= 2.0,
                                        LakeType::Dry => suitability -= 5.0,
                                        LakeType::Marsh => suitability += 5.0,
                                    }
                                } else if water_cell.grouping().is_ocean() {
                                    suitability += 5.0;
                                    if tile.water_count() == &Some(1) { // let pattern unecessary
                                        // since it's a land cell bordering a single cell on the ocean, that single cell is a small bay, which
                                        // probably makes a good harbor.
                                        suitability += 20.0
                                    }
                                }
        
                            },
                            Neighbor::OffMap(_) => unreachable!("Why would there be a harbor_tile_id with an OffMap neighbor?"), // FUTURE: I'm not sure if this should ever happen
                        };
                            

                    }
                }
                let habitability = suitability / 5.0; // I don't know why 5, but that's what AFMG did.
                // AFMG Just shows population in thousands, I'm actually going to have more precision, just for looks.
                let population = (((habitability * tile.area())/area_mean) * 1000.0).floor() as i32;
                (habitability,population)
            } else {
                (0.0,0)
            }
        };

        let mut feature = tiles.try_feature_by_id(&fid)?;

        feature.set_habitability(&habitability)?;
        feature.set_population(&population)?;

        tiles.update_feature(feature)?;

    }

    Ok(())
}
