use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use std::fmt::Display;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::collections::HashSet;

use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use rand::Rng;
use rand_distr::uniform::SampleUniform;

use crate::errors::CommandError;
use crate::world_map::EntityIndex;
use crate::world_map::TileSchema;
use crate::world_map::TileForTerrain;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::raster::RasterMap;
use crate::world_map::Grouping;
use crate::subcommand_def;
use crate::world_map::ElevationLimits;
use crate::utils::RandomNth;
use crate::utils::point_finder::TileFinder;
use crate::utils::Point;
use crate::utils::Extent;
use crate::progress::WatchableDeque;
use crate::progress::WatchableQueue;


// TODO: I think that in order to guarantee reproducibility on random numbers, I'm going to have to be able to sort the tiles before generating. And in order to do that consistently, I might need to add a 'gen_order' field to the tiles, incremented when adding tiles. That has to go all the way back to points. This will help with reproducibility on the other stuff as well. I would also need to move into a sorted HashMap of some sort in order to make sure the iterator comes out correctly.

trait TruncOrSelf {

    fn trunc_or_self(self) -> Self;
}

impl TruncOrSelf for f64 {
    fn trunc_or_self(self) -> Self {
        self.trunc()
    }
}

impl TruncOrSelf for usize {

    fn trunc_or_self(self) -> Self {
        self
    }

}

impl TruncOrSelf for i8 {
    fn trunc_or_self(self) -> Self {
        self
    }
}

#[derive(Clone)]
enum ArgRange<NumberType> {
    // While I could use a real Range<> and RangeInclusive<>, I'd have to copy it every time I want to generate a number from it anyway, and
    Inclusive(NumberType,NumberType),
    Exclusive(NumberType,NumberType),
    Single(NumberType)
}

impl<NumberType: SampleUniform + PartialOrd + Copy + TruncOrSelf> ArgRange<NumberType> {

    fn choose<Random: Rng>(&self, rng: &mut Random) -> NumberType {
        match self  {
            ArgRange::Inclusive(min,max) => rng.gen_range(*min..=*max),
            ArgRange::Exclusive(min,max) => rng.gen_range(*min..*max),
            ArgRange::Single(value) => *value,
        }
    }

    fn includes(&self, value: &NumberType) -> bool {
        match self {
            ArgRange::Inclusive(min, max) => (value >= min) && (value <= max),
            ArgRange::Exclusive(min, max) => (value >= min) && (value < max),
            ArgRange::Single(value) => value.trunc_or_self() == value.trunc_or_self(),
        }
    }
}



impl<'deserializer,NumberType: FromStr + PartialOrd + Deserialize<'deserializer>> Deserialize<'deserializer> for ArgRange<NumberType> {

    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'deserializer> {

        // https://stackoverflow.com/q/56582722/300213
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StrOrNum<NumberType> {
            Str(String),
            Num(NumberType)
        }

        let value = StrOrNum::deserialize(deserializer)?;
        match value {
            StrOrNum::Str(deserialized) => deserialized.parse().map_err(|e: CommandError| serde::de::Error::custom(e.to_string())),
            StrOrNum::Num(deserialized) => Ok(ArgRange::Single(deserialized)),
        }
        
    }
}

impl<NumberType: FromStr + Display> Serialize for ArgRange<NumberType> {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<NumberType: FromStr + PartialOrd> FromStr for ArgRange<NumberType> {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((first,mut last)) = s.split_once("..") {
            let include_last = if last.starts_with('=') {
                last = last.trim_start_matches('=');
                true
            } else {
                false
            };

            let first = first.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            let last = last.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            if first > last {
                Err(CommandError::InvalidRangeArgument(s.to_owned()))?
            }

            Ok(if include_last {
                ArgRange::Inclusive(first,last)
            } else {
                ArgRange::Exclusive(first,last)
            })
        } else {
            let number = s.parse().map_err(|_| CommandError::InvalidRangeArgument(s.to_owned()))?;
            Ok(ArgRange::Single(number))
        }
    }
}

impl<NumberType: FromStr + Display> Display for ArgRange<NumberType> {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgRange::Inclusive(min,max) => write!(f,"{}..={}",min,max),
            ArgRange::Exclusive(min,max) => write!(f,"{}..{}",min,max),
            ArgRange::Single(single) => write!(f,"{}",single),
        }
    }
}

enum RelativeHeightTruncation {
    Floor,
    Ceil,
    UnTruncated,
}

struct TerrainParameters {
    elevations: ElevationLimits,
    positive_elevation_scale: f64,
    negative_elevation_scale: f64,
    expanse_above_sea_level: f64,
    blob_power: f64,
    line_power: f64, 
    extents: Extent
}

impl TerrainParameters {

