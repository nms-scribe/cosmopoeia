use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;

use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;
use rand::Rng;
use serde::Serialize;
use serde::Deserialize;
use serde_json::from_reader as from_json_reader;
use serde_json::to_string_pretty as to_json_string_pretty;
use schemars::JsonSchema;
use indexmap::IndexMap;

use crate::commands::Task;
use crate::world_map::WorldMap;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::terrain::TerrainTask;
use crate::utils::random::random_number_generator;
use crate::progress::ProgressObserver;
use crate::algorithms::terrain::LoadTerrainTask;
use crate::utils::random::RandomNth as _;
use crate::utils::arg_range::ArgRange;
use crate::raster::RasterMap;
use crate::algorithms::terrain::SampleOceanBelowLoaded;
use crate::algorithms::terrain::SampleOceanMaskedLoaded;
use crate::algorithms::terrain::SampleElevationLoaded;
use crate::commands::TargetArg;
use crate::commands::ElevationSourceArg;
use crate::commands::OceanSourceArg;
use crate::commands::RandomSeedArg;




subcommand_def!{

    /// Processes a series of pre-saved tasks
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Recipe {

        #[arg(long)]
        /// JSON File describing the tasks to complete
        pub source: PathBuf
    }
}

impl LoadTerrainTask for Recipe {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe tasks.");
        let recipe_data = File::open(self.source).map_err(|e| CommandError::RecipeFileRead(format!("{e}")))?;
        let reader = BufReader::new(recipe_data);
        let tasks: Vec<Command> = from_json_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{e}")))?;
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct RecipeSet {

        #[arg(long)]
        /// JSON file containing a map of potential recipes to follow
        pub source: PathBuf,

        #[arg(long)]
        pub recipe: Option<String>
    }


}

impl LoadTerrainTask for RecipeSet {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading recipe set.");
        let recipe_data = File::open(&self.source).map_err(|e| CommandError::RecipeFileRead(format!("{e}")))?;
        let reader = BufReader::new(recipe_data);
        let mut tasks: IndexMap<String,Vec<Command>> = from_json_reader(reader).map_err(|e| CommandError::RecipeFileRead(format!("{e}")))?;
        // Need to reproduce randomness
        tasks.sort_keys();
        progress.finish(|| "Recipe set loaded.");
        if tasks.is_empty() {
            Err(CommandError::RecipeFileRead("Recipe set is empty.".to_owned()))
        } else {
            let chosen_key = if let Some(recipe) = self.recipe {
                recipe
            } else {
                tasks.keys().choose(random).expect("Why would this fail if the len > 0?").clone()
            };
            if let Some(tasks) = tasks.remove(&chosen_key) {
                let mut result = Vec::new();
                for task in tasks {
                    result.extend(task.load_terrain_task(random,progress)?)
                }
                Ok(result)
            } else {
                Err(CommandError::RecipeFileRead(format!("Can't find recipe '{chosen_key}' in set.")))
            }

        }
    }

}


subcommand_def!{

    /// Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0.
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Clear;

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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Multiply {
        #[arg(long)]
        pub height_filter: Option<ArgRange<i8>>,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct ClearOcean;

}


impl LoadTerrainTask for ClearOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::ClearOcean(self)])
    }


}


subcommand_def!{

    /// Adds a uniform amount of random noise to the map
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct RandomUniform{

        #[arg(long)]
        pub height_filter: Option<ArgRange<i8>>,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct AddHill {

        #[arg(long)]
        pub count: ArgRange<usize>,

        #[arg(long)]
        pub height_delta: ArgRange<i8>,

        #[arg(long)]
        pub x_filter: ArgRange<f64>,

        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct AddRange {
        #[arg(long)]
        pub count: ArgRange<usize>,
        #[arg(long)]
        pub height_delta: ArgRange<i8>,
        #[arg(long)]
        pub x_filter: ArgRange<f64>,
        #[arg(long)]
        pub y_filter: ArgRange<f64>
    }
}

impl LoadTerrainTask for AddRange {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::AddRange(self)])
    }
}


#[derive(Clone,Deserialize,Serialize,ValueEnum,JsonSchema)]
pub enum StraitDirection {
    Horizontal,
    Vertical
}

subcommand_def!{

    /// Adds a long cut somewhere on the map

    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct AddStrait {
        #[arg(long)]
        pub width: ArgRange<f64>,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Mask {
        #[arg(long,default_value="1")]
        pub power: f64
    }
}

impl LoadTerrainTask for Mask {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Mask(self)])
    }


}

#[derive(Clone,Deserialize,Serialize,ValueEnum,JsonSchema)]
pub enum InvertAxes {
    X,
    Y,
    Both
}

subcommand_def!{

    /// Inverts the heights across the entire map
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Invert {
        #[arg(long)]
        pub probability: f64,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Add {
        #[arg(long)]
        pub height_filter: Option<ArgRange<i8>>,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Smooth {
        #[arg(long,default_value="2")]
        pub fr: f64
    }

}


impl LoadTerrainTask for Smooth {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Smooth(self)])
    }


}

subcommand_def!{
    /// Runs an erosion process on the map
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct Erode {
        #[arg(long,default_value="1000")]
        /// Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)
        pub weathering_amount: f64,

        #[arg(long,default_value="10")]
        pub iterations: usize
    }
}

impl LoadTerrainTask for Erode {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::Erode(self)])
    }

}


