use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::str::FromStr;
use std::fmt::Display;

use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;
use rand::Rng;
use serde::Serialize;
use serde::Deserialize;
use serde_json::from_reader as from_json_reader;
use serde_json::to_string_pretty as to_json_string_pretty;

use super::Task;
use crate::world_map::WorldMap;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::terrain::TerrainTask;
use crate::utils::random_number_generator;
use crate::progress::ProgressObserver;
use crate::algorithms::terrain::LoadTerrainTask;
use crate::utils::RandomNth;
use crate::utils::ArgRange;
use crate::raster::RasterMap;
use crate::algorithms::terrain::SampleOceanBelowLoaded;
use crate::algorithms::terrain::SampleOceanMaskedLoaded;
use crate::algorithms::terrain::SampleElevationLoaded;


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


subcommand_def!{

    /// Processes a series of pre-saved tasks
    #[derive(Deserialize,Serialize)]
    pub struct Recipe {

        /// Raster file defining new elevations
        pub source: PathBuf
    }
}

impl LoadTerrainTask for Recipe {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe tasks.");
        let recipe_data = File::open(&self.source).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        let reader = BufReader::new(recipe_data);
        let tasks: Vec<TerrainCommand> = from_json_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        progress.finish(|| "Recipe tasks loaded.");
        let mut result = Vec::new();
        for task in tasks {
            result.extend(task.load_terrain_task(random,progress)?)
        }
        Ok(result)
    }

}


subcommand_def!{

    /// Randomly chooses a recipe from a set of named recipes and follows it
    #[derive(Deserialize,Serialize)]
    pub struct RecipeSet {

        /// Raster file defining new elevations
        pub source: PathBuf,

        #[arg(long)]
        pub recipe: Option<String>
    }


}

impl LoadTerrainTask for RecipeSet {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe set.");
        let recipe_data = File::open(&self.source).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        let reader = BufReader::new(recipe_data);
        let mut tasks: HashMap<String,Vec<TerrainCommand>> = from_json_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{}",e)))?;
        progress.finish(|| "Recipe set loaded.");
        if tasks.len() > 0 {
            let chosen_key = if let Some(recipe) = self.recipe {
                recipe
            } else {
                tasks.keys().choose(random).expect("Why would this fail if the len > 0?").to_owned() 
            };
            if let Some(tasks) = tasks.remove(&chosen_key) {
                let mut result = Vec::new();
                for task in tasks {
                    result.extend(task.load_terrain_task(random,progress)?)
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

    /// Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0.
    #[derive(Deserialize,Serialize)]
    pub struct Clear{}
    
}

impl LoadTerrainTask for Clear {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Multiply(Multiply { 
            height_filter: None, 
            height_factor: 0.0
        })])
    }


}



subcommand_def!{

    /// Inverts the heights across the entier map
    #[derive(Deserialize,Serialize)]
    pub struct Multiply {
        pub height_filter: Option<ArgRange<i8>>, 
        pub height_factor: f64 // this doesn't have to be i8 because it's a multiplication, will still work no matter what the scale.
    }
    
}


impl LoadTerrainTask for Multiply {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Multiply(self)])
    }


}


subcommand_def!{

    /// Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
    #[derive(Deserialize,Serialize)]
    pub struct ClearOcean{}
    
}


impl LoadTerrainTask for ClearOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::ClearOcean(self)])
    }


}


subcommand_def!{

    /// Adds a uniform amount of random noise to the map
    #[derive(Deserialize,Serialize)]
    pub struct RandomUniform{

        pub height_filter: Option<ArgRange<i8>>, 
        pub height_delta: ArgRange<i8>
    }
    
}


impl LoadTerrainTask for RandomUniform {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::RandomUniform(self)])
    }


}


subcommand_def!{

    /// Adds hills or pits to a certain area of the map
    #[derive(Deserialize,Serialize)]
    pub struct AddHill {

        pub count: ArgRange<usize>,

        pub height_delta: ArgRange<i8>,

        pub x_filter: ArgRange<f64>,

        pub y_filter: ArgRange<f64>

    }
}

impl LoadTerrainTask for AddHill {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::AddHill(self)])
    }
}


subcommand_def!{

    /// Adds a range of heights or a trough to a certain area of a map
    #[derive(Deserialize,Serialize)]
    pub struct AddRange {
        pub count: ArgRange<usize>,
        pub height_delta: ArgRange<i8>,
        pub x_filter: ArgRange<f64>,
        pub y_filter: ArgRange<f64>
    }
}

impl LoadTerrainTask for AddRange {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::AddRange(self)])
    }
}


#[derive(Clone,Deserialize,Serialize,ValueEnum)]
pub enum StraitDirection {
    Horizontal,
    Vertical
}

subcommand_def!{

    /// Adds a long cut somewhere on the map

    #[derive(Deserialize,Serialize)]
    pub struct AddStrait { 
        pub width: ArgRange<f64>,
        pub direction: StraitDirection
    }

}

impl LoadTerrainTask for AddStrait {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::AddStrait(self)])
    }

}


subcommand_def!{

    /// Changes the heights based on their distance from the edge of the map
    #[derive(Deserialize,Serialize)]
    pub struct Mask {
        #[arg(default_value="1")]
        pub power: f64
    }
}

impl LoadTerrainTask for Mask {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Mask(self)])
    }


}

#[derive(Clone,Deserialize,Serialize,ValueEnum)]
pub enum InvertAxes {
    X,
    Y,
    Both
}

subcommand_def!{

    /// Inverts the heights across the entire map
    #[derive(Deserialize,Serialize)]
    pub struct Invert {
        pub probability: f64, 
        pub axes: InvertAxes
    }
    
}