    fn get_blob_power(tile_count: usize) -> f64 {
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

    fn get_line_power(tile_count: usize) -> f64 {
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
        

    fn new(elevations: ElevationLimits, extents: Extent, tile_count: usize) -> Self {
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
            expanse_above_sea_level,
            positive_elevation_scale,
            negative_elevation_scale,
            blob_power,
            line_power,
            extents,
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

    fn get_height_delta(&self, height_delta: &i8) -> (f64,f64) {
        // convert the delta relative to the above sea level range, rather than below, so the
        // input to convert needs to be positive.
        let (height_delta,sign) = if height_delta.is_negative() {
            (height_delta.abs(),-1.0)
        } else {
            (*height_delta,1.0)
        };
        let result = self.convert_relative_height(&height_delta, RelativeHeightTruncation::UnTruncated,false);
        (result,sign)

    }

    fn get_signed_height_delta(&self, height_delta: &i8) -> f64 {
        let (value,sign) = self.get_height_delta(height_delta);
        value.copysign(sign)
    }

    fn gen_height_delta<Random: Rng>(&self, rng: &mut Random, height_delta: &ArgRange<i8>) -> (f64,f64) {
        let chosen = height_delta.choose(rng);
        self.get_height_delta(&chosen)
    }

    fn gen_signed_height_delta<Random: Rng>(&self, rng: &mut Random, height_delta: &ArgRange<i8>) -> f64 {
        let (value,sign) = self.gen_height_delta(rng, height_delta);
        value.copysign(sign)
    }



    fn convert_relative_height(&self, value: &i8, direction: RelativeHeightTruncation, clamp: bool) -> f64 {
        let max_elevation = self.elevations.max_elevation;
        let min_elevation = self.elevations.min_elevation;
        let result = if value == &100 {
            max_elevation
        } else if value == &-100 {
            min_elevation
        } else {
            let fraction = match direction {
                RelativeHeightTruncation::Floor => (*value as f64/100.0).floor(),
                RelativeHeightTruncation::Ceil => (*value as f64/100.0).ceil(),
                RelativeHeightTruncation::UnTruncated => *value as f64/100.0,
            };
            if value >= &0 {
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
                self.convert_relative_height(min, RelativeHeightTruncation::Floor,true), 
                self.convert_relative_height(max, RelativeHeightTruncation::Ceil,true)
            ),
            Some(ArgRange::Exclusive(min, max)) => ArgRange::Exclusive(
                self.convert_relative_height(min, RelativeHeightTruncation::Floor,true), 
                self.convert_relative_height(max, RelativeHeightTruncation::Ceil,true)
            ),
            Some(ArgRange::Single(single)) => ArgRange::Inclusive(
                self.convert_relative_height(single, RelativeHeightTruncation::Floor,true), 
                self.convert_relative_height(single, RelativeHeightTruncation::Ceil,true)
            ),
            None => ArgRange::Inclusive(self.elevations.min_elevation, self.elevations.max_elevation)
        }
    }

    fn is_elevation_within(&self, h: f64, limit_fraction: f64) -> bool {
        h <= (self.elevations.max_elevation * limit_fraction) &&
        if self.elevations.min_elevation < 0.0 {
            h >= (self.elevations.min_elevation * limit_fraction)
        } else {
            h >= (self.elevations.max_elevation - (self.expanse_above_sea_level * limit_fraction))
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
        rng.gen_range(0.0..(self.extents.height * 0.7)) + self.extents.height * 0.15 + self.extents.south
    }
    
    fn gen_end_x<Random: Rng>(&self, rng: &mut Random) -> f64 {
        rng.gen_range(0.0..(self.extents.width * 0.8)) + self.extents.width * 0.1 + self.extents.west
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
        // I've added this function to make it easier to change requirements later.
        unimplemented!("This process requires a point index.")
    }


    fn requires_point_index(&self) -> bool {
        true
    }
}

trait LoadTerrainProcess {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError>;
}


subcommand_def!{

    /// Processes a series of pre-saved tasks
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Recipe {

        /// Raster file defining new elevations
        source: PathBuf
    }
}

impl LoadTerrainProcess for Recipe {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe tasks.");
        let recipe_data = File::open(&self.source).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        let reader = BufReader::new(recipe_data);
        let tasks: Vec<TerrainProcessCommand> = serde_json::from_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        progress.finish(|| "Recipe tasks loaded.");
        let mut result = Vec::new();
        for task in tasks {
            result.extend(task.load_terrain_processes(random,progress)?)
        }
        Ok(result)
    }

}

subcommand_def!{

    /// Randomly chooses a recipe from a set of named recipes and follows it
    #[derive(Deserialize,Serialize)]
    pub(crate) struct RecipeSet {

        /// Raster file defining new elevations
        source: PathBuf,

        #[arg(long)]
        recipe: Option<String>
    }


}

impl LoadTerrainProcess for RecipeSet {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe set.");
        let recipe_data = File::open(&self.source).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        let reader = BufReader::new(recipe_data);
        let mut tasks: HashMap<String,Vec<TerrainProcessCommand>> = serde_json::from_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        progress.finish(|| "Recipe set loaded.");
        if tasks.len() > 0 {
            let chosen_key = if let Some(recipe) = self.recipe {
                recipe
            } else {
                tasks.keys().choose(random).unwrap().to_owned() // there should be at least one here so this should never happen.
            };
            if let Some(tasks) = tasks.remove(&chosen_key) {
                let mut result = Vec::new();
                for task in tasks {
                    result.extend(task.load_terrain_processes(random,progress)?)
                }
                Ok(result)
            } else {
                Err(CommandError::RecipeFileRead(format!("Can't find recipe '{}' in set.",chosen_key)))
            }
    
        } else {
            Err(CommandError::RecipeFileRead("Recipe set is empty.".to_owned()))
        }
    }

}

subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean.
    #[derive(Deserialize,Serialize)]
    pub(crate) struct SampleOceanBelow {

        /// The raster to sample from
        source: PathBuf,

        /// The elevation to compare to
        #[arg(allow_negative_numbers=true)]
        elevation: f64
    }
}

pub(crate) struct SampleOceanBelowLoaded {
    raster: RasterMap,
    elevation: f64
}


impl LoadTerrainProcess for SampleOceanBelow {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainProcess::SampleOceanBelow(SampleOceanBelowLoaded {
            raster,
            elevation: self.elevation
        })])
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
    
            let (x,y) = bounds.coords_to_pixels(tile.site.x.into_inner(), tile.site.y.into_inner());

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                let is_no_data = match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => elevation.is_nan(),
                    Some(no_data_value) => elevation == no_data_value,
                    None => false,
                };

                if !is_no_data {
                    elevation < &self.elevation
                } else {
                    false
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

subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean.
    #[derive(Deserialize,Serialize)]
    pub(crate) struct SampleOceanMasked {

        /// The raster to read ocean data from
        source: PathBuf
    }
}


