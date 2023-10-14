use core::ops::RangeFrom;
use std::collections::HashSet;
use std::collections::HashMap;

use crate::world_map::water_layers::NewLake;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;
use crate::world_map::tile_layer::TileLayer;
use crate::world_map::fields::LakeType;
use crate::commands::OverwriteLakesArg;
use crate::commands::BezierScaleArg;
use crate::commands::LakeBufferScaleArg;
use crate::algorithms::tiles::find_lowest_tile;
use super::water_flow::WaterFlowResult;
use crate::typed_map::entities::EntityIndex;
use crate::world_map::tile_layer::TileSchema;
use crate::world_map::tile_layer::TileForWaterFill;
use crate::progress::QueueWatcher;
use crate::geometry::VariantArealGeometry;
use crate::geometry::MultiPolygon;
use crate::typed_map::features::TypedFeature;
use crate::world_map::fields::NeighborAndDirection;
use crate::world_map::fields::Neighbor;
use crate::typed_map::fields::IdRef;

struct Lake {
    elevation: f64,
    flow: f64,
    bottom_elevation: f64,
    spillover_elevation: f64,
    contained_tiles: Vec<IdRef>, 
    tile_temperatures: Vec<f64>,
    shoreline_tiles: Vec<(IdRef,Neighbor)>, // a bordering lake tile, the actual shoreline tile
    outlet_tiles: Vec<(IdRef,Neighbor)>, // from, to
}

impl Lake {

    pub(crate) fn dissolve_tiles(&self, layer: &mut TileLayer<'_,'_>) -> Result<VariantArealGeometry,CommandError> {

        let mut tiles = self.contained_tiles.iter();
        let first_tile = layer.try_feature_by_id(tiles.next().expect("Someone called dissolve_tiles on a Lake that didn't have any tiles."))?;
        let mut lake_geometry = first_tile.geometry()?.into();
        
        for tile in tiles {
            let tile = layer.try_feature_by_id(tile)?; 
            let tile: VariantArealGeometry = tile.geometry()?.into(); 
            lake_geometry = tile.union(&lake_geometry)?;
    
        }
        Ok(lake_geometry)
    }

    fn calc_temp_and_evap(&self) -> (f64,f64) {
        let lake_temp_sum: f64 = self.tile_temperatures.iter().sum();
        let lake_temp = lake_temp_sum / self.tile_temperatures.len() as f64;
        // This is taken from AFMG, where it says it was based on the Penman formula, except I don't see much relationship to the
        // equation described at https://en.wikipedia.org/wiki/Penman_equation
        let lake_evap = ((700.0 * 0.006f64.mul_add(self.elevation, lake_temp)) / 50.0 + 75.0) / (80.0 - lake_temp);
        let lake_evap = lake_evap * self.contained_tiles.len() as f64;
        (lake_temp,lake_evap)           

    }


    fn get_temp_evap_and_type(&self) -> (f64,f64,LakeType) {
        let (lake_temp,lake_evap) = self.calc_temp_and_evap();
        let flow_per_tile = self.flow / self.contained_tiles.len() as f64;
        let lake_type = if lake_temp < -3.0 {
            LakeType::Frozen
        } else if self.outlet_tiles.is_empty() {
            // NOTE: This was what AFMG did. It's based off of real equations I've seen elsewhere, but I don't
            // know where they come from. However...
            // if lake_evap > (flow_per_tile * 4.0) {
            //     LakeType::Dry
            // } else if lake_evap > flow_per_tile {
            //     LakeType::Salt
            // } else {
            //     LakeType::Fresh
            // }
            // ... Since I already took care of evaporating in determining the lake elevation, I feel that should
            // tell me if it's salty. For example, if it doesn't have any outlets at all, then there wasn't enough
            // flow to push it over the edge, so therefore evaporation is overcoming flow. Remember, I don't
            // want realism, just verisimilitude.
            if lake_evap > (flow_per_tile * 4.0) {
                LakeType::Dry
            } else if (self.bottom_elevation - self.elevation).abs() < f64::EPSILON {
                LakeType::Pluvial
            } else {
                LakeType::Salt
            }
        } else if (self.bottom_elevation - self.elevation).abs() < f64::EPSILON {
            LakeType::Marsh
        } else {
            LakeType::Fresh
        };    
        (lake_temp,lake_evap,lake_type)

    }


}

