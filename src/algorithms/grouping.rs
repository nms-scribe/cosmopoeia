use std::collections::HashSet;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileForGroupingCalc;
use crate::world_map::Grouping;
use crate::errors::CommandError;

pub(crate) fn calculate_grouping<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    // NOTE: By this time, the grouping type "Ocean" is already set.
    let mut tiles = target.edit_tile_layer()?;
    let tile_count = tiles.feature_count();

    // we just want land tiles
    let table = tiles.read_features().to_entities_index::<_,TileForGroupingCalc>(progress)?;

    let mut groupings = Vec::new();
    let mut ocean = HashSet::new();
    let mut next_grouping_id = 1 as u64..;

    let mut table = table.watch_queue(progress,"Calculating group types.","Group types calculated.");
    // pop the next one off of the table.
    while let Some((fid,tile)) = table.pop() {


        // NOTE: I previously considered using the lake_id for the lake grouping_id, and getting rid of that field.
        // However, there is no guarantee that it won't (and almost assured it won't) overlap with the other id
        // numbers. Keeping them separate will simplify some algorithms, as otherwise I'd have to check both the
        // grouping and grouping_id to make sure I'm looking in the right place.
        let grouping_id = next_grouping_id.next().expect("Why would an unlimited range fail?");
        let mut group = vec![fid];
        let mut neighbors = tile.neighbors.clone();

        let grouping_type = if tile.grouping.is_ocean() {
            // track this as an ocean, so we can tell if land borders an ocean later.
            ocean.insert(fid);

            // trace all of it's neighbors until we hit something that isn't part of the same thing.
            while let Some((neighbor_fid,_)) = neighbors.pop() {
                if let Some(neighbor) = table.maybe_get(&neighbor_fid) {
                    if neighbor.grouping.is_ocean() {
                        // it's part of the same group
                        ocean.insert(neighbor_fid); // insert it into oceans so we can check whether an island is a lake island or not.
                        neighbors.extend(neighbor.neighbors.iter());
                        table.try_remove(&neighbor_fid)?;
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
                if let Some(neighbor) = table.maybe_get(&neighbor_fid) {
                    if neighbor.grouping.is_ocean() {
                        // it's not part of the group, but we now know this body is next to the ocean
                        found_ocean_neighbor = true
                    } else if is_lake == neighbor.lake_id {
                        // it's the same kind of non-ocean grouping, so add it to the current group and keep looking at it's neighbors
                        neighbors.extend(neighbor.neighbors.iter());
                        table.try_remove(&neighbor_fid)?;
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
                    // NOTE: There is a possible error if there are no oceans on the map at all. While we could
                    // check oceans.len, that will cause every lake_island to be a continent, even if it actually is 
                    // a lake_island. We could have another flag for having found only lake neighbors, but that's just
                    // going to turn the whole thing into continent.
                    // -- The only solution is to know if we found a tile on the border of the map, and if we have one of those
                    // then it's a continent.
                } else if group_len > (tile_count / 100) { 
                    // NOTE: AFMG had 10 here. That didn't make enough large islands into continents on my map
                    // NOTE: The comparsion shouldn't be made against the tile count, but against a potential
                    // tile count if the map extended to the entire world.
                    // NOTE: Alternatively, we could have a "Scale" parameter which would be required for calculating this.
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