pub(crate) struct SampleOceanMaskedLoaded {
    raster: RasterMap
}


impl LoadTerrainProcess for SampleOceanMasked {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainProcess::SampleOceanMasked(SampleOceanMaskedLoaded {
            raster
        })])
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
    
            let (x,y) = bounds.coords_to_pixels(tile.site.x.into_inner(), tile.site.y.into_inner());

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => !elevation.is_nan(),
                    Some(no_data_value) => elevation != no_data_value,
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
    pub(crate) fn new(raster: RasterMap) -> TerrainProcess {
        TerrainProcess::SampleElevation(Self {
            raster
        })
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
    
    
            let (x,y) = bounds.coords_to_pixels(tile.site.x.into_inner(), tile.site.y.into_inner());
    
            if let Some(elevation) = band.get_value(x, y) {

                tile.elevation = *elevation;
    
            }
    
    
        }

        Ok(())
    }
}

subcommand_def!{

    /// Replaces elevations by sampling from a heightmap
    #[derive(Deserialize,Serialize)]
    pub(crate) struct SampleElevation {

        /// Raster file defining new elevations
        source: PathBuf
    }
}

impl LoadTerrainProcess for SampleElevation {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading elevation raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Elevation raster loaded.");
        Ok(vec![TerrainProcess::SampleElevation(SampleElevationLoaded {
            raster
        })])
    }
}

subcommand_def!{

    /// Adds hills or pits to a certain area of the map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct AddHill {

        count: ArgRange<usize>,

        height_delta: ArgRange<i8>,

        x_filter: ArgRange<f64>,

        y_filter: ArgRange<f64>

    }
}

impl LoadTerrainProcess for AddHill {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::AddHill(self)])
    }
}

impl ProcessTerrainTilesWithPointIndex for AddHill {

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        
        let count = self.count.choose(rng);

        progress.announce(&format!("Generating {} hills.",count));


        for i in 0..count {
            let mut change_map = HashMap::new();
            let (height_delta,sign) = parameters.gen_height_delta(rng, &self.height_delta);

            let mut start;
            let mut limit = 0;
            loop {
                let x = parameters.gen_x(rng, &self.x_filter);
                let y = parameters.gen_y(rng, &self.y_filter);
                start = point_index.find_nearest_tile(&Point::from_f64(x,y)?)?;
                let start_tile = tile_map.try_get(&start)?;

                if (limit >= 50) || parameters.is_elevation_within(start_tile.elevation + height_delta.copysign(sign),0.9) {
                    break;
                }
                limit += 1;
            }

            change_map.insert(start,height_delta);
            let mut queue = VecDeque::from([start]).watch_queue(progress,format!("Generating hill #{}.",i+1),format!("Hill #{} generated.",i+1));

            while let Some(tile_id) = queue.pop_front() {
                let tile = tile_map.try_get(&tile_id)?;
                let last_change = *change_map.get(&tile_id).unwrap(); // shouldn't be any reason why this is not found.
                for (neighbor_id,_) in &tile.neighbors {
                    if change_map.contains_key(&neighbor_id) {
                        continue;
                    }

                    let neighbor_height_delta = last_change.powf(parameters.blob_power) * (rng.gen_range(0.0..0.2) + 0.9);
                    change_map.insert(*neighbor_id, neighbor_height_delta);
                    if neighbor_height_delta > 1.0 { 
                        queue.push_back(*neighbor_id)
                    }

                }
            }

            for (tile_id,height_delta) in change_map {
                tile_map.try_get_mut(&tile_id)?.elevation += height_delta.copysign(sign);
            }

        }

