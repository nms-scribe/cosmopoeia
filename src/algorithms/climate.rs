
use crate::entity;
use crate::entity_field_assign;
use crate::world_map::TileFeature;
use crate::world_map::Entity;
use crate::world_map::TileForWinds;
use crate::world_map::TileForTemperatures;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::Grouping;
use crate::world_map::TileSchema;
use crate::commands::TemperatureRangeArg;
use crate::commands::WindsArg;
use crate::commands::PrecipitationArg;

pub(crate) fn generate_temperatures<Progress: ProgressObserver>(target: &mut WorldMapTransaction, temperatures: &TemperatureRangeArg, progress: &mut Progress) -> Result<(),CommandError> {

    fn interpolate(t: f64) -> f64 { 
        // From AFMG/d3: `t` is supposed to be a value from 0 to 1. If t <= 0.5 (`(t *= 2) <= 1`) then the function above is `y = ((2x)^(1/2))/2`. If t is greater, then the function is `y = (2 - (2-x)^(1/2))/2`. The two functions both create a sort of parabola. The first one starts curving up steep at 0 (the pole) and then flattens out to almost diagonal at 0.5. The second one continues the diagonal that curves more steeply up towards 1 (the equator). I'm not sure whey this curve was chosen, I would have expected a flatter curve at the equator.
        let t = t * 2.0;
        (if t <= 1.0 {
            t.sqrt()
        } else {
            2.0 - (2.0 - t).sqrt()
        })/2.0
    }

    let mut layer = target.edit_tile_layer()?;

    // Algorithm borrowed from AFMG with some modifications

    let equator_temp = temperatures.equator_temp as f64;
    let polar_temp = temperatures.polar_temp as f64;
    let temp_delta = equator_temp - polar_temp;

    let features = layer.read_features().into_entities_vec::<_,TileForTemperatures>(progress)?;

    for feature in features.iter().watch(progress,"Generating temperatures.","Temperatures calculated.") {

        let base_temp = interpolate(feature.site_y.abs()/90.0).mul_add(-temp_delta, equator_temp);
        let adabiatic_temp = base_temp - if feature.grouping.is_ocean() {
            0.0
        } else {
            (feature.elevation/1000.0)*6.5
        };
        let temp = (adabiatic_temp*100.0).round()/100.0;

        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_temperature(temp)?;

            layer.update_feature(working_feature)?;

        }




    }

    Ok(())
}

pub(crate) fn generate_winds<Progress: ProgressObserver>(target: &mut WorldMapTransaction, winds: &WindsArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // Algorithm borrowed from AFMG with some modifications

    let winds = winds.to_range_map();

    let features = layer.read_features().into_entities_vec::<_,TileForWinds>(progress)?;

    for feature in features.iter().watch(progress,"Generating winds.","Winds generated.") {

        let wind_dir = winds.get(&ordered_float::OrderedFloat(feature.site_y)).copied().unwrap_or(90) as i32;
 
        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_wind(wind_dir)?;

            layer.update_feature(working_feature)?;

        }


    }

    Ok(())
}

