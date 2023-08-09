use std::collections::HashMap;

use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::errors::CommandError;
use crate::world_map::BiomeData;
use crate::world_map::TileForPopulation;
use crate::world_map::LakeType;


pub(crate) fn generate_populations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, estuary_threshold: f64, progress: &mut Progress) -> Result<(),CommandError> {

    // This algorithm is almost the same as found in AFMG

    let mut biome_map = HashMap::new();

    {
        let mut biomes = target.edit_biomes_layer()?;

        for biome in biomes.read_entities::<BiomeData>() {
            let (_,biome) = biome?;
            biome_map.insert(biome.name, biome.habitability);
        }
    
    }

    let mut tiles = target.edit_tile_layer()?;

    let mut tile_map = HashMap::new();
    let mut work_queue = Vec::new();
    let mut flow_sum = 0.0;
    let mut flow_max: f64 = 0.0;
    let mut area_sum = 0.0;

    progress.start_known_endpoint(|| ("Indexing tiles.",tiles.feature_count()));

    for (i,tile) in tiles.read_entities::<TileForPopulation>().enumerate() {
        let (fid,tile) = tile?;
        flow_sum += tile.water_flow;
        flow_max = flow_max.max(tile.water_flow);
        area_sum += tile.area;
        work_queue.push(fid);
        tile_map.insert(fid,tile);
        progress.update(|| i);

    }

    progress.finish(|| "Tiles indexed");
    
    let flow_mean = flow_sum/work_queue.len() as f64;
    let area_mean = area_sum/work_queue.len() as f64;
    let flow_divisor = flow_max - flow_mean;

    let total_work = work_queue.len();
    progress.start_known_endpoint(|| ("Calculating population.",total_work));
    while let Some(fid) = work_queue.pop() {
        let (habitability,population) = {
            let tile = tile_map.get(&fid).unwrap(); // should exist, otherwise it wouldn't have been mapped.
            let mut suitability = if tile.lake_type.is_some() {
                0.0
            } else {
                *biome_map.get(&tile.biome).ok_or_else(|| CommandError::UnknownBiome(tile.biome.clone()))? as f64
            };
            if suitability > 0.0 {
                if flow_mean > 0.0 {
                    suitability += ((tile.water_flow - flow_mean)/flow_divisor).clamp(0.0,1.0) * 250.0; // big rivers are nice.
                }
                suitability -= (tile.elevation_scaled - 50) as f64/5.0; // low elevation is preferred
                if tile.shore_distance == 1 {
                    if tile.water_flow > estuary_threshold {
                        suitability += 15.0 // estuaries are liked
                    }
                    if let Some(water_cell) = tile.closest_water {
                        if let Some(water_cell) = tile_map.get(&(water_cell as u64)) {
                            if let Some(lake_type) = &water_cell.lake_type {
                                match lake_type {
                                    LakeType::Fresh => suitability += 30.0,
                                    LakeType::Salt => suitability += 10.0,
                                    LakeType::Frozen => suitability += 1.0,
                                    LakeType::Pluvial => suitability -= 2.0,
                                    LakeType::Dry => suitability -= 5.0,
                                    LakeType::Marsh => suitability += 5.0,
                                }
                            } else if water_cell.is_ocean {
                                suitability += 5.0;
                                if let Some(1) = tile.water_count {
                                    // since it's a land cell bordering a single cell on the ocean, that single cell is a small bay, which
                                    // probably makes a good harbor.
                                    suitability += 20.0
                                }
                            }
                        }

                    }
                }
                let habitability = suitability / 5.0; // FUTURE: I don't know why 5, but that's what AFMG did.
                // AFMG Just shows population in thousands, I'm actually going to have more precision, just for looks.
                let population = (((habitability * tile.area)/area_mean) * 1000.0).floor() as i32;
                (habitability,population)
            } else {
                (0.0,0)
            }
        };

        let tile = tile_map.get_mut(&fid).unwrap();
        tile.habitability = habitability;
        tile.population = population;

        progress.update(|| total_work - work_queue.len());

    }

    progress.finish(|| "Population calculated.");

    progress.start_known_endpoint(|| ("Writing population.",tile_map.len()));

    for (i,(fid,tile)) in tile_map.iter().enumerate() {

        if let Some(mut feature) = tiles.feature_by_id(&fid) {

            feature.set_habitability(tile.habitability)?;

            feature.set_population(tile.population)?;

            tiles.update_feature(feature)?;

        }

        progress.update(|| i)

    }

    progress.finish(|| "Population written.");


/*

* while fid = work_queue.pop:
  * let habitability = 0;
  * let population = 0;
  * lifetime block:
    * let tile = tile_map.get(fid)
    * let suitability = biomes_data.get(tile.biome) or error
    * if suitability > 0:
      * if flow_mean > 0:
        * suitability += ((tile.water_flow - flow_mean)/flow_divisor).clamp(0,1) * 250; // TODO: Is there a number I can just multiply by here?
      * suitability -= (tile.elevation_scaled - 50) / 5 -- low elevation is better
      * if cell.shore_distance == 1:
        * if cell.water_flow > estuary_threshold: suitability += 15 -- estuary
        * if cell.closest_water 
          * if tile_map[cell.closest_water].lake_type:
            * match lake_type
              * lake_type is fresh: suitability += 30
              * salt: suitability += 10
              * frozen: suitability += 1
              * pluvial or marsh: suitability -= 2
              * dry: suitability -= 5
          * else if tile_map[cell.closest_water].is_ocean:
            * suitability += 5
            * if cell.water_count == 1: suitability += 20 -- this means it's a single cell of ocean, which implies a small bay, which could be a harbor
      * habitability = suitability / 5
      * population = (habitability * tile.area) / area_mean
  * let tile = tile_map.get_mut(&fid)
    * tile.habitability = habitability;
    * tile.population = population;
* Write tile_map to layer.
 */    
    Ok(())
}
