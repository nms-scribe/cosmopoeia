use std::cmp::Reverse;
use std::collections::HashMap;

use priority_queue::PriorityQueue;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileForWaterDistance;
use crate::errors::CommandError;

pub(crate) fn generate_water_distance<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.edit_tile_layer()?;

    let mut queue = Vec::new();
    let mut land_queue = PriorityQueue::new();
    let mut water_queue = PriorityQueue::new();

    let mut tile_map = tiles.read_features().to_entities_index_for_each::<_,TileForWaterDistance,_>(|fid,_| {
        queue.push(*fid);
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

        for (neighbor_fid,_) in &tile.neighbors {
            let neighbor = tile_map.try_get(&neighbor_fid)?;
            if is_land && (neighbor.grouping.is_water()) {

                on_shore = true;
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
                *water_count.get_or_insert_with(|| 0) += 1;
            } else if !is_land && !neighbor.grouping.is_water() {

                on_shore = true;
            }
        }

        let mut tile = tile_map.try_get_mut(&fid)?;
        tile.water_count = water_count;
        tile.closest_water_tile_id = closest_water;
        if on_shore {
            if is_land {
                shore_distances.insert(fid,1);
                land_queue.push(fid,Reverse(1));
            } else {
                shore_distances.insert(fid,-1);
                water_queue.push(fid,Reverse(1));
            }
        }

    }

    // use the cost-expansion algorithm, as was done with expanding cultures, nations, subnations, etc. Except
    // there is no limit and cost is exactly 1 per tile.

    let mut land_queue = land_queue.watch_queue(progress, "Measuring land tiles.", "Land tiles measured.");

    while let Some((fid,priority)) = land_queue.pop() {

        let tile = tile_map.try_get(&fid)?;
        for (neighbor_id,_) in &tile.neighbors {

            let cost = priority.0 + 1;

            let replace_distance = if let Some(neighbor_cost) = shore_distances.get(&neighbor_id) {
                if &cost < neighbor_cost {
                    true
                } else {
                    false
                }
            } else {
                true
            };

            if replace_distance {
                shore_distances.insert(*neighbor_id,cost);
                land_queue.push(*neighbor_id, Reverse(cost));
            }

        }

    }

    let mut water_queue = water_queue.watch_queue(progress, "Measuring water tiles.", "Water tiles measured.");

    while let Some((fid,priority)) = water_queue.pop() {

        let tile = tile_map.try_get(&fid)?;
        for (neighbor_id,_) in &tile.neighbors {

            let cost = priority.0 + 1;

            let replace_distance = if let Some(neighbor_cost) = shore_distances.get(&neighbor_id) {
                if &cost < neighbor_cost {
                    true
                } else {
                    false
                }
            } else {
                true
            };

            if replace_distance {
                shore_distances.insert(*neighbor_id,-cost);
                water_queue.push(*neighbor_id, Reverse(cost));
            }

        }

    }

    for (fid,tile) in tile_map.into_iter().watch(progress, "Writing data.", "Data written.") {

        let mut feature = tiles.try_feature_by_id(&fid)?;
        let shore_distance = shore_distances.remove(&fid).unwrap(); // There should be no reason the shore_distance wasn't generated for the tile
        feature.set_shore_distance(shore_distance)?;
        feature.set_harbor_tile_id(tile.closest_water_tile_id)?;
        feature.set_water_count(tile.water_count)?;
        tiles.update_feature(feature)?;


    }


    Ok(())
}