use std::collections::HashMap;
use std::cmp::Ordering;

use crate::world_map::TileForWaterflow;
use crate::errors::CommandError;
use crate::world_map::TileForWaterFill;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;

pub(crate) fn generate_water_flow<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(HashMap<u64,TileForWaterFill>,Vec<(u64,f64)>),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // from the AFMG code, this is also done in calculating precipitation. I'm wondering if it's unscaling the precipitation somehow?
    let cells_number_modifier = ((layer.feature_count() / 10000) as f64).powf(0.25);

    // TODO: I've had good luck with going directly to the database with population and shore_distance, so maybe I don't need to map it?
    let mut tile_map = HashMap::new();
    let mut tile_list = Vec::new();
    let mut lake_queue = Vec::new();

    for data in layer.read_features().into_entities::<TileForWaterflow>().watch(progress,"Indexing tiles.","Tiles indexed.") {
        let (fid,entity) = data?;
        if !entity.grouping.is_ocean() {
            // pushing the elevation onto here is easier than trying to map out the elevation during the sort, 
            // FUTURE: Although it takes about twice as much memory, which could be important in the future.
            tile_list.push((fid,entity.elevation));
        }
        tile_map.insert(fid, entity);

    }

    // sort tile list so the highest is first.
    tile_list.sort_by(|(_,a),(_,b)| 
        if a > b {
            Ordering::Less
        } else if a < b {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    );

    for (fid,elevation) in tile_list.iter().watch(progress,"Calculating initial flow.","Flow calculated.") {
        let (water_flow,lowest,lowest_elevation) = if let Some(entity) = tile_map.get(fid) {
            let water_flow = entity.water_flow + entity.precipitation / cells_number_modifier;
            let (lowest,lowest_elevation) = super::tiles::find_lowest_neighbors(entity,&tile_map);

            (water_flow,lowest,lowest_elevation)

        } else {
            (0.0,vec![],None)
        };

        let (water_accumulation,flow_to) = if let Some(lowest_elevation) = lowest_elevation {

            if &lowest_elevation < elevation {
                let neighbor_flow = water_flow/lowest.len() as f64;
                //println!("flowing {} to {} neighbors",neighbor_flow,lowest.len());
                for neighbor in &lowest {
                    if let Some(neighbor) = tile_map.get_mut(&neighbor) {
                        neighbor.water_flow += neighbor_flow;
                    }
                }
                (0.0,lowest)
            } else {
                lake_queue.push((*fid,water_flow));
                (water_flow,Vec::new())
            }

        } else { // else there are... no neighbors? for some reason? I'm not going to start a lake, though.
            (water_flow,Vec::new())
        };

        if let Some(tile) = tile_map.get_mut(&fid) {
            tile.water_flow = water_flow;
            tile.water_accumulation += water_accumulation;
            tile.flow_to = flow_to;
        }

    }


    for (fid,tile) in tile_map.iter().watch(progress,"Writing flow.","Flow written.") {
        if let Some(mut working_feature) = layer.feature_by_id(&fid) {

            working_feature.set_water_flow(tile.water_flow)?;
            working_feature.set_water_accumulation(tile.water_accumulation)?;
            working_feature.set_flow_to(&tile.flow_to)?;

            layer.update_feature(working_feature)?;
        }


    }

    Ok((tile_map.into_iter().map(|(k,v)| (k,v.into())).collect(),lake_queue))





}
