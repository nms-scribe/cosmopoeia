use std::cmp::Ordering;

use crate::world_map::TileEntityForWaterFlow;

use crate::errors::CommandError;

use crate::world_map::TileEntityForWaterFill;

use std::collections::HashMap;

use crate::world_map::WorldMapTransaction;

use crate::progress::ProgressObserver;

pub(crate) fn generate_water_flow<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(HashMap<u64,TileEntityForWaterFill>,Vec<(u64,f64)>),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // from the AFMG code, this is also done in calculating precipitation. I'm wondering if it's unscaling the precipitation somehow?
    let cells_number_modifier = ((layer.feature_count() / 10000) as f64).powf(0.25);

    let mut tile_map = HashMap::new();
    let mut tile_list = Vec::new();
    let mut lake_queue = Vec::new();

    progress.start_known_endpoint(|| ("Indexing data.",layer.feature_count() as usize));

    for (i,data) in layer.read_entities::<TileEntityForWaterFlow>().enumerate() {
        let (fid,entity) = data?;
        if !entity.is_ocean {
            // pushing the elevation onto here is easier than trying to map out the elevation during the sort, 
            // FUTURE: Although it takes about twice as much memory, which could be important in the future.
            tile_list.push((fid,entity.elevation));
        }
        tile_map.insert(fid, entity);
        progress.update(|| i);

    }
    progress.finish(|| "Data indexed.");

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

    progress.start_known_endpoint(|| ("Calculating initial flow",tile_list.len()));

    for (i,(fid,elevation)) in tile_list.iter().enumerate() {
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

        progress.update(|| i);

    }

    progress.finish(|| "Flow calculated.");

    progress.start_known_endpoint(|| ("Writing flow",tile_map.len()));

    for (fid,tile) in &tile_map {
        if let Some(mut working_feature) = layer.feature_by_id(&fid) {

            working_feature.set_water_flow(tile.water_flow)?;
            working_feature.set_water_accumulation(tile.water_accumulation)?;
            working_feature.set_flow_to(&tile.flow_to)?;

            layer.update_feature(working_feature)?;
        }


    }

    progress.finish(|| "Flow written.");

    Ok((tile_map.into_iter().map(|(k,v)| (k,v.into())).collect(),lake_queue))





}
