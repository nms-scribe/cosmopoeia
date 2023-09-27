use std::collections::HashSet;

use angular_units::Deg;
use angular_units::Angle;

use crate::entity;
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
use crate::progress::WatchableQueue;

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

        if let Some(mut working_feature) = layer.feature_by_id(feature.fid) {
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

        let wind_dir = Deg(winds.get(&ordered_float::OrderedFloat(feature.site_y)).copied().unwrap_or(90) as f64);
 
        if let Some(mut working_feature) = layer.feature_by_id(feature.fid) {
            working_feature.set_wind(wind_dir)?;

            layer.update_feature(working_feature)?;

        }


    }

    Ok(())
}



#[derive(Clone)]
pub(crate) struct PrecipitationFactors {
    lat_modifier: f64,
    max_precipitation: f64
}

impl PrecipitationFactors {

    // Bands of rain at different latitudes, like the ITCZ
    const LATITUDE_MODIFIERS: [f64; 18] = [4.0, 2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 2.0, 2.0, 1.0, 1.0, 1.0, 0.5];

    fn from_tile_feature(feature: &TileFeature) -> Result<Self,CommandError> {
        let site_y = feature.site_y()?;
        let lat_band = ((site_y.abs() - 1.0) / 5.0).floor() as usize;
        let lat_modifier = Self::LATITUDE_MODIFIERS[lat_band];
        let max_precipitation = 120.0 * lat_modifier;
        Ok(PrecipitationFactors {
            lat_modifier,
            max_precipitation
        })

    }
}

entity!(TileDataForPrecipitation: Tile {
    elevation: f64,
    wind: Deg<f64>, 
    grouping: Grouping, 
    neighbors: Vec<(u64,Deg<f64>)>,
    temperature: f64,
    precipitation: f64 = |_| {
        Ok::<_,CommandError>(0.0)
    },
    factors: PrecipitationFactors = PrecipitationFactors::from_tile_feature
});



pub(crate) fn generate_precipitation<Progress: ProgressObserver>(target: &mut WorldMapTransaction, precipitation_arg: &PrecipitationArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    let precipitation_modifier = precipitation_arg.precipitation_factor;

        
    // I need to trace the data across the map, so I can't just do quick read and writes to the database.
    let mut tile_map = layer.read_features().into_entities_index::<_,TileDataForPrecipitation>(progress)?;

    let mut visited = HashSet::new();

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_queue: Vec<(u64,Option<f64>,u64)> = tile_map.keys().map(|id| (*id,None,*id)).collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_queue.sort_by_key(|(id,_,_)| *id);
    let mut working_queue = working_queue.watch_queue(progress,"Tracing winds.","Winds traced.");

    while let Some((tile_id,humidity,start_id)) = working_queue.pop() {
        let mut tile = tile_map.try_get(&tile_id)?.clone(); // I'm cloning so I can make some changes without messing with the original.
        let humidity = if let Some(humidity) = humidity {
            humidity
        } else if tile.grouping.is_ocean() {
            // humidity is only picked up over the ocean
            precipitation_modifier * 5.0 * tile.factors.max_precipitation
        } else {
            // a small amount of additional humidity on land
            precipitation_modifier
            //(precipitation_modifier * tile.factors.max_precipitation) / 100.0
        };

        if humidity > 0.0 {
            // push humidity onto the neighbor tiles and then process them.

            // find neighbor closest to wind direction
            let mut best_neighbors = Vec::new();
            for (fid,direction) in &tile.neighbors {
                // calculate angle difference
                let angle_diff = Deg((direction.scalar() - tile.wind.scalar()).abs());
                // if the difference is greater than half a turn, it's actually reflected
                let angle_diff = if angle_diff > Deg::half_turn() {
                    angle_diff.reflect_x()
                } else {
                    angle_diff
                };
            
                // if the angle difference is greater than 45, it's not going the right way, so don't even bother with this one.
                if angle_diff < Deg(45.0) {
                    best_neighbors.push(*fid)
                }

            }

            if best_neighbors.is_empty() {
                // otherwise there were no other neighbors in the wind direction, so drop the remaining humidity here.
                // (I don't know why this would happen on a global world)
                tile.precipitation = (tile.precipitation + humidity).min(tile.factors.max_precipitation);

                let real_current = tile_map.try_get_mut(&tile_id)?; 
                real_current.precipitation = tile.precipitation;

            } else {
                // spread the humidity amongst them... FUTURE: Should I wait it for the more direct tiles?
                let humidity = humidity/best_neighbors.len() as f64;

                for next_fid in best_neighbors {
                    if visited.contains(&(start_id,next_fid)) {
                        continue;
                        // we've reached one we've already visited, I don't want to go in circles.
                    }
    
                    // visit it so we don't do this one again.
                    _ = visited.insert((start_id,next_fid));
    
                    let mut next = tile_map.try_get(&next_fid)?.clone(); // I'm cloning so I can make some changes without messing with the original.

                    let humidity = precipitate(&mut tile, &mut next, humidity)?;
    
                    let real_current = tile_map.try_get_mut(&tile_id)?;
                    real_current.precipitation = tile.precipitation;
            
                    let real_next = tile_map.try_get_mut(&next_fid)?;
                    real_next.precipitation = next.precipitation;
            
                    working_queue.push((next_fid,Some(humidity),start_id));
    
                }

            }


        } else {
            // there is no humidity left to work with.
        }

    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing precipitation.","Precipitation written.") {
        if let Some(mut working_feature) = layer.feature_by_id(*fid) {

            working_feature.set_precipitation(tile.precipitation)?;

            layer.update_feature(working_feature)?;
        }


    }

    Ok(())
}

