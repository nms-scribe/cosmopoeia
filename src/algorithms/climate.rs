use std::collections::HashSet;

use angular_units::Deg;
use angular_units::Angle;

use crate::entity;
use crate::world_map::TileFeature;
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
use crate::world_map::NeighborAndDirection;
use crate::world_map::Neighbor;
use crate::world_map::IdRef;

pub(crate) fn generate_temperatures<Progress: ProgressObserver>(target: &mut WorldMapTransaction, temperatures: &TemperatureRangeArg, progress: &mut Progress) -> Result<(),CommandError> {

    /*

    I analyzed [WorldClim 2.1 data](https://www.worldclim.org/data/worldclim21.html), which includes global average monthly temperatures from 1970-2000. I created a point layer with points for every pixel on their height layer below 5 meters. Then I sampled values from their temperature layers for those points. The goal was to get a temperature curve which wasn't effected by elevation. I then averaged the temperatures by latitude of the points, and then again over the course of the year (weighted by days in the month, with Feb. getting 28.25).

    Using Veusz graphing software, I calculated a parabolic fit line. Although I might have gotten a better fit with a higher degree polynomial, the values were spread so far up and down that it wouldn't have been more accurate. It was definitely a polynomial curve, however. I got the following equation:
    
        y = -0.0070196132029816x^2 + 0.0022027112516246x + 26.674154895898

    And separated out the constants:

        a = -0.0070196132029816
        b = 0.0022027112516246
        c = 26.674154895898

    Since the curve was acceptable, I decided that such a curve would be fine for generating base temperatures in this algorithm. I had to figure out how to plug in the polar and equatorial temperatures into an equation to get a curve.

    I ran the equation to determine the values of y at -90, 0 and 90 latitude: -30.382956060899174, 26.674154895898, -29.986468035606746. That seems fairly symetrical within 0.5 degrees of temperature, so it seems like a symmetrical parabolic curve would be sufficient. This makes it easy to calculate. These calculations gave me my default values for polar and equatorial temperatures as well: -30 and 27 respectively.

    I can recreate a parabolic equation given three points. So, here are those points:

        (x1,y1) = (-90,P)
        (x2,y2) = (0,E)
        (x3,y3) = (90,P)

    Now, substitute these points into the quadratic equation (`y = ax^2 + bx + c) and solve the system. Where `P` is the polar temperature and `E` is the equatorial.

        1) P = c + -90b + 8100a
        2) E = c + 0b + 0a
        3) P = c + 90b + 8100a

    Except I don't have to solve the whole system, because there's an obvious shortcut from simplifying equation 2:

        2) E = c

    So I can just substitute that into the other two equations.

        1) P = E + -90b + 8100a
        3) P = E + 90b + 8100a

    Solving for a in the first equation:

        E + -90b + 8100a = P
        8100a = P - (E + -90b)
        8100a = P + -E + 90b
        a = (P + -E + 90b)/8100
        a = (P - E)/8100 + b/90

    And again in the second equation:

        E + 90b + 8100a = P
        8100a = P - (E + 90b)
        8100a = P + -E + -90b
        a = (P + -E + -90b)/8100
        a = (P - E)/8100 + -b/90

    There's a difference between the two occasions, which gives me a huge hunch about `b`. And knowing a little about how parabolas work, it's pretty obvious. But, I'm going to sustitute the first value of a into equation 1 to be sure, to solve for `b`:

        P = E + -90b + 8100((P - E)/8100 + b/90)
        P = E + -90b + P - E + 90b
        P = P

    Okay, apparently, I should have substituted that into the second equation instead.

        P = E + 90b + 8100((P - E)/8100 + b/90)
        P = E + 90b + P - E + 90b
        P = 90b + P + 90b
        P = 180b + P
        180b + P = P
        180b = P - P
        180b = 0
        b = 0

    Now, finally, I should be able to solve for `a` in terms of `E` and `P`. I substitute `b` in the formula for `a` above:

        a = (P - E)/8100 + 0/90
        a = (P - E)/8100

    And the equation for the parabola is:

        y = ((P - E)/8100)x^2 + E
    
    Thus the formula for determining the temperature (`T`) for a giving latitude (`L`) is:

        T = ((P - E)/8100)L^2 + E

    FUTURE: At some point in the future, in order to create more interesting climates, I might want to calculate seasonal averages instead. This is going to be a matter of adding a axial tilt value in and calculating new formulas. For reference for that, I did fit some curves to January and July temperatures, and got the following formulas:

        January) 25.136548984193 + -0.1670256346913*x + -0.0071296075931176*x**2
        July) 27.035956648176 + 0.16712408545881*x + -0.00613561121244*x**2

    The spring and fall formulas are very similar to the average formula above.

    `c` is still very close to the equatorial temperature, maybe a little cooler, and `a` is still very close to the same number, so I suspect the axial tilt is used to calculate `b`. I would have to solve the parabola equation at three points again, but the problem is trying to figure out what that middle latitude is going to be.

    */


    let mut layer = target.edit_tile_layer()?;

    let equator_temp = temperatures.equator_temp as f64;
    let polar_temp = temperatures.polar_temp as f64;

    let features = layer.read_features().into_entities_vec::<_,TileForTemperatures>(progress)?;

    for feature in features.iter().watch(progress,"Generating temperatures.","Temperatures calculated.") {

        let base_temp = ((polar_temp - equator_temp)/8100.0).mul_add(feature.site_y.powi(2),equator_temp);
        let adabiatic_temp = base_temp - if feature.grouping.is_ocean() {
            0.0
        } else {
            (feature.elevation/1000.0)*6.5
        };
        let temp = (adabiatic_temp*100.0).round()/100.0;

        let mut working_feature = layer.try_feature_by_id(&feature.fid)?; 
        
        working_feature.set_temperature(&temp)?;

        layer.update_feature(working_feature)?;




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
 
        let mut working_feature = layer.try_feature_by_id(&feature.fid)?;
        
        working_feature.set_wind(&wind_dir)?;

        layer.update_feature(working_feature)?;


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
        Ok(Self {
            lat_modifier,
            max_precipitation
        })

    }
}