        Ok(())

    }
}

subcommand_def!{

    /// Adds a range of heights or a trough to a certain area of a map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct AddRange {
        count: ArgRange<usize>,
        height_delta: ArgRange<i8>,
        x_filter: ArgRange<f64>,
        y_filter: ArgRange<f64>
    }
}

impl LoadTerrainProcess for AddRange {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::AddRange(self)])
    }
}

impl ProcessTerrainTilesWithPointIndex for AddRange {

    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        // FUTURE: I'm not getting the same results with the same seeds. It might be different with a genesis command that runs the recipe after creating
        // the blank.

        let count = self.count.choose(rng);

        progress.announce(&format!("Generating {} ranges.",count));

        let lower_dist_limit = parameters.extents.width / 8.0;
        let upper_dist_limit = parameters.extents.width / 3.0;

        for i in 0..count {
            // add one range

            let mut used = HashSet::new();
            let (mut height_delta,sign) = parameters.gen_height_delta(rng, &self.height_delta);

            // find start and end points
            let start_x = parameters.gen_x(rng, &self.x_filter);
            let start_y = parameters.gen_y(rng, &self.y_filter);
            let start_point = Point::from_f64(start_x, start_y)?;
            let mut end_point;

            // find an end point that's far enough away
            let mut limit = 0;
            loop {
                let end_x = parameters.gen_end_x(rng);
                let end_y = parameters.gen_end_y(rng);
                end_point = Point::from_f64(end_x, end_y)?;
                let dist = start_point.distance(&end_point);
                if (limit >= 50) || (dist >= lower_dist_limit) && (dist <= upper_dist_limit) {
                    break;
                }
                limit += 1;

            }

            let start = point_index.find_nearest_tile(&start_point)?;
            let end = point_index.find_nearest_tile(&end_point)?;

            progress.start_unknown_endpoint(|| format!("Generating range #{}.",i+1));
            let range = get_range(rng, tile_map, &mut used, start, end, 0.85)?;

            // add height to ridge and neighboring cells
            let mut queue = range.clone();
            let mut spread_count = 0;
            // TODO: How do we watch this queue for progress
            // TODO: Instead of processing in batches, pass the next height_delta into the queue. Then, calculate the next height_delta
            // before queueing in neighbors. This will calculate different height_deltas for each set of neighbors, which might
            // even create some rougher ranges?
            while queue.len() > 0 {
                let frontier = std::mem::replace(&mut queue, Vec::new());
                spread_count += 1;
                for tile_id in frontier {
                    tile_map.try_get_mut(&tile_id)?.elevation += (height_delta * (rng.gen_range(0.0..0.3) + 0.85)).copysign(sign);
                    for (neighbor_id,_) in &tile_map.try_get(&tile_id)?.neighbors {
                        if !used.contains(neighbor_id) {
                            queue.push(*neighbor_id);
                            used.insert(*neighbor_id);
                        }

                    }
                }
                height_delta = height_delta.powf(parameters.line_power) - 1.0;
                if height_delta < 2.0 { // TODO: This limit was based on scaled elevation originally. It needs to be higher?
                    break;
                }

            }

            // create some prominences in the range.
            for (i,mut current_id) in range.into_iter().enumerate() {
                if (i % 6) != 0 {
                    continue;
                }
                for _ in 0..spread_count {
                    let current = tile_map.try_get(&current_id)?;
                    let current_elevation = current.elevation;
                    let mut min_elevation = None;
                    for (neighbor_id,_) in &current.neighbors {
                        let neighbor = tile_map.try_get(&neighbor_id)?;
                        let elevation = neighbor.elevation;
                        match min_elevation {
                            None => min_elevation = Some((*neighbor_id,elevation)),
                            Some((_,prev_elevation)) => if elevation < prev_elevation {
                                min_elevation = Some((*neighbor_id,elevation))
                            }
                        }
                    }
                    if let Some((min_tile_id,elevation)) = min_elevation {
                        tile_map.try_get_mut(&min_tile_id)?.elevation = ((current_elevation * 2.0) + elevation.copysign(sign)) / 3.0;
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

fn get_range<Random: Rng>(rng: &mut Random, tile_map: &mut EntityIndex<TileSchema, TileForTerrain>, used: &mut HashSet<u64>, start: u64, end: u64, jagged_probability: f64) -> Result<Vec<u64>, CommandError> {
    let mut cur_id = start;
    let end_tile = tile_map.try_get(&end)?;
    let mut range = vec![cur_id];
    used.insert(cur_id);
    while cur_id != end {
        let mut min = f64::INFINITY;
        let cur_tile = tile_map.try_get(&cur_id)?;
        // basically, find the neighbor that is closest to the end
        for (neighbor_id,_) in &cur_tile.neighbors {
            if used.contains(&neighbor_id) {
                continue;
            }

            let neighbor_tile = tile_map.try_get(&neighbor_id)?;
            let diff = end_tile.site.distance(&neighbor_tile.site);
            let diff = if rng.gen_bool(jagged_probability) {
                // every once in a while, make the neighbor seem closer, to create more jagged ridges.
                diff / 2.0
            } else {
                diff
            };
            if diff < min {
                min = diff;
                cur_id = *neighbor_id;
            }
        }
        if min.is_infinite() { // no neighbors at all were found?
            break;
        }
        range.push(cur_id);
        used.insert(cur_id);
    }
    Ok(range)

}


#[derive(Clone,Deserialize,Serialize,ValueEnum)]
enum Direction {
    Horizontal,
    Vertical
}

subcommand_def!{

    /// Adds a long cut somewhere on the map
    // TODO: Why isn't there an equivalent "isthmus" of some sort? Should I specify the height change? Why are the directions limited to horizontal and vertical? And shouldn't the direction at least be an axis instead of vert/horiz, would be a z-axis?
    #[derive(Deserialize,Serialize)]
    pub(crate) struct AddStrait { 
        width: ArgRange<f64>,
        direction: Direction
    }

}

impl LoadTerrainProcess for AddStrait {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::AddStrait(self)])
    }

}

impl ProcessTerrainTilesWithPointIndex for AddStrait {
    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {


        // TODO: I feel like this isn't as nice as the others:
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
            Direction::Vertical => {
                let start_x = rng.gen_range(0.0..(e_width * 0.4)) + (e_width * 0.3);
                let start_y = 5.0;
                let end_x = e_width - start_x - (e_width * 0.1) + rng.gen_range(0.0..(e_width * 0.2));
                let end_y = e_height - 5.0;
                (start_x,start_y,end_x,end_y)
            },
            Direction::Horizontal => {
                let start_x = 5.0;
                let start_y = rng.gen_range(0.0..(e_height * 0.4)) + (e_height * 0.3);
                let end_x = e_width - 5.0;
                let end_y = e_height - start_y - (e_height * 0.1) + rng.gen_range(0.0..(e_height * 0.2));
                (start_x,start_y,end_x,end_y)
            },
        };
        let start_point = Point::from_f64(start_x + e_west, start_y + e_south)?;
        let end_point = Point::from_f64(end_x + e_west, end_y + e_south)?;

        let start = point_index.find_nearest_tile(&start_point)?;
        let end = point_index.find_nearest_tile(&end_point)?;

        let mut range = get_range(rng, tile_map, &mut used, start, end, 0.8)?;

        let mut next_queue = Vec::new();

        let step = 0.1/width;

        let progress_width = width.ceil() as usize;
        progress.start_known_endpoint(|| ("Generating strait.",progress_width));

        // TODO: Just like add_range, if I pass the exp along with the item in the queue, then I could do this in a real
        // queue.
        while width > 0.0 {
            // TODO: The changes on this aren't right, I feel like they are way too deep, probably because they were created with elevation_scale in mind.
            let exp = 0.99 - (step * width);
            for tile_id in &range {
                let tile = tile_map.try_get(&tile_id)?;
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
                    new_elevation = (parameters.expanse_above_sea_level * 0.5) + parameters.elevations.min_elevation;
                }

                for (neighbor_id,_) in &tile.neighbors {
                    if used.contains(&neighbor_id) {
                        continue;
                    }
                    used.insert(*neighbor_id);
                    next_queue.push(*neighbor_id);
                }

                tile_map.try_get_mut(&tile_id)?.elevation = new_elevation;
            }
            range = std::mem::replace(&mut next_queue, Vec::new());

            width -= 1.0;
            progress.update(|| progress_width - (width.ceil() as usize));
        }
        progress.finish(|| "Strait generated.");

        Ok(())
    }
}