enum WaterFillTask {
    FillLake(IdRef, f64),
    AddToFlow(f64)
}



// this one is quite tight with generate_water_flow, it even shares some pre-initialized data.
pub(crate) fn generate_water_fill<Progress: ProgressObserver>(target: &mut WorldMapTransaction, water_flow_result: WaterFlowResult, lake_bezier_scale: &BezierScaleArg, lake_buffer_scale: &LakeBufferScaleArg, overwrite_layer: &OverwriteLakesArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tile_queue = water_flow_result.lake_queue.watch_queue(progress,"Filling lakes.","Lakes filled.");

    let mut tile_map = water_flow_result.tile_map;
    let mut next_lake_id = 1..;
    let mut lake_map = HashMap::new();

    while let Some((tile_fid,tile_accumulation)) = tile_queue.pop() {

        let tile = tile_map.try_get(&tile_fid)?; 
        // we don't bother with accumulation in ocean.
        if tile.grouping.is_ocean() {
            continue;
        }

        // if the tile has no accumulation, there's nothing to do:
        if tile_accumulation <= 0.0 {
            continue;
        }


        // figure out what we've got to do. I don't figure out the task until now in order to avoid having to do extra neighbor checks every time we fill.
        if let Some(task) = determine_water_fill_task(&tile_fid, tile, tile_accumulation, &tile_map, &mut next_lake_id, &mut tile_queue, &mut lake_map)? {
            match task {
                WaterFillTask::AddToFlow(accumulation) => {
                    let edit_tile = tile_map.try_get_mut(&tile_fid)?; 
                    edit_tile.water_flow += accumulation;
                    let mut feature = tiles_layer.try_feature_by_id(&tile_fid)?; 
                    
                    feature.set_water_flow(&edit_tile.water_flow)?;

                    tiles_layer.update_feature(feature)?;
    
                }
                WaterFillTask::FillLake(lake_id,accumulation) => {
                    let (new_lake,accumulation,delete_lakes) = if let Some(lake) = lake_map.get(&lake_id) {
                        grow_or_flow_lake(lake, accumulation, &tile_map, &lake_map, &mut tile_queue)?
    
                    } else {
                        continue;
                    };
    
                    // update the new lake.
                    // mark the contained tiles...
                    for contained_tile in &new_lake.contained_tiles {
                        let contained_tile = tile_map.try_get_mut(contained_tile)?; 
                        contained_tile.lake_id = Some(lake_id.clone());
                        contained_tile.outlet_from = None
                    }
    
                    // mark the outlet tiles...
                    for (sponsor,outlet_tile) in &new_lake.outlet_tiles {
                        match outlet_tile {
                            Neighbor::Tile(outlet_tile) => {
                                let outlet_tile = tile_map.try_get_mut(outlet_tile)?; 
                                outlet_tile.outlet_from = Some(Neighbor::Tile(sponsor.clone()));
                            },
                            Neighbor::CrossMap(outlet_tile, direction) => {
                                let outlet_tile = tile_map.try_get_mut(outlet_tile)?; 
                                outlet_tile.outlet_from = Some(Neighbor::CrossMap(sponsor.clone(),direction.opposite()));
                            },
                            Neighbor::OffMap(_) => (),
                        } // else it's an outlet off the map, and there's nothing to mark
                    }
    
                    if accumulation > 0.0 { // we're still not done we have to do something with the remaining water.
                        let outlet_tiles = &new_lake.outlet_tiles;
                        if outlet_tiles.is_empty() {
                            // add this task back to the queue so it can try to flood the lake to the next spillover.
                            tile_queue.push((tile_fid,accumulation));
    
                        } else {
                            // this is the same as above, but with the new lake.
                            // we can automatically flow to those tiles.
                            let neighbor_flow = accumulation/outlet_tiles.len() as f64;

                            for (_,neighbor_fid) in outlet_tiles {
                                match neighbor_fid {
                                    Neighbor::Tile(neighbor_fid) | Neighbor::CrossMap(neighbor_fid,_) => {
                                        // add a task to the queue to flow this down.
                                        tile_queue.push((neighbor_fid.clone(),neighbor_flow));
                                    }
                                    Neighbor::OffMap(_) => (),
                                }
                            }
                        }
    
                    }
    
                    // replace it in the map.
                    for lake in delete_lakes {
                        _ = lake_map.remove(&lake);
                    }
                    _ = lake_map.insert(lake_id, new_lake);
                },
            
            }
    
        }


    }


    // figure out some numbers for generating curvy lakes.
    let tile_area = tiles_layer.estimate_average_tile_area()?;
    let tile_width = tile_area.sqrt();
    let buffer_distance = (tile_width/10.0) * -lake_buffer_scale.lake_buffer_scale;
    // the next isn't customizable, it just seems to work right. 
    let simplify_tolerance = tile_width/10.0;
    let mut new_lake_map = HashMap::new();


    for (id,lake) in lake_map.into_iter().watch(progress,"Drawing lakes.","Lakes drawn.") {
        if !lake.contained_tiles.is_empty() {
            let lake_geometry = lake.dissolve_tiles(&mut tiles_layer)?;
            let (lake_temp,lake_evap,lake_type) = lake.get_temp_evap_and_type();

            let geometry = make_curvy_lakes(lake_geometry, lake_bezier_scale, buffer_distance, simplify_tolerance)?;
            let lake = NewLake {
                elevation: lake.elevation,
                type_: lake_type.clone(),
                flow: lake.flow,
                size: lake.contained_tiles.len() as i32,
                temperature: lake_temp,
                evaporation: lake_evap
            };
            _ = new_lake_map.insert(id, (lake,geometry));

        }

    }



    // I can't write to the lakes layer at the same time I'm drawing because I'm also using
    // the tile layer to get the geometries for dissolving the shapes. That's a mutable borrow conflict.
    let mut lakes_layer = target.create_lakes_layer(overwrite_layer)?;

    let mut written_lake_map = HashMap::new();

    for (id,(lake,geometry)) in new_lake_map.into_iter().watch(progress,"Writing lakes.","Lakes written.") {
        let lake_fid = lakes_layer.add_lake(&lake,geometry)?;
        _ = written_lake_map.insert(id, lake_fid);

    }


    // re-open layer to avoid mutability conflict from writing the lakes (this allows the layer to be dropped)
    // when borrowed to open the lakes_layer.
    let edit_tiles_layer = target.edit_tile_layer()?;

    for (tile_fid,tile) in tile_map.into_iter().watch(progress,"Writing lake elevations.","Lake elevations written.") {
        let mut feature = edit_tiles_layer.try_feature_by_id(&tile_fid)?;
        
        let lake_id = if let Some(lake_id) = tile.lake_id {
            written_lake_map.get(&lake_id)
        } else {
            None
        };

        feature.set_lake_id(&lake_id.cloned())?;

        feature.set_outlet_from(&tile.outlet_from)?;

        edit_tiles_layer.update_feature(feature)?;

    }



    Ok(())


}

