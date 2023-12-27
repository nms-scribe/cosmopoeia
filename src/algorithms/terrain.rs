use std::collections::HashMap;
use std::collections::VecDeque;
use std::collections::HashSet;
use core::cmp::Reverse;

use rand::Rng;
use ordered_float::OrderedFloat;

use crate::errors::CommandError;
use crate::typed_map::entities::EntityIndex;
use crate::world_map::tile_layer::TileSchema;
use crate::world_map::tile_layer::TileForTerrain;
use crate::world_map::tile_layer::TileFeature;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::raster::RasterMap;
use crate::world_map::fields::Grouping;
use crate::world_map::property_layer::ElevationLimits;
use crate::utils::point_finder::TileFinder;
use crate::utils::coordinates::Coordinates;
use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use crate::progress::WatchableDeque;
use crate::progress::WatchableQueue;
use crate::commands::terrain::Multiply;
use crate::utils::arg_range::ArgRange;
use crate::commands::terrain::ClearOcean;
use crate::commands::terrain::RandomUniform;
use crate::commands::terrain::AddHill;
use crate::commands::terrain::AddRange;
use crate::commands::terrain::AddStrait;
use crate::commands::terrain::StraitDirection;
use crate::commands::terrain::Mask;
use crate::commands::terrain::Invert;
use crate::commands::terrain::InvertAxes;
use crate::commands::terrain::Add;
use crate::commands::terrain::Smooth;
use crate::commands::terrain::Erode;
use crate::commands::terrain::SeedOcean;
use crate::commands::terrain::FloodOcean;
use crate::commands::terrain::FillOcean;
use crate::entity;
use crate::algorithms::tiles::find_lowest_tile;
use crate::world_map::fields::NeighborAndDirection;
use crate::world_map::fields::Neighbor;
use crate::typed_map::fields::IdRef;


enum RelativeHeightTruncation {
    Floor,
    Ceil,
    UnTruncated,
}

struct TerrainParameters {
    elevations: ElevationLimits,
    world_shape: WorldShape,
    positive_elevation_scale: f64,
    negative_elevation_scale: f64,
    expanse_above_sea_level: f64,
    blob_power: f64,
    line_power: f64, 
    extents: Extent
}

impl TerrainParameters {

    const fn get_blob_power(tile_count: usize) -> f64 {
        // These numbers came from AFMG
        match tile_count {
            0..=1001 => 0.93,
            1002..=2001 => 0.95,
            2002..=5001 => 0.97,
            5002..=10001 => 0.98,
            10002..=20001 => 0.99,
            20002..=30001 => 0.991,
            30002..=40001 => 0.993,
            40002..=50001 => 0.994,
            50002..=60001 => 0.995,
            60002..=70001 => 0.9955,
            70002..=80001 => 0.996,
            80002..=90001 => 0.9964,
            90002..=100001 => 0.9973,
            _ => 0.998
        }        
    }

    const fn get_line_power(tile_count: usize) -> f64 {
        match tile_count {
            0..=1001 => 0.75,
            1002..=2001 => 0.77,
            2002..=5001 => 0.79,
            5002..=10001 => 0.81,
            10002..=20001 => 0.82,
            20002..=30001 => 0.83,
            30002..=40001 => 0.84,
            40002..=50001 => 0.86,
            50002..=60001 => 0.87,
            60002..=70001 => 0.88,
            70002..=80001 => 0.91,
            80002..=90001 => 0.92,
            90002..=100001 => 0.93,
            _ => 0.94
        }
    }
        

    fn new(world_shape: WorldShape, elevations: ElevationLimits, extents: Extent, tile_count: usize) -> Self {
        let expanse_above_sea_level = elevations.max_elevation - (elevations.min_elevation.max(0.0));
        let blob_power = Self::get_blob_power(tile_count);
        let line_power = Self::get_line_power(tile_count);

        let positive_elevation_scale = 80.0/elevations.max_elevation;
        let negative_elevation_scale = if elevations.min_elevation < 0.0 { 
            20.0/elevations.min_elevation.abs()
        } else {
            0.0
        };

        Self { 
            elevations, 
            world_shape,
            positive_elevation_scale, 
            negative_elevation_scale, 
            expanse_above_sea_level, 
            blob_power, 
            line_power, 
            extents 
        }

    }

    /// whatever
    fn gen_x<Random: Rng>(&self, rng: &mut Random, range: &ArgRange<f64>) -> f64 {
        let x = ((range.choose(rng) / 100.0) * self.extents.width).clamp(0.0, self.extents.width);
        self.extents.west + x
    }

    fn gen_y<Random: Rng>(&self, rng: &mut Random, range: &ArgRange<f64>) -> f64 {
        let y = ((range.choose(rng) / 100.0) * self.extents.height).clamp(0.0, self.extents.height);
        self.extents.south + y
    }

    fn get_height_delta(&self, height_delta: i8) -> (f64,f64) {
        // convert the delta relative to the above sea level range, rather than below, so the
        // input to convert needs to be positive.
        let (height_delta,sign) = if height_delta.is_negative() {
            (height_delta.abs(),-1.0)
        } else {
            (height_delta,1.0)
        };
        let result = self.convert_relative_height(height_delta, &RelativeHeightTruncation::UnTruncated,false);
        (result,sign)

    }

    fn get_signed_height_delta(&self, height_delta: i8) -> f64 {
        let (value,sign) = self.get_height_delta(height_delta);
        value.copysign(sign)
    }

    fn gen_height_delta<Random: Rng>(&self, rng: &mut Random, height_delta: &ArgRange<i8>) -> (f64,f64) {
        let chosen = height_delta.choose(rng);
        self.get_height_delta(chosen)
    }

    fn gen_signed_height_delta<Random: Rng>(&self, rng: &mut Random, height_delta: &ArgRange<i8>) -> f64 {
        let (value,sign) = self.gen_height_delta(rng, height_delta);
        value.copysign(sign)
    }