subcommand_def!{

    /// Changes the heights based on their distance from the edge of the map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Mask {
        #[arg(default_value="1")]
        power: f64
    }
}

impl LoadTerrainProcess for Mask {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Mask(self)])
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
            let mut distance = (1.0 - nx.powi(2)) * (1.0 - ny.powi(2)); // 0<--:1:-->0
            if self.power.is_sign_negative() {
                distance = 1.0 - distance; // inverted, // 1<--:0:-->1
            }
            let masked = tile.elevation * distance;
            let new_elevation = ((tile.elevation * (factor - 1.0)) + masked)/factor;

            tile.elevation = new_elevation;

        }

        Ok(())

    }
}

#[derive(Clone,Deserialize,Serialize,ValueEnum)]
enum InvertAxes {
    X,
    Y,
    Both
}

subcommand_def!{

    /// Inverts the heights across the entire map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Invert {
        probability: f64, 
        axes: InvertAxes
    }
    
}


impl LoadTerrainProcess for Invert {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Invert(self)])
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

            let switch_point = Point::from_f64(switch_x, switch_y)?;

            // cache the calculation
            let switch_tile_id = match switches.get(fid) {
                None => {
                    // NOTE: This is where the most time is spent. Removing this and setting switch_tile_id to a constant value 
                    // sped up things about 90%. Of course, it also broke the algorithm.
                    let switch_tile_id = point_index.find_nearest_tile(&switch_point)?;
                    switches.insert(switch_tile_id, *fid);     
                    switch_tile_id               
                },
                Some(id) => *id,
            };


            let switch_tile = tile_map.try_get(&switch_tile_id)?;

            // removing this command did not produce significant speed improvements for this part of the progress,
            // so this isn't adding to the time. (And would have broken the algorithm)
            inverted_heights.push((*fid, switch_tile.elevation));

        }

        for (fid,elevation) in inverted_heights.into_iter().watch(progress, "Writing inversions.", "Inversions written.") {
            tile_map.try_get_mut(&fid)?.elevation = elevation;
        }

        Ok(())


    }

}