fn determine_water_fill_task<Progress: ProgressObserver>(tile_fid: &IdRef, tile: &TileForWaterFill, tile_accumulation: f64, tile_map: &EntityIndex<TileSchema, TileForWaterFill>, next_lake_id: &mut RangeFrom<u64>, tile_queue: &mut QueueWatcher<&str, Progress, (IdRef, f64)>, lake_map: &mut HashMap<IdRef, Lake>) -> Result<Option<WaterFillTask>,CommandError> {
    if let Some(lake_id) = &tile.lake_id {
        // we're already in a lake, so the accumulation is intended to fill it.
        Ok(Some(WaterFillTask::FillLake(lake_id.clone(), tile_accumulation)))
    } else {
        // there is no lake here, so this is a flow task, unless it turns out we need a lake here.
        // we already calculated the lowest neighbors that are actually below the tile in Flow, so let's just check that first.

        let flow_to = &tile.flow_to;
        if flow_to.is_empty() {
            // we need to recalculate to find the lowest neighbors for this area:
            let (_,lowest_elevation) = find_lowest_tile(tile,tile_map,|t| {
                match t {
                    Some((t,_)) => t.elevation,
                    // for off the map, assume that the tile is the lowest possible elevation. This will force
                    // water to flow off the map rather than accumulate.
                    None => f64::NEG_INFINITY,
                }
            }, |t| &t.neighbors)?;

            // assuming that succeeded, we can create a new lake now.
            if let Some(lowest_elevation) = lowest_elevation {
                // we need to be in a lake, so create a new one.
                let lake_id = IdRef::new(next_lake_id.next().expect("Why would an unlimited range fail to return a next value?")); // it should be an infinite iterator, so it should always return Some.

                let new_lake = Lake {
                    elevation: tile.elevation,
                    bottom_elevation: tile.elevation,
                    flow: 0.0, // will be added to in the task.
                    spillover_elevation: lowest_elevation,
                    contained_tiles: vec![tile_fid.clone()],
                    tile_temperatures: vec![tile.temperature],
                    shoreline_tiles: tile.neighbors.iter().map(|NeighborAndDirection(a,_)| (tile_fid.clone(),a.clone())).collect(),
                    outlet_tiles: Vec::new()
                };

                _ = lake_map.insert(lake_id.clone(), new_lake);
                Ok(Some(WaterFillTask::FillLake(lake_id,tile_accumulation))) // I just inserted it, it should exist here.

            } else {
                // this is a tile with no neighbors, which should be impossible. but there is nothing I can do.
                Ok(None)
            }


        } else {
            // we've got tiles that are lowever in elevation to go to...
            let neighbor_flow = tile_accumulation/flow_to.len() as f64;

            for neighbor_fid in flow_to {
                match neighbor_fid {
                    Neighbor::Tile(neighbor_fid) | Neighbor::CrossMap(neighbor_fid,_) => {
                        // add a task to the queue to flow this down.
                        tile_queue.push((neighbor_fid.clone(),neighbor_flow));
                    }
                    Neighbor::OffMap(_) => (),
                }
            }
            // and the task for this one is to add to the flow:
            Ok(Some(WaterFillTask::AddToFlow(tile_accumulation)))
        }
    


    }
}


