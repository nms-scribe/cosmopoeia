use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileEntityForTerrainCalc;
use crate::world_map::Terrain;
use crate::errors::CommandError;

pub(crate) fn calculate_terrain<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.edit_tile_layer()?;
    let tile_count = tiles.feature_count();

    // we just want land tiles
    let mut table = tiles.read_features().to_entities_index::<_,TileEntityForTerrainCalc>(progress)?;

    let table_len = table.len();
    progress.start_known_endpoint(|| ("Calculating terrain types.",table_len));

    let mut terrains = Vec::new();
    let mut ocean = Vec::new();
    let mut changes = 0;

    // pop the next one off of the table.
    while let Some(tile) = table.keys().next().cloned().and_then(|first| table.remove(&first)) {

        if tile.terrain.is_ocean() {
            ocean.push(tile.fid)
        } else {
            let mut found_ocean_neighbor = false;
            let is_lake = tile.lake_id;
            let mut neighbors = tile.neighbors.clone();
            let mut group = vec![tile.fid];
    
            while let Some((neighbor_fid,_)) = neighbors.pop() {
                if let Some(neighbor) = table.get(&neighbor_fid) {
                    if neighbor.terrain.is_ocean() {
                        // it's not part of the group, but we now know this body is next to the ocean
                        found_ocean_neighbor = true
                    } else if is_lake == neighbor.lake_id {
                        // it's the same kind of non-ocean terrain, so add it to the current terrain group and keep looking at it's neighbors
                        neighbors.extend(neighbor.neighbors.iter());
                        table.remove(&neighbor_fid);
                        group.push(neighbor_fid);
                    }
    
                } else if ocean.contains(&neighbor_fid) {
                    // the reason it's not found is because it was already processed as an ocean, so, we know this body is next to the ocean.
                    found_ocean_neighbor = true;
                } // else it's been processed already, either in this group or in another group
    
            }

            let group_len = group.len();
    
            let terrain = if is_lake.is_some() {
                Terrain::Lake
            } else {
                if !found_ocean_neighbor {
                    Terrain::LakeIsland // even if it's continent size
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
                    Terrain::Continent
                } else if group_len > (tile_count / 1000) {
                    Terrain::Island
                } else {
                    Terrain::Islet // Except it's not really that small either, but what the heck it will work.
                }
            };

            changes += group_len;

            terrains.push((terrain,group));
    
                
        }

        progress.update(|| table_len - table.len())

    }

    progress.finish(|| "Terrain types calculated.");

    progress.start_known_endpoint(|| ("Writing terrain types.",changes));

    let mut changes_written = 0;

    for (terrain,group) in terrains {
        for tile in group {
            let mut feature = tiles.try_feature_by_id(&tile)?;
            feature.set_terrain(&terrain)?;
            tiles.update_feature(feature)?;
            changes_written += 1;
            progress.update(|| changes_written);
        }
    }

    progress.finish(|| "Terrain types written.");

    Ok(())
}