pub(crate) fn generate_precipitation<Progress: ProgressObserver>(target: &mut WorldMapTransaction, precipitation_arg: &PrecipitationArg, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications, most importantly I don't have a grid, so I follow the paths of the wind to neighbors.

    const MAX_PASSABLE_ELEVATION: i32 = 85; 

    // Bands of rain at different latitudes, like the ITCZ
    const LATITUDE_MODIFIERS: [f64; 18] = [4.0, 2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 2.0, 2.0, 1.0, 1.0, 1.0, 0.5];

    let mut layer = target.edit_tile_layer()?;

    // I believe what this does is scale the moisture scale correctly to the size of the map. Otherwise, I don't know.
    let cells_number_modifier = (layer.feature_count() as f64 / 10000.0).powf(0.25);
    let prec_input_modifier = precipitation_arg.precipitation_factor as f64/100.0;
    let modifier = cells_number_modifier * prec_input_modifier;

    entity!(TileDataForPrecipitation: Tile {
        elevation_scaled: i32, 
        wind: i32, 
        grouping: Grouping, 
        neighbors: Vec<(u64,i32)>,
        temperature: f64,
        precipitation: f64 = |_| {
            Ok::<_,CommandError>(0.0)
        },
        lat_modifier: f64 = |feature: &TileFeature| {
            let site_y = entity_field_assign!(feature site_y f64);
            let lat_band = ((site_y.abs() - 1.0) / 5.0).floor() as usize;
            let lat_modifier = LATITUDE_MODIFIERS[lat_band];
            Ok::<_,CommandError>(lat_modifier)
        }
    });

    // I need to trace the data across the map, so I can't just do quick read and writes to the database.
    let mut tile_map = layer.read_features().into_entities_index::<_,TileDataForPrecipitation>(progress)?;

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_tiles: Vec<u64> = tile_map.keys().copied().collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_tiles.sort();
    let working_tiles = working_tiles;

    for start_fid in working_tiles.iter().watch(progress,"Tracing winds.","Winds traced.") {
        let tile = tile_map.try_get(start_fid)?.clone();
        let max_prec = 120.0 * tile.lat_modifier;
        let mut humidity = max_prec - tile.elevation_scaled as f64;

        let mut current = tile;
        let mut current_fid = *start_fid;
        let mut visited = vec![current_fid];

        loop {
            if humidity < 0.0 {
                // there is no humidity left to work with.
                break;
            }

            // find neighbor closest to wind direction
            let mut best_neighbor: Option<(_,_)> = None;
            for (fid,direction) in &current.neighbors {
                // calculate angle difference
                let angle_diff = (direction - current.wind).abs();
                let angle_diff = if angle_diff > 180 {
                    360 - angle_diff
                } else {
                    angle_diff
                };
            
                // if the angle difference is greater than 45, it's not going the right way, so don't even bother with this one.
                if angle_diff < 45 {
                    if let Some(better_neighbor) = best_neighbor {
                        if better_neighbor.1 > angle_diff {
                            best_neighbor = Some((*fid,angle_diff));
                        }

                    } else {
                        best_neighbor = Some((*fid,angle_diff));
                    }

                }

            }

            let next = if let Some((next_fid,_)) = best_neighbor {
                if visited.contains(&next_fid) {
                    // we've reached one we've already visited, I don't want to go in circles.
                    None
                } else {
                    // visit it so we don't do this one again.
                    visited.push(next_fid);
                    Some((next_fid,tile_map.try_get(&next_fid)?.clone()))
                }

            } else {
                None
            };

            if let Some((next_fid,mut next)) = next {
                if current.temperature >= -5.0 { // no humidity change across permafrost? 'm not sure this is right. There should still be precipitation in the cold, and if there's a bunch of humidity it should all precipitate in the first cell, shouldn't it?
                    if current.grouping.is_ocean() {
                        if next.grouping.is_ocean() {
                            // add more humidity
                            humidity = 5.0f64.mul_add(current.lat_modifier, humidity).max(max_prec);
                            // precipitation over water cells
                            current.precipitation += 5.0 * modifier;
                        } else {
                            // coastal precipitation
                            // NOTE: The AFMG code uses a random number between 10 and 20 instead of 15. I didn't feel like this was
                            // necessary considering it's the only randomness I would use, and nothing else is randomized.
                            next.precipitation += (humidity / 15.0).max(1.0);
                        }
                    } else {
                        let is_passable = next.elevation_scaled < MAX_PASSABLE_ELEVATION;
                        let precipitation = if is_passable {
                            // precipitation under normal conditions
                            let normal_loss = (humidity / (10.0 * current.lat_modifier)).max(1.0);
                            // difference in height
                            let diff = (next.elevation_scaled - current.elevation_scaled).max(0) as f64;
                            // additional modifier for high elevation of mountains
                            let elev_modifier = (next.elevation_scaled / 70).pow(2) as f64;
                            (normal_loss + diff + elev_modifier).clamp(1.0,humidity.max(1.0))
                        } else {
                            humidity
                        };
                        current.precipitation = precipitation;
                        // sometimes precipitation evaporates
                        humidity = if is_passable {
                            // FUTURE: I feel like this evaporation was supposed to be a multiplier not an addition. Not much is evaporating.
                            // FUTURE: Shouldn't it also depend on temperature?
                            let evaporation = if precipitation > 1.5 { 1.0 } else { 0.0 };
                            (humidity - precipitation + evaporation).clamp(0.0,max_prec)
                        } else {
                            0.0
                        };

                    }

                    let real_current = tile_map.try_get_mut(&current_fid)?;
                    real_current.precipitation = current.precipitation;

                    let real_next = tile_map.try_get_mut(&next_fid)?;
                    real_next.precipitation = next.precipitation;

                }

                current_fid = next_fid;
                current = next;
            } else {
                if current.grouping.is_ocean() {
                    // precipitation over water cells
                    current.precipitation += 5.0 * modifier;
                } else {
                    current.precipitation = humidity;
                }

                let real_current = tile_map.try_get_mut(&current_fid)?; 
                real_current.precipitation = current.precipitation;

                break;

            }
        }

    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing precipitation.","Precipitation written.") {
        if let Some(mut working_feature) = layer.feature_by_id(fid) {

            working_feature.set_precipitation(tile.precipitation)?;

            layer.update_feature(working_feature)?;
        }


    }

    Ok(())
}
