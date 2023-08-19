use crate::entity;
use crate::entity_field_assign;
use crate::world_map::TileFeature;
use crate::world_map::Entity;
use crate::world_map::TileEntityLat;
use crate::world_map::TileEntityLatElevOcean;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::world_map::Terrain;

pub(crate) fn generate_temperatures<Progress: ProgressObserver>(target: &mut WorldMapTransaction, equator_temp: i8, polar_temp: i8, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // Algorithm borrowed from AFMG with some modifications

    let equator_temp = equator_temp as f64;
    let polar_temp = polar_temp as f64;
    let temp_delta = equator_temp - polar_temp;
    const EXPONENT: f64 = 0.5;

    fn interpolate(t: f64) -> f64 { // TODO: Test this somehow...
        // From AFMG/d3: `t` is supposed to be a value from 0 to 1. If t <= 0.5 (`(t *= 2) <= 1`) then the function above is `y = ((2x)^(1/2))/2`. If t is greater, then the function is `y = (2 - (2-x)^(1/2))/2`. The two functions both create a sort of parabola. The first one starts curving up steep at 0 (the pole) and then flattens out to almost diagonal at 0.5. The second one continues the diagonal that curves more steeply up towards 1 (the equator). I'm not sure whey this curve was chosen, I would have expected a flatter curve at the equator.
        let t = t * 2.0;
        (if t <= 1.0 {
            t.powf(EXPONENT)
        } else {
            2.0 - (2.0-t).powf(EXPONENT)
        })/2.0
    }

    let features = layer.read_features().to_entities_vec::<_,TileEntityLatElevOcean>(progress)?;

    progress.start_known_endpoint(|| ("Generating temperatures.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let base_temp = equator_temp - (interpolate(feature.site_y.abs()/90.0) * temp_delta);
        let adabiatic_temp = base_temp - if !feature.terrain.is_ocean() {
            (feature.elevation/1000.0)*6.5
        } else {
            0.0
        };
        let temp = (adabiatic_temp*100.0).round()/100.0;

        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_temperature(temp)?;

            layer.update_feature(working_feature)?;

        }



        progress.update(|| i);


    }

    progress.finish(|| "Temperatures calculated.");

    Ok(())
}

pub(crate) fn generate_winds<Progress: ProgressObserver>(target: &mut WorldMapTransaction, winds: [i32; 6], progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // Algorithm borrowed from AFMG with some modifications


    let features = layer.read_features().to_entities_vec::<_,TileEntityLat>(progress)?;

    progress.start_known_endpoint(|| ("Generating winds.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let wind_tier = ((feature.site_y - 89.0)/30.0).abs().floor() as usize;
        let wind_dir = winds[wind_tier];

        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_wind(wind_dir)?;

            layer.update_feature(working_feature)?;

        }


        progress.update(|| i);


    }

    progress.finish(|| "Winds generated.");

    Ok(())
}