    fn convert_relative_height(&self, value: i8, direction: &RelativeHeightTruncation, clamp: bool) -> f64 {
        let max_elevation = self.elevations.max_elevation;
        let min_elevation = self.elevations.min_elevation;
        let result = if value == 100 {
            max_elevation
        } else if value == -100 {
            min_elevation
        } else {
            let fraction = match direction {
                RelativeHeightTruncation::Floor => (value as f64/100.0).floor(),
                RelativeHeightTruncation::Ceil => (value as f64/100.0).ceil(),
                RelativeHeightTruncation::UnTruncated => value as f64/100.0,
            };
            if value >= 0 {
                fraction * max_elevation
            } else if min_elevation < 0.0 {
                -fraction * min_elevation
            } else {
                0.0
            }
        };
        if clamp {
            result.clamp(min_elevation, max_elevation)
        } else {
            result
        }
    }

    fn convert_height_filter(&self, height_filter: &Option<ArgRange<i8>>) -> ArgRange<f64> {
        match height_filter {
            Some(ArgRange::Inclusive(min, max)) => ArgRange::Inclusive(
                self.convert_relative_height(*min, &RelativeHeightTruncation::Floor,true), 
                self.convert_relative_height(*max, &RelativeHeightTruncation::Ceil,true)
            ),
            Some(ArgRange::Exclusive(min, max)) => ArgRange::Exclusive(
                self.convert_relative_height(*min, &RelativeHeightTruncation::Floor,true), 
                self.convert_relative_height(*max, &RelativeHeightTruncation::Ceil,true)
            ),
            Some(ArgRange::Single(single)) => {
                let single = *single;
                ArgRange::Inclusive(
                    self.convert_relative_height(single, &RelativeHeightTruncation::Floor,true), 
                    self.convert_relative_height(single, &RelativeHeightTruncation::Ceil,true)
                )
            },
            None => ArgRange::Inclusive(self.elevations.min_elevation, self.elevations.max_elevation)
        }
    }

    fn is_elevation_within(&self, h: f64, limit_fraction: f64) -> bool {
        h <= (self.elevations.max_elevation * limit_fraction) &&
        if self.elevations.min_elevation < 0.0 {
            h >= (self.elevations.min_elevation * limit_fraction)
        } else {
            h >= self.expanse_above_sea_level.mul_add(-limit_fraction, self.elevations.max_elevation)
        }

    }

    fn clamp_elevation(&self, elevation: f64) -> f64 {
        elevation.clamp(self.elevations.min_elevation, self.elevations.max_elevation)
    }

    fn scale_elevation(&self, elevation: f64) -> i32 {
        if elevation >= 0.0 {
            20 + (elevation * self.positive_elevation_scale).floor() as i32
        } else {
            20 - (elevation.abs() * self.negative_elevation_scale).floor() as i32
        }.clamp(0,100)
    }

    fn gen_end_y<Random: Rng>(&self, rng: &mut Random) -> f64 {
        self.extents.height.mul_add(0.15, rng.gen_range(0.0..(self.extents.height * 0.7)) + self.extents.south)
    }
    
    fn gen_end_x<Random: Rng>(&self, rng: &mut Random) -> f64 {
        self.extents.width.mul_add(0.1, rng.gen_range(0.0..(self.extents.width * 0.8)) + self.extents.west)
    }

    
}

trait ProcessTerrainTiles {

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError>;

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, _: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        // I've added this function to make it easier to change requirements later.
        self.process_terrain_tiles(rng, parameters, tile_map, progress)
    }


    // This is mostly for quick runtime checking of which interface a command provides.
    fn requires_point_index(&self) -> bool {
        false
    }

}

trait ProcessTerrainTilesWithPointIndex {

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError>;

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, _: &mut EntityIndex<TileSchema,TileForTerrain>, _: &mut Progress) -> Result<(),CommandError> {
        // I've added this function to make it easier to change task requirements later. If I were to put this unimplemented
        // in the match statement that calls process_terrain_tiles, I might forget to change it to call this function instead. This
        // way, if I change the requirements later, I just have to change what trait is implemented and the function being implemented.
        unreachable!("Code never should have called this.")
    }


    fn requires_point_index(&self) -> bool {
        true
    }
}

pub(crate) trait LoadTerrainTask {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError>;
}



pub(crate) struct SampleOceanBelowLoaded {
    raster: RasterMap,
    elevation: f64
}

impl SampleOceanBelowLoaded {

    pub(crate) const fn new(raster: RasterMap, elevation: f64) -> Self {
        Self {
            raster,
            elevation,
        }
    }
}

impl ProcessTerrainTiles for SampleOceanBelowLoaded {

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling ocean data");

        progress.start_unknown_endpoint(|| "Reading raster");

        let band = self.raster.read_band::<f64>(1)?;
        let bounds = self.raster.bounds()?;
        let no_data_value = band.no_data_value();
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling oceans.","Oceans sampled.") {
    
            let (tile_x,tile_y) = tile.site.to_tuple();
            let (x,y) = bounds.coords_to_pixels(tile_x, tile_y);

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                let is_no_data = match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => elevation.is_nan(),
                    Some(no_data_value) => (elevation - no_data_value).abs() < f64::EPSILON,
                    None => false,
                };

                if is_no_data {
                    false
                } else {
                    elevation < &self.elevation
                }


            } else {

                false

            };

            // only apply if the data actually is ocean now, so one can use multiple ocean methods
            if is_ocean {
                tile.grouping = Grouping::Ocean;
            }

        }
    
        Ok(())        
    }
}

pub(crate) struct SampleOceanMaskedLoaded {
    raster: RasterMap
}

impl SampleOceanMaskedLoaded {

    pub(crate) const fn new(raster: RasterMap) -> Self {
        Self {
            raster
        }
    }
}



impl ProcessTerrainTiles for SampleOceanMaskedLoaded {

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling ocean data");

        progress.start_unknown_endpoint(|| "Reading raster");

