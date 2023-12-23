use core::cmp::Reverse;
use std::collections::HashMap;

use priority_queue::PriorityQueue;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::WorldMapTransaction;
use crate::world_map::tile_layer::TileForWaterDistance;
use crate::errors::CommandError;
use crate::world_map::fields::NeighborAndDirection;
use crate::world_map::fields::Neighbor;

pub(crate) fn generate_water_distance<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    let mut tiles = target.edit_tile_layer()?;

    let mut queue = Vec::new();
    let mut land_queue = PriorityQueue::new();
    let mut water_queue = PriorityQueue::new();

    let mut tile_map = tiles.read_features().into_entities_index_for_each::<_,TileForWaterDistance,_>(|fid,_| {
        queue.push(fid.clone());
        Ok(())
    }, progress)?;

    let mut queue = queue.watch_queue(progress, "Finding shoreline tiles.", "Shoreline tiles found.");
    let mut shore_distances = HashMap::new();

    while let Some(fid) = queue.pop() {

        let mut on_shore = false;
        let mut closest_water = None;
        let mut water_distance = None;
        let mut water_count = None;

        let tile = tile_map.try_get(&fid)?;
        let is_land = !tile.grouping.is_water();

        for NeighborAndDirection(neighbor_fid,_) in &tile.neighbors {
            match neighbor_fid {
                neighbor_tile @ (Neighbor::Tile(neighbor_fid) | Neighbor::CrossMap(neighbor_fid, _)) => {
                    let neighbor = tile_map.try_get(neighbor_fid)?;
                    if is_land && (neighbor.grouping.is_water()) {
    
                        on_shore = true;
                        let neighbor_water_distance = match neighbor_tile {
                            Neighbor::Tile(_) => tile.site.distance(&neighbor.site,&world_shape),
                            Neighbor::CrossMap(_, _) => tile.site.distance(&neighbor.site.across_antimeridian(&tile.site),&world_shape),
                            Neighbor::OffMap(_) => unreachable!("neighbor_tile should only be set if Tile or CrossMap"),
                        };

                        if let Some(old_water_distance) = water_distance {
                            if neighbor_water_distance < old_water_distance {
                                water_distance = Some(neighbor_water_distance);
                                closest_water = Some(neighbor_tile.clone());
                            }
                        } else {
                            water_distance = Some(neighbor_water_distance);
                            closest_water = Some(neighbor_tile.clone());
                        }
                        *water_count.get_or_insert(0) += 1;
                    } else if !is_land && !neighbor.grouping.is_water() {
    
                        on_shore = true;
                    }
                }
                Neighbor::OffMap(_) => (),
            } // else ignore off the map, it's as if there were no neighbors

        }

        let edit_tile = tile_map.try_get_mut(&fid)?;
        edit_tile.water_count = water_count;
        edit_tile.closest_water_tile_id = closest_water;
        if on_shore {
            if is_land {
                _ = shore_distances.insert(fid.clone(),1);
                _ = land_queue.push(fid,Reverse(1));
            } else {
                _ = shore_distances.insert(fid.clone(),-1);
                _ = water_queue.push(fid,Reverse(1));
            }
        }

    }

    // use the cost-expansion algorithm, as was done with expanding cultures, nations, subnations, etc. Except
    // there is no limit and cost is exactly 1 per tile.

    let mut land_queue = land_queue.watch_queue(progress, "Measuring land tiles.", "Land tiles measured.");

    while let Some((fid,priority)) = land_queue.pop() {

        let tile = tile_map.try_get(&fid)?;
        for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {
            match neighbor_id {
                Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {

                    let cost = priority.0 + 1;

                    let replace_distance = if let Some(neighbor_cost) = shore_distances.get(neighbor_id) {
                        &cost < neighbor_cost
                    } else {
                        true
                    };

                    if replace_distance {
                        _ = shore_distances.insert(neighbor_id.clone(),cost);
                        land_queue.push(neighbor_id.clone(), Reverse(cost));
                    }
                }
                Neighbor::OffMap(_) => (),
            } // else ignore off-the-map as if there were no tile

        }

    }

    let mut water_queue = water_queue.watch_queue(progress, "Measuring water tiles.", "Water tiles measured.");

    while let Some((fid,priority)) = water_queue.pop() {

        let tile = tile_map.try_get(&fid)?;
        for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {

            match neighbor_id {
                Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                    let cost = priority.0 + 1;

                    let replace_distance = if let Some(neighbor_cost) = shore_distances.get(neighbor_id) {
                        &cost < neighbor_cost
                    } else {
                        true
                    };

                    if replace_distance {
                        _ = shore_distances.insert(neighbor_id.clone(),-cost);
                        water_queue.push(neighbor_id.clone(), Reverse(cost));
                    }
                }
                Neighbor::OffMap(_) => (),
            } // else ignore off-the-map as if there were no tile

        }

    }

    for (fid,tile) in tile_map.into_iter().watch(progress, "Writing data.", "Data written.") {

        let mut feature = tiles.try_feature_by_id(&fid)?;
        let shore_distance = shore_distances.remove(&fid).expect("Why wouldn't this value have been generated for the tile?");
        feature.set_shore_distance(&shore_distance)?;
        feature.set_harbor_tile_id(&tile.closest_water_tile_id)?;
        feature.set_water_count(&tile.water_count)?;
        tiles.update_feature(feature)?;


    }


    Ok(())
}