subcommand_def!{

    /// Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct SeedOcean {
        #[arg(long)]
        pub count: ArgRange<usize>,
        #[arg(long)]
        pub x_filter: ArgRange<f64>,
        #[arg(long)]
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
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct FloodOcean;

}


impl LoadTerrainTask for FloodOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::FloodOcean(self)])
    }


}


subcommand_def!{

    /// Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct FillOcean;

}


impl LoadTerrainTask for FillOcean {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, _: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        Ok(vec![TerrainTask::FillOcean(self)])
    }


}


subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean.
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct SampleOceanBelow {

        #[clap(flatten)]
        #[serde(flatten)]
        pub ocean_arg: OceanSourceArg,

        /// The elevation to compare to
        #[arg(long,allow_negative_numbers=true)]
        pub elevation: f64
    }
}


impl LoadTerrainTask for SampleOceanBelow {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(&self.ocean_arg.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainTask::SampleOceanBelow(SampleOceanBelowLoaded::new(raster,self.elevation))])
    }
}


subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean.
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct SampleOceanMasked {

        #[clap(flatten)]
        #[serde(flatten)]
        pub ocean_arg: OceanSourceArg,
    }
}



impl LoadTerrainTask for SampleOceanMasked {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading ocean raster.");
        let raster = RasterMap::open(self.ocean_arg.source)?;
        progress.finish(|| "Ocean raster loaded.");
        Ok(vec![TerrainTask::SampleOceanMasked(SampleOceanMaskedLoaded::new(raster))])
    }
}


subcommand_def!{

    /// Replaces elevations by sampling from a heightmap
    #[derive(Deserialize,Serialize,JsonSchema)]
    pub struct SampleElevation {

        #[clap(flatten)]
        #[serde(flatten)]
        pub heightmap_arg: ElevationSourceArg,
    }
}

impl LoadTerrainTask for SampleElevation {

    fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, _: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {
        progress.start_unknown_endpoint(|| "Loading elevation raster.");
        let raster = RasterMap::open(self.heightmap_arg.source)?;
        progress.finish(|| "Elevation raster loaded.");
        Ok(vec![TerrainTask::SampleElevation(SampleElevationLoaded::new(raster))])
    }
}

// FUTURE: all this to get rid of a few warnings that I can't get rid of in the derive macro output
#[allow(unused_qualifications)]
pub mod command {
    use super::*;

    #[derive(Deserialize,Serialize,Subcommand,JsonSchema)]
    #[command(disable_help_subcommand(true))]
    #[serde(tag="task")]
    pub enum Command {
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
        Erode(Erode),
        SeedOcean(SeedOcean),
        FillOcean(FillOcean),
        FloodOcean(FloodOcean),
        SampleOceanMasked(SampleOceanMasked),
        SampleOceanBelow(SampleOceanBelow),
        SampleElevation(SampleElevation),
    }
}
pub(crate) use command::Command;

impl Command {

    pub(crate) fn to_json(&self) -> Result<String,CommandError> {
        to_json_string_pretty(self).map_err(|e| CommandError::TerrainProcessWrite(format!("{e}")))
    }

    pub(crate) fn load_terrain_task<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<Vec<TerrainTask>,CommandError> {

        match self {
            Self::Clear(params) => params.load_terrain_task(random,progress),
            Self::ClearOcean(params) => params.load_terrain_task(random,progress),
            Self::RandomUniform(params) => params.load_terrain_task(random,progress),
            Self::Recipe(params) => params.load_terrain_task(random,progress),
            Self::RecipeSet(params) => params.load_terrain_task(random,progress),
            Self::AddHill(params) => params.load_terrain_task(random,progress),
            Self::AddRange(params) => params.load_terrain_task(random,progress),
            Self::AddStrait(params) => params.load_terrain_task(random,progress),
            Self::Mask(params) => params.load_terrain_task(random,progress),
            Self::Invert(params) => params.load_terrain_task(random,progress),
            Self::Add(params) => params.load_terrain_task(random,progress),
            Self::Multiply(params) => params.load_terrain_task(random,progress),
            Self::Smooth(params) => params.load_terrain_task(random,progress),
            Self::Erode(params) => params.load_terrain_task(random,progress),
            Self::SeedOcean(params) => params.load_terrain_task(random,progress),
            Self::FillOcean(params) => params.load_terrain_task(random,progress),
            Self::FloodOcean(params) => params.load_terrain_task(random,progress),
            Self::SampleOceanMasked(params) => params.load_terrain_task(random,progress),
            Self::SampleOceanBelow(params) => params.load_terrain_task(random,progress),
            Self::SampleElevation(params) => params.load_terrain_task(random,progress),
        }
    }

}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub struct Terrain {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[command(subcommand)]
        pub command: Command,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[arg(long)]
        /// Instead of processing, display the serialized value for inclusion in a recipe file.
        pub serialize: bool

    }
}

impl Task for Terrain {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(&self.random_seed_arg);

        let mut target = WorldMap::edit(&self.target_arg.target)?;

        if self.serialize {
            println!("{}",self.command.to_json()?);
            Ok(())
        } else {
            Self::run_default(&mut random, self.command, &mut target, progress)
        }


    }
}

impl Terrain {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(random: &mut Random, terrain_command: Command, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {

            progress.announce("Loading terrain processes.");

            let processes = terrain_command.load_terrain_task(random, progress)?;

            TerrainTask::process_terrain(&processes,random,transaction,progress)

        })?;

        target.save(progress)
    }
}