        let band = self.raster.read_band::<f64>(1)?;
        let bounds = self.raster.bounds()?;
        let no_data_value = band.no_data_value();
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling oceans.","Oceans sampled.") {
    
            let (tile_x,tile_y) = tile.site.to_tuple();
            let (x,y) = bounds.coords_to_pixels(tile_x, tile_y);

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => !elevation.is_nan(),
                    Some(no_data_value) => (elevation - no_data_value).abs() > f64::EPSILON,
                    None => true,
                }

            } else {

                false

            };

            // only apply if the data actually is ocean now, so one can use multiple ocean methods
            if is_ocean {
                tile.grouping = Grouping::Ocean;
            }

        }
    
        Ok(())
    }
}

pub(crate) struct SampleElevationLoaded {
    raster: RasterMap
}

impl SampleElevationLoaded {
    pub(crate) const fn new(raster: RasterMap) -> Self {
        Self {
            raster
        }
    }
}

impl ProcessTerrainTiles for SampleElevationLoaded {

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling elevations from raster.");

        progress.start_unknown_endpoint(|| "Reading raster");

        let raster = &self.raster;

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling elevations.","Elevations sampled.") {
    
            let (tile_x,tile_y) = tile.site.to_tuple();
            let (x,y) = bounds.coords_to_pixels(tile_x, tile_y);

            if let Some(elevation) = band.get_value(x, y) {

                tile.elevation = *elevation;
    
            }
    
    
        }

        Ok(())
    }
}


impl ProcessTerrainTilesWithPointIndex for AddHill {

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        
        let count = self.count.choose(rng);

        progress.announce(&format!("Generating {count} hills."));


        for i in 0..count {
            let mut change_map = HashMap::new();
            let (height_delta,sign) = parameters.gen_height_delta(rng, &self.height_delta);

            let mut start;
            let mut limit = 0;
            loop {
                let x = parameters.gen_x(rng, &self.x_filter);
                let y = parameters.gen_y(rng, &self.y_filter);
                start = point_index.find_nearest_tile(&(x,y).try_into()?)?;
                let start_tile = tile_map.try_get(&start)?;

                if (limit >= 50) || parameters.is_elevation_within(start_tile.elevation + height_delta.copysign(sign),0.9) {
                    break;
                }
                limit += 1;
            }

            _ = change_map.insert(start.clone(),height_delta);
            let mut queue = VecDeque::from([start.clone()]).watch_queue(progress,format!("Generating hill #{}.",i+1),format!("Hill #{} generated.",i+1));

            while let Some(tile_id) = queue.pop_front() {
                let tile = tile_map.try_get(&tile_id)?;
                let last_change = *change_map.get(&tile_id).expect("How could there be something in the queue if it wasn't added to this map?"); 
                for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {

                    match neighbor_id {
                        Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                            if change_map.contains_key(neighbor_id) {
                                continue;
                            }

                            let neighbor_height_delta = last_change.powf(parameters.blob_power) * (rng.gen_range(0.0..0.2) + 0.9);
                            _ = change_map.insert(neighbor_id.clone(), neighbor_height_delta);
                            if neighbor_height_delta > 1.0 { 
                                queue.push_back(neighbor_id.clone())
                            }
                        }
                        Neighbor::OffMap(_) => (),
                    } // else it's off the map

                }
            }

            for (tile_id,calculated_height_delta) in change_map {
                tile_map.try_get_mut(&tile_id)?.elevation += calculated_height_delta.copysign(sign);
            }

        }

        Ok(())

    }
}

impl ProcessTerrainTilesWithPointIndex for AddRange {

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        let count = self.count.choose(rng);

        progress.announce(&format!("Generating {count} ranges."));

        let lower_dist_limit = parameters.extents.width / 8.0;
        let upper_dist_limit = parameters.extents.width / 3.0;

        for i in 0..count {
            // add one range

            let mut used = HashSet::new();
            let (mut height_delta,sign) = parameters.gen_height_delta(rng, &self.height_delta);

            // find start and end points
            let start_x = parameters.gen_x(rng, &self.x_filter);
            let start_y = parameters.gen_y(rng, &self.y_filter);
            let start_point: Coordinates = (start_x, start_y).try_into()?;
            let mut end_point;

            // find an end point that's far enough away
            let mut limit = 0;
            loop {
                let end_x = parameters.gen_end_x(rng);
                let end_y = parameters.gen_end_y(rng);
                end_point = (end_x, end_y).try_into()?;
                let dist = start_point.shaped_distance(&end_point,&parameters.world_shape);
                if (limit >= 50) || (dist >= lower_dist_limit) && (dist <= upper_dist_limit) {
                    break;
                }
                limit += 1;

            }

            let start = point_index.find_nearest_tile(&start_point)?;
            let end = point_index.find_nearest_tile(&end_point)?;

            progress.start_unknown_endpoint(|| format!("Generating range #{}.",i+1));
            let range = get_range(rng, tile_map, &parameters.world_shape, &mut used, start, &end, 0.85)?;

            // add height to ridge and neighboring cells
            let mut queue = range.clone();
            let mut spread_count = 0;

            while !queue.is_empty() {
                let frontier = core::mem::replace(&mut queue, Vec::new());
                spread_count += 1;
                for tile_id in frontier {
                    tile_map.try_get_mut(&tile_id)?.elevation += (height_delta * (rng.gen_range(0.0..0.3) + 0.85)).copysign(sign);
                    for NeighborAndDirection(neighbor_id,_) in &tile_map.try_get(&tile_id)?.neighbors {
                        match neighbor_id {
                            Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                                if !used.contains(neighbor_id) {
                                    queue.push(neighbor_id.clone());
                                    _ = used.insert(neighbor_id.clone());
                                }

                            }
                            Neighbor::OffMap(_) => (),
                        } // else ignore off the map
                    }
                }
                height_delta = height_delta.powf(parameters.line_power) - 1.0;
                if height_delta < 2.0 {
                    break;
                }

            }