fn grow_or_flow_lake<Progress: ProgressObserver>(lake: &Lake, accumulation: f64, tile_map: &EntityIndex<TileSchema, TileForWaterFill>, lake_map: &HashMap<IdRef, Lake>, tile_queue: &mut QueueWatcher<&str, Progress, (IdRef, f64)>) -> Result<(Lake, f64, Vec<IdRef>), CommandError> {
    let outlet_tiles = &lake.outlet_tiles;
    if outlet_tiles.is_empty() {
        // no outlet tiles, so we have to grow the lake.

        // I will need to reduce the increase according to evaporation.
        let (_,lake_evap) = lake.calc_temp_and_evap(); 

        // Calculate how much it increases per tile, reducing by evaporation, it won't decrease
        let accumulation_per_tile = accumulation/lake.contained_tiles.len() as f64;
        let lake_increase = (accumulation_per_tile - lake_evap).max(0.0);

        // If that would bring it over the spillover, then subtract that from increase, and the difference is the remaining accumulation
        let spillover_increase = (lake.spillover_elevation - lake.elevation).max(0.0);
        let (lake_increase,total_remaining_accumulation,overflow) = if lake_increase > spillover_increase {
            let remaining_accumulation_per_tile = (lake_increase - spillover_increase).max(0.0);
            let total_remaining_accumulation = remaining_accumulation_per_tile * lake.contained_tiles.len() as f64;
            (spillover_increase,total_remaining_accumulation,true)
        } else {
            (lake_increase,0.0,false)
        };

        // get new values for the new lake.
        let new_lake_elevation = lake.elevation + lake_increase;
        let mut new_bottom_elevation = lake.bottom_elevation;
        let new_lake_flow = lake.flow + accumulation;

        

        if overflow {
            // we need to increase the size of the lake. Right now, we are at the spillover level.
            // Basically, pretend that we are making the lake deeper by 0.0001 (or some other small amount)
            // and walk the shoreline and beyond looking for:
            // * tiles that are in a lake already:
            //   * if the lake elevation is between this lake elevation and the test elevation, we need to "swallow" the lake.
            //   * if the lake is shorter than this lake's elevation, then this is the same as if the tile were a lower shoreline.
            // * tiles that are between the lake elevation and this test elevation (new part of a lake, and keep walking it's neighbors)
            // * tiles that are taller than than the test elevation:
            // * tiles that are shorter than the lake elevation (since lake elevation is at spillover, this means we're starting to go downhill again, so this is a new outlet and new shoreline, as above, we'll also add some flow to this eventually)

            let test_elevation = new_lake_elevation + 0.001;
            let mut walk_queue = lake.shoreline_tiles.clone();
            let mut new_shoreline = Vec::new();
            let mut new_outlets = Vec::new();
            let mut new_contained_tiles = lake.contained_tiles.clone();
            let mut new_temperatures = lake.tile_temperatures.clone();
            let mut checked_tiles: HashSet<IdRef> = HashSet::from_iter(new_contained_tiles.iter().cloned());
            let mut new_spillover_elevation = None;
            let mut delete_lakes = Vec::new();


            while let Some((sponsor_fid,check_fid)) = walk_queue.pop() {
                match check_fid {
                    Neighbor::OffMap(_) => {
                        // the edge of the map acts like an ocean
                        new_outlets.push((sponsor_fid.clone(),check_fid.clone()));
                        new_shoreline.push((sponsor_fid,check_fid))
                    },
                    ref neighbor @ (Neighbor::Tile(ref check_fid) | Neighbor::CrossMap(ref check_fid,_)) => {
                        if checked_tiles.contains(check_fid) {
                            continue;
                        }
                        _ = checked_tiles.insert(check_fid.clone());
        
        
                        let check = tile_map.try_get(check_fid)?; 
                        if check.grouping.is_ocean() {
                            // it's an outlet
                            new_outlets.push((sponsor_fid.clone(),neighbor.clone()));
                            new_shoreline.push((sponsor_fid,neighbor.clone()));
                        } else if check.elevation > test_elevation {
                            // it's too high to fill. This is now part of the shoreline.
                            new_shoreline.push((sponsor_fid,neighbor.clone()));
                            // And this might change our spillover elevation
                            new_spillover_elevation = new_spillover_elevation.map(|e: f64| e.min(check.elevation)).or(Some(check.elevation));
                        } else if let Some(lake_id) = &check.lake_id {
                            // it's in a lake already...
                            if let Some(other_lake) = lake_map.get(lake_id) {
                                if (other_lake.elevation <= test_elevation) && (other_lake.elevation >= new_lake_elevation) {
                                    // the lakes are about the same elevation, so
                                    // merge the other one into this one.
                                    // it's contained tiles become part of this one
                                    new_contained_tiles.extend(other_lake.contained_tiles.iter().cloned());
                                    new_temperatures.extend(other_lake.tile_temperatures.iter());
                                    new_bottom_elevation = lake.bottom_elevation.min(other_lake.bottom_elevation);
                                    // plus, we've already checked them.
                                    checked_tiles.extend(other_lake.contained_tiles.iter().cloned());
                                    // add it's shoreline to the check queue
                                    walk_queue.extend(other_lake.shoreline_tiles.iter().cloned());
                                    delete_lakes.push(lake_id.clone());
                                } else {
                                    // otherwise, add this as an outlet. (I'm assuming that the lake is lower in elevation, I'm not sure how else we could have reached it)
                                    new_outlets.push((sponsor_fid.clone(),neighbor.clone()));
                                    new_shoreline.push((sponsor_fid,neighbor.clone()));
                                }
        
                            } else {
                                continue;
                            }
                        } else if check.elevation < new_lake_elevation {
                                // it's below the original spillover, which means it's an outlet beyond our initial shoreline.
                                new_outlets.push((sponsor_fid.clone(),neighbor.clone()));
                                new_shoreline.push((sponsor_fid,neighbor.clone()));
                        } else {
                            // it's floodable.
                            new_contained_tiles.push(check_fid.clone());
                            new_temperatures.push(check.temperature);
                            walk_queue.extend(check.neighbors.iter().map(|NeighborAndDirection(id,_)| (check_fid.clone(),id.clone())));
                        }                    
                    },
                }


            }

            Ok((Lake {
                elevation: new_lake_elevation,
                flow: new_lake_flow,
                bottom_elevation: new_bottom_elevation,
                spillover_elevation: new_spillover_elevation.unwrap_or(new_lake_elevation),
                contained_tiles: new_contained_tiles,
                tile_temperatures: new_temperatures,
                shoreline_tiles: new_shoreline,
                outlet_tiles: new_outlets
            },total_remaining_accumulation,delete_lakes))


        } else {
            Ok((Lake {
                elevation: new_lake_elevation,
                flow: new_lake_flow,
                bottom_elevation: new_bottom_elevation,
                spillover_elevation: lake.spillover_elevation,
                contained_tiles: lake.contained_tiles.clone(),
                tile_temperatures: lake.tile_temperatures.clone(),
                shoreline_tiles: lake.shoreline_tiles.clone(),
                outlet_tiles: lake.outlet_tiles.clone()
            },total_remaining_accumulation,Vec::new()))
        }

    } else {
        // we can automatically flow to those tiles.
        let neighbor_flow = accumulation/outlet_tiles.len() as f64;

        for (_,neighbor_fid) in outlet_tiles {
            match neighbor_fid {
                Neighbor::Tile(neighbor_fid) | Neighbor::CrossMap(neighbor_fid,_) => {
                    // add a task to the queue to flow this down.
                    tile_queue.push((neighbor_fid.clone(),neighbor_flow));
                }
                Neighbor::OffMap(_) => (),
            }
        }

        // but we need to increase the flow
        Ok((Lake {
            elevation: lake.elevation,
            bottom_elevation: lake.bottom_elevation,
            flow: lake.flow + accumulation,
            spillover_elevation: lake.spillover_elevation,
            contained_tiles: lake.contained_tiles.clone(),
            tile_temperatures: lake.tile_temperatures.clone(),
            shoreline_tiles: lake.shoreline_tiles.clone(),
            outlet_tiles: lake.outlet_tiles.clone()
        },0.0,Vec::new()))


    }
}