pub(crate) fn generate_precipitation<Progress: ProgressObserver>(target: &mut WorldMapTransaction, moisture: u16, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    // Algorithm borrowed from AFMG with some modifications, most importantly I don't have a grid, so I follow the paths of the wind to neighbors.

    const MAX_PASSABLE_ELEVATION: i32 = 85; // FUTURE: I've found that this is unnecessary, the elevation change should drop the precipitation and prevent any from passing on. 

    // Bands of rain at different latitudes, like the ITCZ
    const LATITUDE_MODIFIERS: [f64; 18] = [4.0, 2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 2.0, 2.0, 1.0, 1.0, 1.0, 0.5];

    // I believe what this does is scale the moisture scale correctly to the size of the map. Otherwise, I don't know.
    let cells_number_modifier = (layer.feature_count() as f64 / 10000.0).powf(0.25);
    let prec_input_modifier = moisture as f64/100.0;
    let modifier = cells_number_modifier * prec_input_modifier;

    entity!(TileDataForPrecipitation TileFeature {
        elevation_scaled: i32, 
        wind: i32, 
        terrain: Terrain, 
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
    let mut tile_map = layer.read_features().to_entities_index::<_,TileDataForPrecipitation>(progress)?;

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_tiles: Vec<u64> = tile_map.keys().copied().collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_tiles.sort();
    let working_tiles = working_tiles;

    progress.start_known_endpoint(|| ("Tracing winds.",working_tiles.len()));

    for (i,start_fid) in working_tiles.iter().enumerate() {
        if let Some(tile) = tile_map.get(start_fid).cloned() {

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

                // TODO: I think this will be improved if instead of just sending precipitation to one tile, I send it to all
                // tiles within about 20-25 degrees of the wind direction. I'll have less of those "snake arms" that I see
                // now. Split up the precipitation evenly.
                // -- This would require switching to a queue thing like I did for water flow.
                // -- but then we don't have the 'visited' set to check against. If a circle passes over water, it will
                //    infinite loop. What if I have a counter that decrements instead, stopping when we hit zero and passed 
                //    along to the queue.

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
                        tile_map.get(&next_fid).map(|tile| (next_fid,tile.clone()))
                    }

                } else {
                    None
                };

                if let Some((next_fid,mut next)) = next {
                    if current.temperature >= -5.0 { // no humidity change across permafrost? FUTURE: I'm not sure this is right. There should still be precipitation in the cold, and if there's a bunch of humidity it should all precipitate in the first cell, shouldn't it?
                        if current.terrain.is_ocean() {
                            if !next.terrain.is_ocean() {
                                // coastal precipitation
                                // FUTURE: The AFMG code uses a random number between 10 and 20 instead of 15. I didn't feel like this was
                                // necessary considering it's the only randomness I would use, and nothing else is randomized.
                                next.precipitation += (humidity / 15.0).max(1.0);
                            } else {
                                // add more humidity
                                humidity = (humidity + 5.0 * current.lat_modifier).max(max_prec);
                                // precipitation over water cells
                                current.precipitation += 5.0 * modifier;
                            }
                        } else {
                            let is_passable = next.elevation_scaled < MAX_PASSABLE_ELEVATION;
                            let precipitation = if is_passable {
                                // precipitation under normal conditions
                                let normal_loss = (humidity / (10.0 * current.lat_modifier)).max(1.0);
                                // difference in height
                                let diff = (next.elevation_scaled - current.elevation_scaled).max(0) as f64;
                                // additional modifier for high elevation of mountains
                                let modifier = (next.elevation_scaled / 70).pow(2) as f64;
                                (normal_loss + diff + modifier).clamp(1.0,humidity.max(1.0))
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

                        if let Some(real_current) = tile_map.get_mut(&current_fid) {
                            real_current.precipitation = current.precipitation;
                        }

                        if let Some(real_next) = tile_map.get_mut(&next_fid) {
                            real_next.precipitation = next.precipitation;
                        }

                    }

                    (current_fid,current) = (next_fid,next);
                } else {
                    if current.terrain.is_ocean() {
                        // precipitation over water cells
                        current.precipitation += 5.0 * modifier;
                    } else {
                        current.precipitation = humidity;
                    }

                    if let Some(real_current) = tile_map.get_mut(&current_fid) {
                        real_current.precipitation = current.precipitation;
                    }

                    break;

                }
            }
        
        }

        progress.update(|| i);

    }

    progress.finish(|| "Winds traced.");

    progress.start_known_endpoint(|| ("Writing precipitation",tile_map.len()));

    for (fid,tile) in tile_map {
        if let Some(mut working_feature) = layer.feature_by_id(&fid) {

            working_feature.set_precipitation(tile.precipitation)?;

            layer.update_feature(working_feature)?;
        }


    }

    progress.finish(|| "Precipitation written.");

    Ok(())
}