            // create some prominences in the range.
            for (j,mut current_id) in range.into_iter().enumerate() {
                if (j % 6) != 0 {
                    continue;
                }
                for _ in 0..spread_count {
                    let current = tile_map.try_get(&current_id)?;
                    let current_elevation = current.elevation;
                    let mut min_elevation = None;
                    for NeighborAndDirection(neighbor_id,_) in &current.neighbors {
                        match neighbor_id {
                            Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                                let neighbor = tile_map.try_get(neighbor_id)?;
                                let elevation = neighbor.elevation;
                                match min_elevation {
                                    None => min_elevation = Some((neighbor_id.clone(),elevation)),
                                    Some((_,prev_elevation)) => if elevation < prev_elevation {
                                        min_elevation = Some((neighbor_id.clone(),elevation))
                                    }
                                }
                            }
                            Neighbor::OffMap(_) => (),
                        } // else ignore off the map
                    }
                    if let Some((min_tile_id,elevation)) = min_elevation {
                        tile_map.try_get_mut(&min_tile_id)?.elevation = current_elevation.mul_add(2.0, elevation.copysign(sign)) / 3.0;
                        current_id = min_tile_id;
                    } else {
                        break;
                    }
                    
                }

            }
            progress.finish(|| format!("Range #{} generated.",i+1));



        }

      
        Ok(())
    }
}

fn get_range<Random: Rng>(rng: &mut Random, tile_map: &mut EntityIndex<TileSchema, TileForTerrain>, world_shape: &WorldShape, used: &mut HashSet<IdRef>, start: IdRef, end: &IdRef, jagged_probability: f64) -> Result<Vec<IdRef>, CommandError> {
    let mut cur_id = start;
    let end_tile = tile_map.try_get(end)?;
    let mut range = vec![cur_id.clone()];
    _ = used.insert(cur_id.clone());
    while &cur_id != end {
        let mut min = f64::INFINITY;
        let cur_tile = tile_map.try_get(&cur_id)?;
        // basically, find the neighbor that is closest to the end
        for NeighborAndDirection(neighbor_id,_) in &cur_tile.neighbors {
            match neighbor_id {
                Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                    if used.contains(neighbor_id) {
                        continue;
                    }

                    let neighbor_tile = tile_map.try_get(neighbor_id)?;
                    let diff = end_tile.site.shaped_distance(&neighbor_tile.site,world_shape);
                    let diff = if rng.gen_bool(jagged_probability) {
                        // every once in a while, make the neighbor seem closer, to create more jagged ridges.
                        diff / 2.0
                    } else {
                        diff
                    };
                    if diff < min {
                        min = diff;
                        cur_id = neighbor_id.clone();
                    }
                }
                Neighbor::OffMap(_) => (),
            } // else ignore off the map
        }
        if min.is_infinite() { // no neighbors at all were found?
            break;
        }
        range.push(cur_id.clone());
        _ = used.insert(cur_id.clone());
    }
    Ok(range)

}



impl ProcessTerrainTilesWithPointIndex for AddStrait {
    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {


        // NOTE: I feel like this isn't as nice as the others:
        // 1. You can't specify where it begins and ends
        // 2. You can't specify the height delta, including whether it is a raise or not.
        // 3. The path is too straight, and the widths are too even.
        // 4. The depths are very drastic.
        // 5. It tends to cut across the entire map, which is going to be a problem when I get this stuff to work better with polar coordinates and distances.
        // -- I might just deprecate this and come up with something more like range -- I think range can do it with some changes
        //    to how gradual the sloping is, and how long the results are.

        // find the estimated number of tiles across
        // x/y = width/height
        // x*y = tile_count
        // y = tile_count/x
        // x/(tile_count/x) = width/height
        // x^2/tile_count = width/height
        // x^2 = (width/height)*tile_count
        // x = sqrt((width/height)*tile_count)
        let tiles_x = ((parameters.extents.width/parameters.extents.height)*tile_map.len() as f64).sqrt();

        // don't let it get more than one third as wide as the map.
        let mut width = self.width.choose(rng).min(tiles_x/3.0);
        // if it's too small, return
        if width < 1.0 && rng.gen_bool(width) {
            progress.announce("Strait improbable, will not be generated.");
            return Ok(())
        }
        progress.announce("Generating strait.");

        let mut used = HashSet::new();
        let e_width = parameters.extents.width;
        let e_height = parameters.extents.height;
        let e_south = parameters.extents.south;
        let e_west = parameters.extents.west;
        let (start_x,start_y,end_x,end_y) = match self.direction {
            StraitDirection::Vertical => {
                let start_x = e_width.mul_add(0.3, rng.gen_range(0.0..(e_width * 0.4)));
                let start_y = 5.0;
                let end_x = e_width.mul_add(-0.1, e_width - start_x + rng.gen_range(0.0..(e_width * 0.2)));
                let end_y = e_height - 5.0;
                (start_x,start_y,end_x,end_y)
            },
            StraitDirection::Horizontal => {
                let start_x = 5.0;
                let start_y = e_height.mul_add(0.3, rng.gen_range(0.0..(e_height * 0.4)));
                let end_x = e_width - 5.0;
                let end_y = e_height.mul_add(-0.1, e_height - start_y + rng.gen_range(0.0..(e_height * 0.2)));
                (start_x,start_y,end_x,end_y)
            },
        };
        let start_point = (start_x + e_west, start_y + e_south).try_into()?;
        let end_point = (end_x + e_west, end_y + e_south).try_into()?;

        let start = point_index.find_nearest_tile(&start_point)?;
        let end = point_index.find_nearest_tile(&end_point)?;

        let mut range = get_range(rng, tile_map, &parameters.world_shape, &mut used, start, &end, 0.8)?;

        let mut next_queue = Vec::new();

        let step = 0.1/width;

        let progress_width = width.ceil() as usize;
        progress.start_known_endpoint(|| ("Generating strait.",progress_width));

        while width > 0.0 {

            let exp = step.mul_add(-width, 0.99);
            for tile_id in &range {
                let tile = tile_map.try_get(tile_id)?;
                // NOTE: For some reason the AFMG code for this didn't change the elevation for the first row,
                // because it's version of get_range (a special one just for this routine) didn't mark them
                // as used. However, that's still going to create a ridge down the middle, since they'll
                // be marked in the second tier.
                // -- I'm just explaining this in case anyone looks at this.

                // can't do a fractional power on a negative number, so do it on a positive.
                let old_elevation_diff = tile.elevation - parameters.elevations.min_elevation;
                let new_elevation_diff = old_elevation_diff.powf(exp);
                let mut new_elevation = parameters.elevations.min_elevation + new_elevation_diff;
                if new_elevation > parameters.elevations.max_elevation {
                    // I'm not exactly sure what this is doing, but it's taken from AFMG
                    new_elevation = parameters.expanse_above_sea_level.mul_add(0.5, parameters.elevations.min_elevation);
                }

                for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {
                    match neighbor_id {
                        Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                            if used.contains(neighbor_id) {
                                continue;
                            }
                            _ = used.insert(neighbor_id.clone());
                            next_queue.push(neighbor_id.clone());
                        }
                        Neighbor::OffMap(_) => (),
                    } // else ignore off the map
                }

                tile_map.try_get_mut(tile_id)?.elevation = new_elevation;
            }
            range = core::mem::replace(&mut next_queue, Vec::new());

            width -= 1.0;
            progress.update(|| progress_width - (width.ceil() as usize));
        }
        progress.finish(|| "Strait generated.");