subcommand_def!{

    /// Inverts the heights across the entier map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Add {
        height_filter: Option<ArgRange<i8>>, 
        height_delta: i8
    }
    
}


impl LoadTerrainProcess for Add {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Add(self)])
    }


}

impl ProcessTerrainTiles for Add {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce(&format!("Adding {} to some elevations.",self.height_delta));

        let filter = parameters.convert_height_filter(&self.height_filter);
        let height_delta = parameters.get_signed_height_delta(&self.height_delta);

        for (_,tile) in tile_map.iter_mut().watch(progress, format!("Adding heights."), "Heights added.") {

            if filter.includes(&tile.elevation) {
                tile.elevation += height_delta;
            }
        }

        Ok(())

    }
}


subcommand_def!{

    /// Inverts the heights across the entier map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Multiply {
        height_filter: Option<ArgRange<i8>>, 
        height_factor: f64 // this doesn't have to be i8 because it's a multiplication, will still work no matter what the scale.
    }
    
}


impl LoadTerrainProcess for Multiply {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Multiply(self)])
    }


}

impl ProcessTerrainTiles for Multiply {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        progress.announce(&format!("Multiplying some elevations by {}.",self.height_factor));

        let filter = parameters.convert_height_filter(&self.height_filter);

        for (_,tile) in tile_map.iter_mut().watch(progress, format!("Multiplying heights."), "Heights multiplied.") {

            if filter.includes(&tile.elevation) {
                tile.elevation *= self.height_factor;
            }
        }

        Ok(())

    }
}


subcommand_def!{

    /// Smooths elevations by averaging the value against it's neighbors.
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Smooth {
        #[arg(default_value="2")]
        fr: f64 // TODO: I'm not sure what this actually is. It's not quite a weighted average, I don't really understand where AFMG got its algorithm from.
    }
    
}


impl LoadTerrainProcess for Smooth {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Smooth(self)])
    }


}

impl ProcessTerrainTiles for Smooth {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Smoothing heights.");

        // I need to know the heights of different tiles, so I can't update heights inline.
        let mut changed_heights = Vec::new();

        for (fid,tile) in tile_map.iter().watch(progress, "Finding averages.", "Averages found.") {
            let mut heights = vec![tile.elevation];
            for (neighbor_id,_) in &tile.neighbors {
                let neighbor = tile_map.try_get(&neighbor_id)?;
                heights.push(neighbor.elevation);
            }
            let average = heights.iter().sum::<f64>()/heights.len() as f64;
            let new_height = if self.fr == 1.0 {
                average
            } else {
                parameters.clamp_elevation((tile.elevation * (self.fr - 1.0) + average) / self.fr)
            };
            changed_heights.push((*fid,new_height));
        }

        for (fid,elevation) in changed_heights.into_iter().watch(progress, "Writing heights.", "Heights written.") {
            tile_map.try_get_mut(&fid)?.elevation = elevation;
        }

        Ok(())

    }
}


subcommand_def!{

    /// Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
    #[derive(Deserialize,Serialize)]
    pub(crate) struct SeedOcean {
        count: ArgRange<usize>,

        x_filter: ArgRange<f64>,

        y_filter: ArgRange<f64>
    }
    
}


impl LoadTerrainProcess for SeedOcean {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::SeedOcean(self)])
    }


}

