use std::collections::HashMap;

use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileEntityForWaterDistance;
use crate::errors::CommandError;

pub(crate) fn generate_water_distance<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    let mut tile_map = HashMap::new();
    let mut queue = Vec::new();
    let mut next_queue = Vec::new();

    progress.start_known_endpoint(|| ("Indexing data.",layer.feature_count() as usize));

    for (i,data) in layer.read_entities::<TileEntityForWaterDistance>().enumerate() {
        let (fid,entity) = data?;
        queue.push(fid);
        tile_map.insert(fid, entity);
        progress.update(|| i);

    }
    progress.finish(|| "Data indexed.");

    let total_queue = queue.len();
    let mut processed = 0;
    progress.start_known_endpoint(|| ("Finding tiles at distance 1.",total_queue));

    while let Some(fid) = queue.pop() {

        let mut shore_distance = None;
        let mut closest_water = None;
        let mut water_distance = None;
        let mut water_count = 0;

        {
            let tile = tile_map.get(&fid).unwrap(); // We built the list from the same source, so it should always exist
            let is_land = !tile.is_ocean && tile.lake_elevation.is_none();

            for (neighbor_fid,_) in &tile.neighbors {
                if let Some(neighbor) = tile_map.get(&neighbor_fid) {
                    if is_land && (neighbor.is_ocean || neighbor.lake_elevation.is_some()) {
                        shore_distance.get_or_insert_with(|| 1);
                        let neighbor_water_distance = tile.site.distance(&neighbor.site);
                        if let Some(old_water_distance) = water_distance {
                            if neighbor_water_distance < old_water_distance {
                                water_distance = Some(neighbor_water_distance);
                                closest_water = Some(*neighbor_fid);
                            }
                        } else {
                            water_distance = Some(neighbor_water_distance);
                            closest_water = Some(*neighbor_fid);
                        }
                        water_count += 1;
                    } else if !is_land && !neighbor.is_ocean && neighbor.lake_elevation.is_none() {
                        shore_distance.get_or_insert_with(|| -1);
                    }
                }
            }
    
    
        }

        if (water_count > 0) || closest_water.is_some() || shore_distance.is_some() {
            if let Some(tile) = tile_map.get_mut(&fid) {
                if water_count > 0 {
                    tile.water_count = Some(water_count);
                }
                if let Some(closest_water) = closest_water {
                    tile.closest_water = Some(closest_water)
                }
                if let Some(shore_distance) = shore_distance {
                    tile.shore_distance = Some(shore_distance)
                }

            }
            processed += 1;
    
        } else {
            // we couldn't calculate, so push it on the queue for the next iteration.
            next_queue.push(fid);
        }

        progress.update(|| processed);
    }

    // now iterate outwards from there, if the tile is not marked, but it has a neighbor that was marked for the previous distance, then
    // it is marked with the next distance. There might be a more efficient algorithm, but I'd have to think about it. I know I can't calculate
    // the distance n until distance n-1 is calculated, or I might mismark it.
    let mut queue = next_queue;
    for calc_distance in 2.. {
        let mut next_queue = Vec::new();
        progress.message(|| format!("Finding tiles at distance {}.",calc_distance));
        while let Some(fid) = queue.pop() {

            let mut shore_distance = None;
            {
                let tile = tile_map.get(&fid).unwrap(); // In theory I won't have put something on the queue if it wasn't already in the tile map.

                if tile.shore_distance.is_none() {
                    let is_land = !tile.is_ocean && tile.lake_elevation.is_none();
    
                    for (neighbor_fid,_) in &tile.neighbors {
                        if let Some(neighbor) = tile_map.get(neighbor_fid) {
                            if let Some(neighbor_shore_distance) = neighbor.shore_distance {
                                if is_land {
                                    if neighbor_shore_distance == (calc_distance - 1) {
                                        shore_distance.get_or_insert_with(|| calc_distance);
                                    }
                                } else {
                                    if neighbor_shore_distance == (-calc_distance + 1) {
                                        shore_distance.get_or_insert_with(|| -calc_distance);
                                    }
                                }
                            }
    
                        }
    
                    }
                    
                } else {
                    // else we already have the shore distance, so don't do anything.
                    continue;
                }
    
            }

            // apply any changed shore distance
            if shore_distance.is_some() {
                tile_map.get_mut(&fid).unwrap().shore_distance = shore_distance;
                processed += 1;
                progress.update(|| processed);
            } else {
                next_queue.push(fid)
            }

        }

        if next_queue.len() == 0 {
            progress.update(|| processed);
            break;
        }
        queue = next_queue;

    }
    progress.finish(|| "Found distances for tiles.");



    // update the distances:
    progress.start_known_endpoint(|| ("Writing shore distances.",tile_map.len()));

    for (i,(fid,tile)) in tile_map.iter().enumerate() {

        if (tile.water_count.is_some()) || tile.closest_water.is_some() || tile.shore_distance.is_some() {

            if let Some(mut feature) = layer.feature_by_id(fid) {

                if let Some(water_count) = tile.water_count {
                    feature.set_water_count(Some(water_count))?;
                }
                if let Some(closest_water) = tile.closest_water {
                    feature.set_closest_water(Some(closest_water as i64))?;
                }
                if let Some(shore_distance) = tile.shore_distance {
                    feature.set_shore_distance(shore_distance)?;
                }

                layer.update_feature(feature)?;


            }
    
    
        }



        progress.update(|| i);
    }

    progress.finish(|| "Shore distances written.");


    Ok(())
}