        Ok(())
    }
}

impl ProcessTerrainTiles for Mask {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        // I'm not sure what this is actually supposed to do. I would expect a "mask" to mask out based on a heightmap,
        // not distance from edge. It does change the map, however, so maybe it works?

        progress.announce("Masking elevations.");

        let factor = self.power.abs();

        for (_,tile) in tile_map.iter_mut().watch(progress, "Computing mask.", "Mask computed.") {

            let (x,y) = tile.site.to_tuple();
            let x = x - parameters.extents.west;
            let y = y - parameters.extents.south;

            let nx = (x * 2.0) / parameters.extents.width - 1.0; // -1<--:0:-->1
            let ny = (y * 2.0) / parameters.extents.height - 1.0; // -1<--:0:-->1
            let mut distance = nx.mul_add(-nx, 1.0) * ny.mul_add(-ny, 1.0); // 0<--:1:-->0
            if self.power.is_sign_negative() {
                distance = 1.0 - distance; // inverted, // 1<--:0:-->1
            }
            let masked = tile.elevation * distance;
            let new_elevation = tile.elevation.mul_add(factor - 1.0, masked)/factor;

            tile.elevation = new_elevation;

        }

        Ok(())

    }
}


impl ProcessTerrainTilesWithPointIndex for Invert {
    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        // NOTE: While this is a good simplification of the process, it's not a true inversion, and over each pass tends to scatter. Inverting the actual
        // geometry of the points of the tiles would be more accurate and might be a useful tool at some point. But since this is usually intended for
        // randomly generated land, it shouldn't matter too much.

        // FUTURE: This is slower than I expected. Caching the point index search for the other side only reduced the time by about 10-20%. Yes, I'm
        // touching every tile, which is kind of slow, but it's not like some of the other things aren't touching a large number of tiles at once,
        // and don't take this long.
        // -- The problem appears to be in the point index. I wonder if there's some way to speed that up.

        if !rng.gen_bool(self.probability) {
            progress.announce("Inversion improbable, will not be completed.");
            return Ok(());

        }

        progress.announce("Inverting elevations.");

        // I can't modify the elevations inline as I need access to the other tile elevations as I do it.
        let mut inverted_heights = Vec::new();
        let mut switches = HashMap::new();

        for (fid,tile) in tile_map.iter().watch(progress, "Inverting elevations.", "Elevations inverted.") {
            let (x,y) = tile.site.to_tuple();

            macro_rules! switch_x {
                () => {{
                    let x = x - parameters.extents.west;
                    let switch_x = parameters.extents.width - x;
                    parameters.extents.west + switch_x
                }};
            }

            macro_rules! switch_y {
                () => {{
                    let y = y - parameters.extents.south;
                    let switch_y = parameters.extents.height - y;
                    parameters.extents.south + switch_y
                }};
            }

            // reducing this down to one check on self.axes did not produce significant speed improvements
            let (switch_x, switch_y) = match self.axes {
                InvertAxes::X => (switch_x!(),y),
                InvertAxes::Y => (x,switch_y!()),
                InvertAxes::Both => (switch_x!(),switch_y!()),
            };

            let switch_point = (switch_x, switch_y).try_into()?;

            // cache the calculation
            let switch_tile_id = match switches.get(fid) {
                None => {
                    // NOTE: This is where the most time is spent. Removing this and setting switch_tile_id to a constant value 
                    // sped up things about 90%. Of course, it also broke the algorithm.
                    let switch_tile_id = point_index.find_nearest_tile(&switch_point)?;
                    _ = switches.insert(switch_tile_id.clone(), fid.clone());     
                    switch_tile_id               
                },
                Some(id) => id.clone(),
            };


            let switch_tile = tile_map.try_get(&switch_tile_id)?;

            // removing this command did not produce significant speed improvements for this part of the progress,
            // so this isn't adding to the time. (And would have broken the algorithm)
            inverted_heights.push((fid.clone(), switch_tile.elevation));

        }

        for (fid,elevation) in inverted_heights.into_iter().watch(progress, "Writing inversions.", "Inversions written.") {
            tile_map.try_get_mut(&fid)?.elevation = elevation;
        }

        Ok(())


    }

}


impl ProcessTerrainTiles for Add {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce(&format!("Adding {} to some elevations.",self.height_delta));

