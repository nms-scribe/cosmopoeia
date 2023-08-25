
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::errors::CommandError;
use crate::world_map::NewTile;
use crate::world_map::TypedFeature;
use crate::world_map::TileWithNeighborsElevation;
use crate::world_map::TilesLayer;
use crate::utils::Point;
use crate::utils::TryGetMap;

pub(crate) fn load_tile_layer<Generator: Iterator<Item=Result<NewTile,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target = target.create_tile_layer(overwrite_layer)?;

    for tile in generator.watch(progress,"Writing tiles.","Tiles written.") {
        target.add_tile(tile?)?;
    }

    Ok(())

}

pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    //use std::time::Instant;

    // let mut time_map = HashMap::new();

    macro_rules! mark_time {
        {$name: literal: $($block: tt)*} => {
            // NOTE: Uncomment these lines to do some benchmarking
            //let now = Instant::now();
            $($block)*
            //match time_map.entry($name) {
            //    std::collections::hash_map::Entry::Occupied(mut entry) => *entry.get_mut() += now.elapsed().as_secs_f64(),
            //    std::collections::hash_map::Entry::Vacant(entry) => {entry.insert(now.elapsed().as_secs_f64());},
            //}
            
        };
    }

    // TODO: Memory usage for this isn't much more than other algorithms. The problem is almost entirely time.
    // There's CPU usage as well, but the best way to reduce that is to reduce the time (I can reduce usage by sleeping for occasional milliseconds, but that
    // slows the whole process down.)
    let mut layer = target.edit_tile_layer()?;
    
    mark_time!{"reading tiles": 
        let mut features = Vec::new();
        for feature in layer.read_features().watch(progress,"Reading tiles.","Tiles read.") {
            features.push(feature.fid()?);
        }
    };

    // # Loop through all features and find features that touch each feature
    // for f in feature_dict.values():
    for working_fid in features.iter().watch(progress,"Calculating neighbors.","Neighbors calculated.") {

        mark_time!{"feature geometry":
            let (site_x,site_y,envelope,working_geometry) = { // shelter mutable borrow

                let feature = layer.feature_by_id(&working_fid).unwrap();
                let working_geometry = feature.geometry().unwrap();
                let envelope = working_geometry.envelope();
                (feature.site_x()?,feature.site_y()?,envelope,working_geometry.clone())

            };
        }

        mark_time!{"set_spatial_filter_rect":
            layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);
        }

        let mut neighbors = Vec::new();

        mark_time!{"intersecting features":
            for intersecting_feature in layer.read_features() {

                mark_time!{"intersecting features: get fid":
                    let intersecting_fid = intersecting_feature.fid()?;
                }

                if working_fid != &intersecting_fid {

                    // NOTE: this is by far the slowest part of the process, apart from updating the feature. I can think of nothing to do to optimize this.
                    mark_time!{"intersecting features: disjoint check":
                        let is_disjoint = intersecting_feature.geometry().unwrap().disjoint(&working_geometry);
                    }

                    if !is_disjoint {
                        mark_time!{"intersecting features: neighbor geometry":
                            let neighbor_site_x = intersecting_feature.site_x()?;
                            let neighbor_site_y = intersecting_feature.site_y()?;
                        }

                        mark_time!{"intersecting features: neighbor angle":
                            let neighbor_angle = {
                                let (site_x,site_y,neighbor_site_x,neighbor_site_y) = (site_x,site_y,neighbor_site_x,neighbor_site_y);
                                // needs to be clockwise, from the north, with a value from 0..360
                                // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                                let counter_clockwise_from_east = ((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round();
                                // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                                let clockwise_from_north = 450.0 - counter_clockwise_from_east; 
                                // And then, to get the values in the range from 0..360, mod it.
                                let clamped = clockwise_from_north % 360.0;
                                clamped
                            };
                        }
                
                        mark_time!{"intersecting features: push neighbor":
                            neighbors.push((intersecting_fid,neighbor_angle.floor() as i32)); 
                        }

                    }

                }

            }
        }

        mark_time!{"clear_spatial_filter":
            layer.clear_spatial_filter();
        }

        mark_time!{"update feature":
            if let Some(mut working_feature) = layer.feature_by_id(&working_fid) {
                working_feature.set_neighbors(&neighbors)?;

                layer.update_feature(working_feature)?;

            }
        }


    }

    //println!("{:#?}",time_map);
    
    Ok(())

}


pub(crate) fn find_lowest_neighbors<Data: TileWithNeighborsElevation, TileMap: TryGetMap<u64,Data>>(entity: &Data, tile_map: &TileMap) -> Result<(Vec<u64>, Option<f64>),CommandError> {
    let mut lowest = Vec::new();
    let mut lowest_elevation = None;

    // find the lowest neighbors
    for (neighbor_fid,_) in entity.neighbors() {
        let neighbor = tile_map.try_get(&neighbor_fid)?;
        let neighbor_elevation = neighbor.elevation();
        if let Some(lowest_elevation) = lowest_elevation.as_mut() {
            if neighbor_elevation < *lowest_elevation {
                *lowest_elevation = neighbor_elevation;
                lowest = vec![*neighbor_fid];
            } else if neighbor_elevation == *lowest_elevation {
                lowest.push(*neighbor_fid)
            }
        } else {
            lowest_elevation = Some(neighbor_elevation);
            lowest.push(*neighbor_fid)
        }

    }
    Ok((lowest,lowest_elevation.copied()))

}

pub(crate) fn find_tile_site_point(previous_tile: Option<u64>, tiles: &TilesLayer<'_>) -> Result<Option<Point>, CommandError> {
    Ok(if let Some(x) = previous_tile {
        if let Some(x) = tiles.feature_by_id(&x) {
            Some(x.site()?)
        } else {
            None
        }
    } else {
        None
    })
}

