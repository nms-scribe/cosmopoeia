use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TypedFeature;
use crate::world_map::TileEntityForWaterDistance;
use crate::world_map::TileEntityForWaterDistanceNeighbor;
use crate::world_map::TileEntityForWaterDistanceOuter;
use crate::world_map::TileEntityForWaterDistanceOuterNeighbor;
use crate::errors::CommandError;

pub(crate) fn generate_water_distance<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.edit_tile_layer()?;

    let mut queue = Vec::new();
    let mut next_queue = Vec::new();

    progress.start_known_endpoint(|| ("Indexing data.",tiles.feature_count() as usize));

    for (i,feature) in tiles.read_features().enumerate() {
        let fid = feature.fid()?;
        queue.push(fid);
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

        let tile = tiles.try_entity_by_id::<TileEntityForWaterDistance>(&fid)?;
        let is_land = !tile.terrain.is_water();

        for (neighbor_fid,_) in &tile.neighbors {
            let neighbor = tiles.try_entity_by_id::<TileEntityForWaterDistanceNeighbor>(&neighbor_fid)?;
            if is_land && (neighbor.terrain.is_water()) {
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
            } else if !is_land && !neighbor.terrain.is_water() {
                shore_distance.get_or_insert_with(|| -1);
            }
        }

        let mut tile = tiles.try_feature_by_id(&fid)?;
        if (water_count > 0) || closest_water.is_some() || shore_distance.is_some() {

            if water_count > 0 {
                tile.set_water_count(Some(water_count))?;
            }
            tile.set_closest_water(closest_water.map(|n| n as i64))?;

            if let Some(shore_distance) = shore_distance {
                tile.set_shore_distance(shore_distance)?;
            }

            processed += 1;
    
        } else {
            // we couldn't calculate, but I need to fill in the blanks
            tile.set_water_count(None)?;
            tile.set_closest_water(None)?;
            // we'll do shore_distance later, it will be taken care of.
            
            
            //so push it on the queue for the next iteration.
            next_queue.push(fid);
        }
        tiles.update_feature(tile)?;

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
                let tile = tiles.try_entity_by_id::<TileEntityForWaterDistanceOuter>(&fid)?; 

                if tile.shore_distance.is_none() {
                    let is_land = !tile.terrain.is_water();
    
                    for (neighbor_fid,_) in &tile.neighbors {
                        let neighbor = tiles.try_entity_by_id::<TileEntityForWaterDistanceOuterNeighbor>(neighbor_fid)?; 
                        
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
                    
                } else {
                    // else we already have the shore distance, so don't do anything.
                    continue;
                }
    
            }

            // apply any changed shore distance
            if let Some(shore_distance) = shore_distance {
                let mut tile = tiles.try_feature_by_id(&fid)?;
                tile.set_shore_distance(shore_distance)?;
                tiles.update_feature(tile)?;
                
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

    Ok(())
}