        let filter = parameters.convert_height_filter(&self.height_filter);
        let height_delta = parameters.get_signed_height_delta(self.height_delta);

        for (_,tile) in tile_map.iter_mut().watch(progress, "Adding heights.", "Heights added.") {

            if filter.includes(&tile.elevation) {
                tile.elevation += height_delta;
            }
        }

        Ok(())

    }
}


impl ProcessTerrainTiles for Multiply {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        progress.announce(&format!("Multiplying some elevations by {}.",self.height_factor));

        let filter = parameters.convert_height_filter(&self.height_filter);

        for (_,tile) in tile_map.iter_mut().watch(progress, "Multiplying heights.", "Heights multiplied.") {

            if filter.includes(&tile.elevation) {
                tile.elevation *= self.height_factor;
            }
        }

        Ok(())

    }
}


impl ProcessTerrainTiles for Smooth {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Smoothing heights.");

        // I need to know the heights of different tiles, so I can't update heights inline.
        let mut changed_heights = Vec::new();

        for (fid,tile) in tile_map.iter().watch(progress, "Finding averages.", "Averages found.") {
            let mut heights = vec![tile.elevation];
            for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {
                match neighbor_id {
                    Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                        let neighbor = tile_map.try_get(neighbor_id)?;
                        heights.push(neighbor.elevation);
                    }
                    Neighbor::OffMap(_) => (),
                } // ignore off the map
            }
            let average = heights.iter().sum::<f64>()/heights.len() as f64;
            let new_height = if (self.fr - 1.0).abs() < f64::EPSILON {
                average
            } else {
                parameters.clamp_elevation(tile.elevation.mul_add(self.fr - 1.0, average) / self.fr)
            };
            changed_heights.push((fid.clone(),new_height));
        }

        for (fid,elevation) in changed_heights.into_iter().watch(progress, "Writing heights.", "Heights written.") {
            tile_map.try_get_mut(&fid)?.elevation = elevation;
        }

        Ok(())

    }
}

impl ProcessTerrainTiles for Erode {

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        
        entity!(TileForSoil: Tile {
            site: Coordinates,
            elevation: f64, 
            neighbors: Vec<NeighborAndDirection>,
            soil: f64 = |_| Ok::<_,CommandError>(0.0)
        });

        let mut tile_list = Vec::new();

        let weathering_amount = self.weathering_amount;

        progress.announce(&format!("Eroding {weathering_amount} of elevation {} time(s).",self.iterations));

        // The number of iterations that might need to be done make using a single progress bar more useful
        progress.start_known_endpoint(|| ("Weathering and eroding.",tile_map.len() * 2 * self.iterations));
        let mut update_count = 0;

        let mut soil_map = EntityIndex::from_iter(tile_map.iter().map(|(fid,tile)| {
            update_count += 1;
            progress.update(|| update_count);

            let elevation = tile.elevation - weathering_amount;
            tile_list.push(fid.clone());
            (fid.clone(),TileForSoil {
                site: tile.site.clone(),
                neighbors: tile.neighbors.clone(),
                elevation,
                soil: weathering_amount
            })
        }));
        

        for iteration in 0..self.iterations {

            // sort by elevation, with highest at the top;
            tile_list.sort_by_cached_key(|fid| {
                let entity = soil_map.try_get(fid).expect("How could there be a key that's not in the soil map?");
                Reverse(OrderedFloat(entity.elevation + entity.soil))
            });


            for fid in &tile_list {
                update_count += 1;
                progress.update(|| update_count);
    
                let entity = soil_map.try_get(fid)?;

                // this is very similar to algorithms::tiles::find_flowingest_tile, but 1) I need to include soil in the result and 2) I'm more interested in steepness than depth and 3) Even if I even used the closures to replace the elevation with steepness somehow, I'm looking at the highest levels instead.
                let (steepest_neighbors,steepest_grade) = find_lowest_tile(entity, &soil_map, |t| {
                    match t {
                        Some((t,across_map)) => {
                            // calculate it backwards, because the algorithm finds the lowest value.
                            let rise = (t.elevation + t.soil) - (entity.elevation + entity.soil);
                            // the meridian distance (between two degrees latitude) on a sphere with the mean radius of Earth is 111.2km.
                            // FUTURE: Once I get "spheremode" I will have to use that to calculate the distance.
                            // FUTURE: Another issue I'm going to have: the grades are going to be steeper for smaller tile sizes. However, I might not need to account for this, because the extra relief will smooth out over iterations.
                            let run = if across_map {
                                entity.site.shaped_distance(&t.site.across_antimeridian(&entity.site),&parameters.world_shape)
                            } else {
                                entity.site.shaped_distance(&t.site,&parameters.world_shape)
                            } * 111200.0;
                            rise/run
                        },
                        // else the tile is off the map. I feel like the best results will be found if off-the-map is assumed to be
                        // the same level. If there are lower places around, then we won't go here, but if there is soil coming in
                        // then it can pile up here.
                        None => 0.0,
                    }
                }, |t| &t.neighbors)?;

                if let Some(steepest_grade) = steepest_grade {
                    // remember, the algorithm returned the *lowest*, so the one we're after is actually less than zero.
                    if steepest_grade < 0.0 {

                        // Grade is a percent. The following shifts all the soil if it's only 45 degrees, but this is much less likely than you'd think.
                        let shift_soil = (steepest_grade.abs() * entity.soil).min(entity.soil);
                        
                        soil_map.try_get_mut(fid)?.soil -= shift_soil;

                        let shift_soil = shift_soil / steepest_neighbors.len() as f64;

                        for neighbor_id in steepest_neighbors {
                            match neighbor_id {
                                Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                                    soil_map.try_get_mut(&neighbor_id)?.soil += shift_soil;
                                }
                                Neighbor::OffMap(_) => (),
                            } // else just let it fall off the map
                        }

                    }


                } // otherwise, no lower neighbors were found, so just leave any soil there.


            }

