use core::cmp::Ordering;

use crate::world_map::TileForWaterflow;
use crate::errors::CommandError;
use crate::world_map::TileForWaterFill;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::EntityIndex;
use crate::world_map::TileSchema;
use super::tiles::find_lowest_tile;
use crate::world_map::Neighbor;

pub(crate) struct WaterFlowResult  { 
    pub(crate) tile_map: EntityIndex<TileSchema,TileForWaterFill>, 
    pub(crate) lake_queue: Vec<(u64,f64)> 
}

pub(crate) fn generate_water_flow<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<WaterFlowResult,CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // from the AFMG code, this is also done in calculating precipitation. I'm wondering if it's unscaling the precipitation somehow?
    let cells_number_modifier = (layer.feature_count() as f64 / 10000.0).powf(0.25);

    let mut tile_list = Vec::new();
    let mut lake_queue = Vec::new();

    let mut tile_map = layer.read_features().into_entities_index_for_each::<_,TileForWaterflow,_>(|fid,tile| {
        if !tile.grouping.is_ocean() {
            // pushing the elevation onto here is easier than trying to map out the elevation during the sort, 
            tile_list.push((*fid,tile.elevation));
        }

        Ok(())

    },progress)?;
    
    // sort tile list so the highest is first.
    tile_list.sort_by(|(_,a),(_,b)| // FUTURE: could use sort by key if I conver the values to OrderedFloats.
        if a > b {
            Ordering::Less
        } else if a < b {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    );

    for (fid,elevation) in tile_list.iter().watch(progress,"Calculating initial flow.","Flow calculated.") {
        let entity = tile_map.try_get(fid)?;
        let water_flow = entity.water_flow + entity.precipitation / cells_number_modifier;
        let (lowest,lowest_elevation) = find_lowest_tile(entity,&tile_map,|t| {
            match t {
                Some((t,_)) => t.elevation,
                // water always flows off the map
                None => f64::NEG_INFINITY,
            }
        }, |t| &t.neighbors)?;

        let (water_accumulation,flow_to) = if let Some(lowest_elevation) = lowest_elevation {

            if &lowest_elevation < elevation {
                let neighbor_flow = water_flow/lowest.len() as f64;
                //println!("flowing {} to {} neighbors",neighbor_flow,lowest.len());
                for neighbor in &lowest {
                    match neighbor {
                        Neighbor::Tile(neighbor) | Neighbor::CrossMap(neighbor,_) => {
                            let neighbor = tile_map.try_get_mut(neighbor)?;
                            neighbor.water_flow += neighbor_flow;
                        }
                        Neighbor::OffMap(_) => (),
                    } // else it just disappears off the map
                }
                (0.0,lowest)
            } else {
                lake_queue.push((*fid,water_flow));
                (water_flow,Vec::new())
            }

        } else { // else there are... no neighbors? for some reason? I'm not going to start a lake, though.
            (water_flow,Vec::new())
        };

        let tile = tile_map.try_get_mut(fid)?; 
        tile.water_flow = water_flow;
        tile.water_accumulation += water_accumulation;
        tile.flow_to = flow_to;

    }


    for (fid,tile) in tile_map.iter().watch(progress,"Writing flow.","Flow written.") {
        if let Some(mut working_feature) = layer.feature_by_id(*fid) {

            working_feature.set_water_flow(tile.water_flow)?;
            working_feature.set_water_accumulation(tile.water_accumulation)?;
            working_feature.set_flow_to(&tile.flow_to)?;

            layer.update_feature(working_feature)?;
        }


    }

    Ok(WaterFlowResult {
        tile_map: tile_map.into_iter().map(|(k,v)| (k,v.into())).collect(),
        lake_queue,
    })


}