fn precipitate(tile: &mut TileDataForPrecipitation, next: &mut TileDataForPrecipitation, humidity: f64) -> Result<f64, CommandError> {

    // Many of these calculations were taken from AFMG and I don't know where they got that.
    // FUTURE: I would love if someone could give me some better calculations, as I feel there are some things missing here compared to what I learned in school.
    // - The max_precipitation factor seems wrong. It should be a max_humidity, and once you go over that it becomes precipitation... i.e. dewpoint.
    //   - said max_humidity should depend on temperature and elevation.
    // - temperature change should increase precipitation (and might be the real reason for the coastal precipitation)
    // - elevation precipitation should be based on the difference in elevation, not the elevation scaled.


    let (tile_precipitation,humidity) = if tile.temperature >= -5.0 { 
        if tile.grouping.is_ocean() {
            if next.grouping.is_ocean() {
                (
                    // precipitation over water cells, not that it's going to change our climates at all...
                    5.0,
                    // add more humidity 
                    5.0f64.mul_add(tile.factors.lat_modifier, humidity)//.max(next.factors.max_precipitation)
                )
            } else {
                // coastal precipitation
                // we don't subtract this from regular humidity
                // NOTE: The AFMG code uses a random number between 10 and 20 instead of 15. I didn't feel like this was
                // necessary considering it's the only randomness I would use, and nothing else is randomized.
                next.precipitation += (humidity / 15.0).max(1.0);

                (
                    // no precipitation on this cell
                    0.0,
                    // humidity doesn't change.
                    humidity
                )
            }
        } else {
            // precipitation under normal conditions
            let normal_loss = humidity / (10.0 * tile.factors.lat_modifier);
            // difference in height
            let diff = (next.elevation - tile.elevation).max(0.0)/100 as f64;
            // additional modifier for high elevation of mountains
            let elev_modifier = (next.elevation/700.0).powi(2);
            let precipitation = (normal_loss + diff + elev_modifier).min(humidity);

            // sometimes precipitation evaporates
            // FUTURE: Shouldn't this depend on temperature?
            let evaporation = if precipitation > 1.5 { precipitation.min(10.0) } else { 0.0 };

            (
                precipitation,
                (humidity - precipitation + evaporation)
            )
        }

    } else {
        
        // FUTURE: no humidity change across permafrost? I'm not sure this is right. I know it gets too cold to snow sometimes, but there should be some precipitation, or there are no glaciers.
        (0.0,humidity)
    };

    let humidity = {
        tile.precipitation += tile_precipitation;
        if tile.precipitation > tile.factors.max_precipitation {
            let extra = (tile.precipitation - tile.factors.max_precipitation).min(tile_precipitation);
            tile.precipitation -= extra;
            humidity + extra
        } else {
            humidity
        }
    };

    Ok(humidity)
}

