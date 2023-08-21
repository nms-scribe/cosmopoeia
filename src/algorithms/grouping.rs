use std::collections::HashSet;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileEntityForGroupingCalc;
use crate::world_map::Grouping;
use crate::errors::CommandError;

pub(crate) fn calculate_grouping<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    // NOTE: By this time, the grouping type "Ocean" is already set.
    let mut tiles = target.edit_tile_layer()?;
    let tile_count = tiles.feature_count();

    // we just want land tiles
    let mut table = tiles.read_features().to_entities_index::<_,TileEntityForGroupingCalc>(progress)?;

    let mut groupings = Vec::new();
    let mut ocean = HashSet::new();
    let mut next_grouping_id = 1 as i64..;

    // I can't watch anything here, because this isn't really a queue and I'm only picking the first key off the iterator every time I create a key iterator.
    // I could create a WatchableHashMap, but I don't foresee this process being used a lot.
    let original_table_len = table.len();
    progress.start_known_endpoint(|| ("Calculating group types.",original_table_len));

    // pop the next one off of the table.
    // TODO: The 'watch' isn't going to work here since it only watches when it picks.
    while let Some(tile) = table.keys().next().cloned().and_then(|first| table.remove(&first)) {
        progress.update(|| original_table_len - table.len());


        // NOTE: I previously considered using the lake_id for the lake grouping_id, and getting rid of that field.
        // However, there is no guarantee that it won't (and almost assured it won't) overlap with the other id
        // numbers. Keeping them separate will simplify some algorithms, as otherwise I'd have to check both the
        // grouping and grouping_id to make sure things are the same.
        let grouping_id = next_grouping_id.next().unwrap();
        let mut group = vec![tile.fid];
        let mut neighbors = tile.neighbors.clone();

        let grouping_type = if tile.grouping.is_ocean() {
            // track this as an ocean, so we can tell if land borders an ocean later.
            ocean.insert(tile.fid);

            // trace all of it's neighbors until we hit something that isn't part of the same thing.
            while let Some((neighbor_fid,_)) = neighbors.pop() {
                if let Some(neighbor) = table.get(&neighbor_fid) {
                    if neighbor.grouping.is_ocean() {
                        // it's part of the same group
                        ocean.insert(neighbor_fid); // insert it into oceans so we can check whether an island is a lake island or not.
                        neighbors.extend(neighbor.neighbors.iter());
                        table.remove(&neighbor_fid);
                        progress.update(|| original_table_len - table.len());
                        group.push(neighbor_fid);
                    }
    
                } // else it's been processed already, either in this group or in another group
    
            }


            Grouping::Ocean
        } else {
            let mut found_ocean_neighbor = false;
            let is_lake = tile.lake_id;
    
            // trace all of it's neighbors until we hit something that isn't part of the same thing.
            while let Some((neighbor_fid,_)) = neighbors.pop() {
                if let Some(neighbor) = table.get(&neighbor_fid) {
                    if neighbor.grouping.is_ocean() {
                        // it's not part of the group, but we now know this body is next to the ocean
                        found_ocean_neighbor = true
                    } else if is_lake == neighbor.lake_id {
                        // it's the same kind of non-ocean grouping, so add it to the current group and keep looking at it's neighbors
                        neighbors.extend(neighbor.neighbors.iter());
                        table.remove(&neighbor_fid);
                        progress.update(|| original_table_len - table.len());
                        group.push(neighbor_fid);
                    }
    
                } else if ocean.contains(&neighbor_fid) {
                    // the reason it's not found is because it was already processed as an ocean, so, we know this body is next to the ocean.
                    found_ocean_neighbor = true;
                } // else it's been processed already, either in this group or in another group
    
            }

            let group_len = group.len();
    
            if is_lake.is_some() {
                Grouping::Lake
            } else {
                if !found_ocean_neighbor {
                    Grouping::LakeIsland // even if it's continent size
                    // FUTURE: There is a possible error if there are no oceans on the map at all. While we could
                    // check oceans.len, that will cause every lake_island to be a continent, even if it actually is 
                    // a lake_island. We could have another flag for having found only lake neighbors, but that's just
                    // going to turn the whole thing into continent.
                    // -- The only solution is to know if we found a tile on the border of the map, and if we have one of those
                    // then it's a continent.
                } else if group_len > (tile_count / 100) { 
                    // NOTE: AFMG had 10 here. That didn't make enough large islands into continents on my map
                    // FUTURE: The comparsion shouldn't be made against the tile count, but against a potential
                    // tile count if the map extended to the entire world.
                    // FUTURE: Alternatively, we could have a "Scale" parameter which would be required for calculating this.
                    Grouping::Continent
                } else if group_len > (tile_count / 1000) {
                    Grouping::Island
                } else {
                    Grouping::Islet // Except it's not really that small either, but what the heck it will work.
                }
            }

        };

        groupings.push((grouping_type,grouping_id,group));
    
                

    }

    progress.finish(|| "Grouping types calculated.");

    for (grouping,grouping_id,group) in groupings.iter().watch(progress,"Writing grouping types.","Grouping types written.") {
        for tile in group {
            let mut feature = tiles.try_feature_by_id(&tile)?;
            feature.set_grouping(&grouping)?;
            feature.set_grouping_id(*grouping_id)?;
            tiles.update_feature(feature)?;
        }
    }

    Ok(())
}