impl ProcessTerrainTilesWithPointIndex for SeedOcean {
    fn process_terrain_tiles_with_point_index<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, point_index: &TileFinder, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {


        if parameters.elevations.min_elevation >= 0.0 {
            progress.announce("World is above sea level, ocean seeds will not be placed.")
        }

        let count = self.count.choose(rng);

        progress.announce(&format!("Placing {} ocean seeds.",count));

        for _ in 0..count {

            let x = parameters.gen_x(rng, &self.x_filter);
            let y = parameters.gen_y(rng, &self.y_filter);
            let mut seed_id = point_index.find_nearest_tile(&Point::from_f64(x,y)?)?;

            progress.start_unknown_endpoint(|| "Tracing seed down hill.");

            let mut seed = tile_map.try_get(&seed_id)?;
            let mut found = seed.elevation < 0.0;
            while !found {
                let mut diff = 0.0;
                let mut found_downslope = false;
                for (neighbor_id,_) in &seed.neighbors {
                    let neighbor = tile_map.try_get(neighbor_id)?;
                    if neighbor.elevation < seed.elevation {
                        let neighbor_diff = seed.elevation - neighbor.elevation;
                        if neighbor_diff > diff {
                            found_downslope = true;
                            diff = neighbor_diff;
                            seed_id = *neighbor_id;
                            seed = neighbor;
                            if seed.elevation < 0.0 {
                                found = true;
                            }
                        }
                    }
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
            } else {
                progress.finish(|| "Seed traced.");
            }
        

            tile_map.try_get_mut(&seed_id)?.grouping = Grouping::Ocean;


        }

        Ok(())
    }
}


subcommand_def!{

    /// Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found.
    #[derive(Deserialize,Serialize)]
    pub(crate) struct FloodOcean{}
    
}


impl LoadTerrainProcess for FloodOcean {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::FloodOcean(self)])
    }


}

impl ProcessTerrainTiles for FloodOcean {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, _: &mut Random, _: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Flooding ocean.");
        
        let mut queue = Vec::new();

        macro_rules! queue_neighbors {
            ($tile: ident, $queue: ident) => {
                for (neighbor_id,_) in &$tile.neighbors {
                    let neighbor = tile_map.try_get(&neighbor_id)?;
                    if (neighbor.elevation < 0.0) && !matches!(neighbor.grouping,Grouping::Ocean) {
                        $queue.push(*neighbor_id)
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


subcommand_def!{

    /// Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
    #[derive(Deserialize,Serialize)]
    pub(crate) struct FillOcean{}
    
}


impl LoadTerrainProcess for FillOcean {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::FillOcean(self)])
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


subcommand_def!{

    /// Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
    #[derive(Deserialize,Serialize)]
    pub(crate) struct ClearOcean{}
    
}


impl LoadTerrainProcess for ClearOcean {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::ClearOcean(self)])
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


subcommand_def!{

    /// Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0.
    #[derive(Deserialize,Serialize)]
    pub(crate) struct Clear{}
    
}

impl LoadTerrainProcess for Clear {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::Multiply(Multiply { 
            height_filter: None, 
            height_factor: 0.0
        })])
    }


}


subcommand_def!{

    /// Adds a uniform amount of random noise to the map
    #[derive(Deserialize,Serialize)]
    pub(crate) struct RandomUniform{

        height_filter: Option<ArgRange<i8>>, 
        height_delta: ArgRange<i8>
    }
    
}


impl LoadTerrainProcess for RandomUniform {

    fn load_terrain_process<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {
        Ok(vec![TerrainProcess::RandomUniform(self)])
    }


}

impl ProcessTerrainTiles for RandomUniform {
    fn process_terrain_tiles<Random: Rng, Progress: ProgressObserver>(&self, rng: &mut Random, parameters: &TerrainParameters, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        
        progress.announce("Generating random noise.");

        let filter = parameters.convert_height_filter(&self.height_filter);

        for (_,tile) in tile_map.iter_mut().watch(progress, format!("Making some noise."), "Noise made.") {

            if filter.includes(&tile.elevation) {
                let height_delta = parameters.gen_signed_height_delta(rng, &self.height_delta);
                tile.elevation += height_delta;
            }
        }

        Ok(())

    }
}

#[derive(Deserialize,Serialize,Subcommand)]
pub(crate) enum TerrainProcessCommand {
    Recipe(Recipe),
    RecipeSet(RecipeSet),
    Clear(Clear),
    ClearOcean(ClearOcean),
    RandomUniform(RandomUniform),
    AddHill(AddHill),
    AddRange(AddRange),
    AddStrait(AddStrait),
    Mask(Mask),
    Invert(Invert),
    Add(Add),
    Multiply(Multiply),
    Smooth(Smooth),
    SeedOcean(SeedOcean),
    FillOcean(FillOcean),
    FloodOcean(FloodOcean),
    SampleOceanMasked(SampleOceanMasked),
    SampleOceanBelow(SampleOceanBelow),
    SampleElevation(SampleElevation),
}

impl TerrainProcessCommand {

    pub(crate) fn to_json(&self) -> Result<String,CommandError> {
        // NOTE: Not technically a recipe read error, but this shouldn't be used very often.
        Ok(serde_json::to_string_pretty(self).map_err(|e| CommandError::TerrainProcessWrite(format!("{}",e)))?)
    }