            if iteration < self.iterations - 1 {
                // re-weather for next iteration

                for fid in &tile_list {
                    update_count += 1;
                    progress.update(|| update_count);

                    let entity = soil_map.try_get_mut(fid)?;
                    // apply soil from previous iteration to elevation, and subtract the weathering amount
                    entity.elevation = (entity.elevation + entity.soil) - weathering_amount;
                    // now subtract the weathering amount
                    entity.soil = weathering_amount;
                }
    
            }


        }

        progress.finish(|| "Eroded.");

        for (fid,changed) in soil_map.into_iter().watch(progress, "Applying elevations back to tiles.", "Elevations applied.") {
            let tile = tile_map.try_get_mut(&fid)?;
            tile.elevation = changed.elevation + changed.soil;
        }

        Ok(())

        


    }
}


impl ProcessTerrainTilesWithPointIndex for SeedOcean {
    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {


        if parameters.elevations.min_elevation >= 0.0 {
            progress.announce("World is above sea level, ocean seeds will not be placed.")
        }

        let count = self.count.choose(rng);

        progress.announce(&format!("Placing {count} ocean seeds."));

        for _ in 0..count {

            let x = parameters.gen_x(rng, &self.x_filter);
            let y = parameters.gen_y(rng, &self.y_filter);
            let mut seed_id = point_index.find_nearest_tile(&(x,y).try_into()?)?;

            progress.start_unknown_endpoint(|| "Tracing seed down hill.");

            let mut seed = tile_map.try_get(&seed_id)?;
            let mut found = seed.elevation < 0.0;
            while !found {
                let mut diff = 0.0;
                let mut found_downslope = false;
                for NeighborAndDirection(neighbor_id,_) in &seed.neighbors {
                    match neighbor_id {
                        Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                            let neighbor = tile_map.try_get(neighbor_id)?;
                            if neighbor.elevation < seed.elevation {
                                let neighbor_diff = seed.elevation - neighbor.elevation;
                                if neighbor_diff > diff {
                                    found_downslope = true;
                                    diff = neighbor_diff;
                                    seed_id = neighbor_id.clone();
                                    seed = neighbor;
                                    if seed.elevation < 0.0 {
                                        found = true;
                                    }
                                }
                            }
                        }
                        Neighbor::OffMap(_) => (),
                    } // ignore off the map
                }
                if found {
                    // found one that was below sea level
                    break;
                }    
                if !found_downslope {
                    // no neighbors were found that were than the last, so we have to give up without having found one.
                    break;
                }
            }

            if !found {
                progress.finish(|| "Could not trace to below sea level.");
                // continue to attempt to place further seeds
                continue;
            } 
            
            progress.finish(|| "Seed traced.");
            
        

            tile_map.try_get_mut(&seed_id)?.grouping = Grouping::Ocean;


        }

        Ok(())
    }
}



impl ProcessTerrainTiles for FloodOcean {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Flooding ocean.");
        
        let mut queue = Vec::new();

        macro_rules! queue_neighbors {
            ($tile: ident, $queue: ident) => {
                for NeighborAndDirection(neighbor_id,_) in &$tile.neighbors {
                    match neighbor_id {
                        Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {
                            let neighbor = tile_map.try_get(&neighbor_id)?;
                            if (neighbor.elevation < 0.0) && !matches!(neighbor.grouping,Grouping::Ocean) {
                                $queue.push(neighbor_id.clone())
                            }
        
                        } // else it's off the map and unknowable
                        Neighbor::OffMap(_) => ()
                    }
                }
                
            };
        }

        for (_,tile) in tile_map.iter().watch(progress, "Finding ocean seeds.", "Ocean seeds found.") {
            if matches!(tile.grouping,Grouping::Ocean) && (tile.elevation < 0.0) {
                queue_neighbors!(tile,queue);
            }
        }

        let mut queue = queue.watch_queue(progress, "Flooding ocean.", "Ocean flooded.");

        while let Some(tile_id) = queue.pop() {
            let tile = tile_map.try_get(&tile_id)?;
            if !matches!(tile.grouping,Grouping::Ocean) {
                queue_neighbors!(tile,queue);
                tile_map.try_get_mut(&tile_id)?.grouping = Grouping::Ocean;
            } // else someone else got to this one already, so don't change it.
            
        }

        Ok(())
    }
}


impl ProcessTerrainTiles for FillOcean {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Filling ocean.");

        for (_,tile) in tile_map.iter_mut().watch(progress, "Oceanizing tiles below sea level.", "Tiles oceanized.") {
            if !matches!(tile.grouping,Grouping::Ocean) && (tile.elevation < 0.0) {
                tile.grouping = Grouping::Ocean;
            }
        }

        Ok(())
    }
}



impl ProcessTerrainTiles for ClearOcean {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Clear ocean.");

        for (_,tile) in tile_map.iter_mut().watch(progress, "Deoceanizing all tiles.", "Tiles deoceanized.") {
            if matches!(tile.grouping,Grouping::Ocean) {
                tile.grouping = Grouping::Continent;
            }
        }

        Ok(())
    }
}




impl ProcessTerrainTiles for RandomUniform {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        
        progress.announce("Generating random noise.");

        let filter = parameters.convert_height_filter(&self.height_filter);

        for (_,tile) in tile_map.iter_mut().watch(progress, "Making some noise.", "Noise made.") {

            if filter.includes(&tile.elevation) {
                let height_delta = parameters.gen_signed_height_delta(rng, &self.height_delta);
                tile.elevation += height_delta;
            }
        }

        Ok(())

    }
}


pub(crate) enum TerrainTask {
    RandomUniform(RandomUniform),
    ClearOcean(ClearOcean),
    AddHill(AddHill),
    AddRange(AddRange),
    AddStrait(AddStrait),
    Mask(Mask),
    Invert(Invert),
    Add(Add),
    Multiply(Multiply),
    Smooth(Smooth),
    Erode(Erode),
    SeedOcean(SeedOcean),
    FillOcean(FillOcean),
    FloodOcean(FloodOcean),
    SampleOceanMasked(SampleOceanMaskedLoaded),
    SampleOceanBelow(SampleOceanBelowLoaded),
    SampleElevation(SampleElevationLoaded),
}