pub(crate) fn make_curvy_lakes(lake_geometry: VariantArealGeometry, bezier_scale: &BezierScaleArg, buffer_distance: f64, simplify_tolerance: f64) -> Result<MultiPolygon, CommandError> {
    let lake_geometry = simplify_lake_geometry(lake_geometry,buffer_distance,simplify_tolerance)?;
    // occasionally, the simplification or other tasks turns the lakes into a multipolygon, which is why the lakes layer has to be multipolygon
    let lake_geometry: MultiPolygon = lake_geometry.try_into()?;
    lake_geometry.bezierify(bezier_scale.bezier_scale)
    /*
    // Old code when I was dealing with geometry directly
    let mut new_geometry = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
    if lake_geometry.geometry_type() == OGRwkbGeometryType::wkbMultiPolygon {
        for i in 0..lake_geometry.geometry_count() {
            for geometry in bezierify_polygon(&lake_geometry.get_geometry(i),bezier_scale)? {
                new_geometry.add_geometry(geometry)?;
            }
        }

    } else {
        for geometry in bezierify_polygon(&lake_geometry,bezier_scale)? {
            new_geometry.add_geometry(geometry)?;
        }

    };

    Ok(new_geometry)
    */
}


pub(crate) fn simplify_lake_geometry(lake_geometry: VariantArealGeometry, buffer_distance: f64, simplify_tolerance: f64) -> Result<VariantArealGeometry, CommandError> {
    let lake_geometry = if buffer_distance == 0.0 {
        lake_geometry
    } else {
        lake_geometry.buffer(buffer_distance, 1)?
    };
    let lake_geometry = if simplify_tolerance > 0.0 {
        let mut simplify_tolerance = simplify_tolerance;
        let mut simplified = lake_geometry.simplify(simplify_tolerance)?;
        // There have been occasions where the geometry gets simplified out of existence, which makes the polygon_to_vertices function
        // print out error messages. This loop decreases simplification until the geometry works.
        while simplified.is_empty() {
            simplify_tolerance -= 0.05;
            if simplify_tolerance <= 0.0 {
                simplified = lake_geometry;
                break;
            }
            
            simplified = lake_geometry.simplify(simplify_tolerance)?;
        }
        simplified
    } else {
        lake_geometry
    };
    Ok(lake_geometry)
}