entity!(TileDataForPrecipitation: Tile {
    elevation: f64,
    wind: Deg<f64>, 
    grouping: Grouping, 
    neighbors: Vec<NeighborAndDirection>,
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
    let mut working_queue: Vec<(IdRef,Option<f64>,IdRef)> = tile_map.keys().map(|id| (id.clone(),None,id.clone())).collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_queue.sort_by_cached_key(|(id,_,_)| id.clone());
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
            for NeighborAndDirection(fid,direction) in &tile.neighbors {
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
                    best_neighbors.push(fid.clone())
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
                    if !visited.insert((start_id.clone(),next_fid.clone())) {
                        continue;
                        // set already contained the value, so we've reached one we've already visited, I don't want to go in circles.
                    } 
    

                    match next_fid {
                        Neighbor::Tile(next_fid) | Neighbor::CrossMap(next_fid,_) => {
   
                            let mut next = tile_map.try_get(&next_fid)?.clone(); // I'm cloning so I can make some changes without messing with the original.

                            let humidity = precipitate(&mut tile, Some(&mut next), humidity);
        
                            let real_current = tile_map.try_get_mut(&tile_id)?;
                            real_current.precipitation = tile.precipitation;
                
                            let real_next = tile_map.try_get_mut(&next_fid)?;
                            real_next.precipitation = next.precipitation;
                
                            working_queue.push((next_fid,Some(humidity),start_id.clone()));                        
                        }
                        Neighbor::OffMap(_) => {
                            // the humidity spreads off of the map
                            _ = precipitate(&mut tile, None, humidity);

                            let real_current = tile_map.try_get_mut(&tile_id)?;
                            real_current.precipitation = tile.precipitation;

                        }
                    }
 
    
                }

            }


        } else {
            // there is no humidity left to work with.
        }

    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing precipitation.","Precipitation written.") {
        let mut working_feature = layer.try_feature_by_id(fid)?; 
        
        working_feature.set_precipitation(&tile.precipitation)?;

        layer.update_feature(working_feature)?;


    }

    Ok(())
}

fn precipitate(tile: &mut TileDataForPrecipitation, next: Option<&mut TileDataForPrecipitation>, humidity: f64) -> f64 {

    // Many of these calculations were taken from AFMG and I don't know where they got that.
    // FUTURE: I would love if someone could give me some better calculations, as I feel there are some things missing here compared to what I learned in school.
    // - The max_precipitation factor seems wrong. It should be a max_humidity, and once you go over that it becomes precipitation... i.e. dewpoint.
    //   - said max_humidity should depend on temperature and elevation.
    // - temperature change should increase precipitation (and might be the real reason for the coastal precipitation)
    // - elevation precipitation should be based on the difference in elevation, not the elevation scaled.


    let (tile_precipitation,humidity) = if tile.temperature >= -5.0 { 
        if tile.grouping.is_ocean() {
            if let Some(next) = next {
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
                // it's going off the map, no changes
                (0.0,humidity)
            }
        } else {
            // precipitation under normal conditions
            let normal_loss = humidity / (10.0 * tile.factors.lat_modifier);
            let (diff,elev_modifier) = if let Some(next) = next {
                (
                    // difference in height
                    (next.elevation - tile.elevation).max(0.0)/100.0,
                    // additional modifier for high elevation of mountains
                    (next.elevation/700.0).powi(2)
                )
            } else {
                // off the map, assume the same height
                (
                    0.0,
                    (tile.elevation/700.0).powi(2)
                )
            };
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

    tile.precipitation += tile_precipitation;
    if tile.precipitation > tile.factors.max_precipitation {
        let extra = (tile.precipitation - tile.factors.max_precipitation).min(tile_precipitation);
        tile.precipitation -= extra;
        humidity + extra
    } else {
        humidity
    }
}