    pub(crate) fn load_terrain_processes<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainProcess>,CommandError> {

        match self {
            TerrainProcessCommand::Clear(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::ClearOcean(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::RandomUniform(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Recipe(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::RecipeSet(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::AddHill(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::AddRange(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::AddStrait(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Mask(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Invert(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Add(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Multiply(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::Smooth(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::SeedOcean(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::FillOcean(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::FloodOcean(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::SampleOceanMasked(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::SampleOceanBelow(params) => params.load_terrain_process(random,progress),
            TerrainProcessCommand::SampleElevation(params) => params.load_terrain_process(random,progress),
        }
    }

}


pub(crate) enum TerrainProcess {
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
    SeedOcean(SeedOcean),
    FillOcean(FillOcean),
    FloodOcean(FloodOcean),
    SampleOceanMasked(SampleOceanMaskedLoaded),
    SampleOceanBelow(SampleOceanBelowLoaded),
    SampleElevation(SampleElevationLoaded),
}

impl TerrainProcess {

    pub(crate) fn process_terrain<Random: Rng, Progress: ProgressObserver>(selves: &[Self], rng: &mut Random, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Preparing for processes.");

        let limits = target.edit_properties_layer()?.get_elevation_limits()?;

        let mut layer = target.edit_tile_layer()?;
        let tile_extents = layer.get_extent()?;
        let tile_count = layer.feature_count();
        let parameters = TerrainParameters::new(limits, tile_extents.clone(), tile_count);



        // I only want to create the point index if any of the tasks require it. If none of them
        // require it, it's a waste of time to create it.
        let tile_map = if selves.iter().any(|s| s.requires_point_index()) {
            // estimate the spacing between tiles:
            // * divide the area of the extents up by the number of tiles to get the average area covered by a tile.
            // * the distance across, if the tiles were square, is the square root of this area.
            let tile_spacing = ((tile_extents.height * tile_extents.width)/tile_count as f64).sqrt();
            let tile_search_radius = tile_spacing * 2.0; // multiply by two to make darn sure we cover something.


            let mut point_index = TileFinder::new(&tile_extents, tile_count, tile_search_radius);
            let mut tile_map = layer.read_features().to_entities_index_for_each::<_,TileForTerrain,_>(|fid,tile| {
                point_index.add_tile(tile.site.clone(), *fid)
            }, progress)?;

            for me in selves {
                me.process_terrain_tiles_with_point_index(rng, &parameters, &point_index, &mut tile_map, progress)?;
            }

            tile_map    

        } else {
            let mut tile_map = layer.read_features().to_entities_index::<_,TileForTerrain>(progress)?;
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
                    bad_ocean_tiles_found.push(fid);
                }        


                let mut feature = layer.try_feature_by_id(&fid)?;
                if elevation_changed {

                    let elevation = parameters.clamp_elevation(tile.elevation);
                    let elevation_scaled = parameters.scale_elevation(elevation);
    
   
                    feature.set_elevation(elevation)?;
                    feature.set_elevation_scaled(elevation_scaled)?;
                }
                if grouping_changed {

        
                    // Should I check to make sure?
                    feature.set_grouping(&tile.grouping)?;
                }
                layer.update_feature(feature)?;

            }

        }

        if bad_ocean_tiles_found.len() > 0 {
            progress.warning(|| format!("At least one ocean tile was found with an elevation above 0 (id: {}).",bad_ocean_tiles_found[0]))
        }



        Ok(())
    }

    fn requires_point_index(&self) -> bool {
        match self {
            TerrainProcess::ClearOcean(params) => params.requires_point_index(),
            TerrainProcess::RandomUniform(params) => params.requires_point_index(),
            TerrainProcess::AddHill(params) => params.requires_point_index(),
            TerrainProcess::AddRange(params) => params.requires_point_index(),
            TerrainProcess::AddStrait(params) => params.requires_point_index(),
            TerrainProcess::Mask(params) => params.requires_point_index(),
            TerrainProcess::Invert(params) => params.requires_point_index(),
            TerrainProcess::Add(params) => params.requires_point_index(),
            TerrainProcess::Multiply(params) => params.requires_point_index(),
            TerrainProcess::Smooth(params) => params.requires_point_index(),
            TerrainProcess::SeedOcean(params) => params.requires_point_index(),
            TerrainProcess::FillOcean(params) => params.requires_point_index(),
            TerrainProcess::FloodOcean(params) => params.requires_point_index(),
            TerrainProcess::SampleOceanMasked(params) => params.requires_point_index(),
            TerrainProcess::SampleOceanBelow(params) => params.requires_point_index(),
            TerrainProcess::SampleElevation(params) => params.requires_point_index(),
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
            Self::SeedOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::FillOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::FloodOcean(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleOceanMasked(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleOceanBelow(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress),
            Self::SampleElevation(params) => params.process_terrain_tiles_with_point_index(rng,limits,point_index,tile_map,progress)
        }
    }


}