impl LoadTerrainTask for Invert {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Invert(self)])
    }


}


subcommand_def!{

    /// Inverts the heights across the entier map
    #[derive(Deserialize,Serialize)]
    pub struct Add {
        pub height_filter: Option<ArgRange<i8>>, 
        pub height_delta: i8
    }
    
}


impl LoadTerrainTask for Add {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Add(self)])
    }


}


subcommand_def!{

    /// Smooths elevations by averaging the value against it's neighbors.
    #[derive(Deserialize,Serialize)]
    pub struct Smooth {
        #[arg(default_value="2")]
        pub fr: f64
    }
    
}


impl LoadTerrainTask for Smooth {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Smooth(self)])
    }


}


subcommand_def!{

    /// Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
    #[derive(Deserialize,Serialize)]
    pub struct SeedOcean {
        pub count: ArgRange<usize>,
        pub x_filter: ArgRange<f64>,
        pub y_filter: ArgRange<f64>
    }
    
}


impl LoadTerrainTask for SeedOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::SeedOcean(self)])
    }


}


subcommand_def!{

    /// Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found.
    #[derive(Deserialize,Serialize)]
    pub struct FloodOcean{}
    
}


impl LoadTerrainTask for FloodOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::FloodOcean(self)])
    }


}


subcommand_def!{

    /// Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
    #[derive(Deserialize,Serialize)]
    pub struct FillOcean{}
    
}


impl LoadTerrainTask for FillOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::FillOcean(self)])
    }


}


subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean.
    #[derive(Deserialize,Serialize)]
    pub struct SampleOceanBelow {

        /// The raster to sample from
        pub source: PathBuf,

        /// The elevation to compare to
        #[arg(allow_negative_numbers=true)]
        pub elevation: f64
    }
}


impl LoadTerrainTask for SampleOceanBelow {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainTask::SampleOceanBelow(SampleOceanBelowLoaded::new(raster,self.elevation))])
    }
}


subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean.
    #[derive(Deserialize,Serialize)]
    pub struct SampleOceanMasked {

        /// The raster to read ocean data from
        pub source: PathBuf
    }
}



impl LoadTerrainTask for SampleOceanMasked {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainTask::SampleOceanMasked(SampleOceanMaskedLoaded::new(raster))])
    }
}


subcommand_def!{

    /// Replaces elevations by sampling from a heightmap
    #[derive(Deserialize,Serialize)]
    pub struct SampleElevation {

        /// Raster file defining new elevations
        pub source: PathBuf
    }
}

impl LoadTerrainTask for SampleElevation {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading elevation raster.");
        let raster = RasterMap::open(&self.source)?;
        progress.finish(|| "Elevation raster loaded.");
        Ok(vec![TerrainTask::SampleElevation(SampleElevationLoaded::new(raster))])
    }
}


#[derive(Deserialize,Serialize,Subcommand)]
#[command(disable_help_subcommand(true))]
pub enum TerrainCommand {
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

impl TerrainCommand {

    pub(crate) fn to_json(&self) -> Result<String,CommandError> {
        // NOTE: Not technically a recipe read error, but this shouldn't be used very often.
        Ok(to_json_string_pretty(self).map_err(|e| CommandError::TerrainProcessWrite(format!("{}",e)))?)
    }

    pub(crate) fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {

        match self {
            TerrainCommand::Clear(params) => params.load_terrain_task(random,progress),
            TerrainCommand::ClearOcean(params) => params.load_terrain_task(random,progress),
            TerrainCommand::RandomUniform(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Recipe(params) => params.load_terrain_task(random,progress),
            TerrainCommand::RecipeSet(params) => params.load_terrain_task(random,progress),
            TerrainCommand::AddHill(params) => params.load_terrain_task(random,progress),
            TerrainCommand::AddRange(params) => params.load_terrain_task(random,progress),
            TerrainCommand::AddStrait(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Mask(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Invert(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Add(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Multiply(params) => params.load_terrain_task(random,progress),
            TerrainCommand::Smooth(params) => params.load_terrain_task(random,progress),
            TerrainCommand::SeedOcean(params) => params.load_terrain_task(random,progress),
            TerrainCommand::FillOcean(params) => params.load_terrain_task(random,progress),
            TerrainCommand::FloodOcean(params) => params.load_terrain_task(random,progress),
            TerrainCommand::SampleOceanMasked(params) => params.load_terrain_task(random,progress),
            TerrainCommand::SampleOceanBelow(params) => params.load_terrain_task(random,progress),
            TerrainCommand::SampleElevation(params) => params.load_terrain_task(random,progress),
        }
    }

}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub struct Terrain {

        /// The path to the world map GeoPackage file
        pub target: PathBuf,

        #[command(subcommand)]
        pub command: TerrainCommand,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        pub seed: Option<u64>,

        #[arg(long)]
        /// Instead of processing, display the serialized value for inclusion in a recipe file.
        pub serialize: bool

    }
}

impl Task for Terrain {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::create_or_edit(self.target)?;

        if self.serialize {
            println!("{}",self.command.to_json()?);
            Ok(())
        } else {
            Self::run_default(&mut random, self.command, &mut target, progress)
        }


    }
}

impl Terrain {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(random: &mut Random, terrain_command: TerrainCommand, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {

            progress.announce("Loading terrain processes.");

            let processes = terrain_command.load_terrain_task(random, progress)?;

            TerrainTask::process_terrain(&processes,random,target,progress)

        })?;

        target.save(progress)
    }
}