impl TerrainTask {

    pub(crate) fn process_terrain<Random: Rng, Progress: ProgressObserver>(selves: &[Self], rng: &mut Random, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

        if !selves.is_empty() {


            progress.announce("Preparing for processes.");

            let mut properties = target.edit_properties_layer()?;
            let limits = properties.get_elevation_limits()?;
            let world_shape = properties.get_world_shape()?;
    
            let mut layer = target.edit_tile_layer()?;
            let tile_extents = layer.get_extent()?;
            let tile_count = layer.feature_count();
            let parameters = TerrainParameters::new(world_shape, limits, tile_extents.clone(), tile_count);
    
    
    
            // I only want to create the point index if any of the tasks require it. If none of them
            // require it, it's a waste of time to create it.
            let tile_map = if selves.iter().any(Self::requires_point_index) {
                // estimate the spacing between tiles:
                // * divide the area of the extents up by the number of tiles to get the average area covered by a tile.
                // * the distance across, if the tiles were square, is the square root of this area.
                let tile_spacing = ((tile_extents.height * tile_extents.width)/tile_count as f64).sqrt();
                let tile_search_radius = tile_spacing * 2.0; // multiply by two to make darn sure we cover something.
    
    
                let mut point_index = TileFinder::new(&tile_extents, parameters.world_shape.clone(), tile_count, tile_search_radius);
                let mut tile_map = layer.read_features().into_entities_index_for_each::<_,TileForTerrain,_>(|fid,tile| {
                    point_index.add_tile(tile.site.clone(), fid.clone())
                }, progress)?;
    
                for me in selves {
                    me.process_terrain_tiles_with_point_index(rng, &parameters, &point_index, &mut tile_map, progress)?;
                }
    
                tile_map    
    
            } else {
                let mut tile_map = layer.read_features().into_entities_index::<_,TileForTerrain>(progress)?;
                for me in selves {
                    me.process_terrain_tiles(rng, &parameters, &mut tile_map, progress)?;
                }
    
                tile_map
        
            };
    
        
            let mut bad_ocean_tiles_found = Vec::new();
        
            for (fid,tile) in tile_map.into_iter().watch(progress,"Writing data.","Data written.") {
    
                
                let elevation_changed = tile.elevation_changed();
                let grouping_changed = tile.grouping_changed();
                if elevation_changed || grouping_changed {
    
                    // warn user if a tile was set to ocean that's above 0.
                    if matches!(tile.grouping,Grouping::Ocean) && (tile.elevation > 0.0) {
                        bad_ocean_tiles_found.push(fid.clone());
                    }        
    
    
                    let mut feature = layer.try_feature_by_id(&fid)?;
                    if elevation_changed {
    
                        let elevation = parameters.clamp_elevation(tile.elevation);
                        let elevation_scaled = parameters.scale_elevation(elevation);
        
       
                        feature.set_elevation(&elevation)?;
                        feature.set_elevation_scaled(&elevation_scaled)?;
                    }
                    if grouping_changed {
    
            
                        // Should I check to make sure?
                        feature.set_grouping(&tile.grouping)?;
                    }
                    layer.update_feature(feature)?;
    
                }
    
            }
    
            if !bad_ocean_tiles_found.is_empty() {
                progress.warning(|| format!("At least one ocean tile was found with an elevation above 0 (id: {}).",bad_ocean_tiles_found[0]))
            }
                

        } // else there are no processes, so don't bother doing anything.


        Ok(())
    }

    fn requires_point_index(&self) -> bool {
        match self {
            Self::ClearOcean(params) => params.requires_point_index(),
            Self::RandomUniform(params) => params.requires_point_index(),
            Self::AddHill(params) => params.requires_point_index(),
            Self::AddRange(params) => params.requires_point_index(),
            Self::AddStrait(params) => params.requires_point_index(),
            Self::Mask(params) => params.requires_point_index(),
            Self::Invert(params) => params.requires_point_index(),
            Self::Add(params) => params.requires_point_index(),
            Self::Multiply(params) => params.requires_point_index(),
            Self::Smooth(params) => params.requires_point_index(),
            Self::Erode(params) => params.requires_point_index(),
            Self::SeedOcean(params) => params.requires_point_index(),
            Self::FillOcean(params) => params.requires_point_index(),
            Self::FloodOcean(params) => params.requires_point_index(),
            Self::SampleOceanMasked(params) => params.requires_point_index(),
            Self::SampleOceanBelow(params) => params.requires_point_index(),
            Self::SampleElevation(params) => params.requires_point_index(),
        }
    }

    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, limits: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        match self {
            Self::ClearOcean(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::RandomUniform(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::AddHill(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::AddRange(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::AddStrait(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Mask(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Invert(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Add(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Multiply(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Smooth(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::Erode(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::SeedOcean(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::FillOcean(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::FloodOcean(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::SampleOceanMasked(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::SampleOceanBelow(params) => params.process_terrain_tiles(rng,limits,tile_map,progress),
            Self::SampleElevation(params) => params.process_terrain_tiles(rng,limits,tile_map,progress)
        }
    }

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, limits: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        match self {
            Self::ClearOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::RandomUniform(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::AddHill(params) => params.process_terrain_tiles_with_point_index(rng, limits, point_index, tile_map, progress),
            Self::AddRange(params) => params.process_terrain_tiles_with_point_index(rng, limits, point_index, tile_map, progress),
            Self::AddStrait(params) => params.process_terrain_tiles_with_point_index(rng, limits, point_index, tile_map, progress),
            Self::Mask(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::Invert(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::Add(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::Multiply(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::Smooth(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::Erode(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SeedOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::FillOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::FloodOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleOceanMasked(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleOceanBelow(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleElevation(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress)
        }
    }


}

