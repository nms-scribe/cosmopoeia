use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::utils::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::algorithms::tiles::generate_random_tiles;
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::algorithms::terrain::SampleElevationLoaded;
use crate::algorithms::terrain::TerrainTask;
use crate::commands::terrain::TerrainCommand;
use crate::world_map::ElevationLimits;
use crate::world_map::WorldMapTransaction;
use crate::commands::TargetArg;

// I don't form the subcommands for this quite the same, since I already have a subcommand for specifying the source.

subcommand_def!{
    /// Calculates neighbors for tiles
    #[command(hide=true)]
    pub struct CreateCalcNeighbors {

        #[clap(flatten)]
        pub target_arg: TargetArg


    }
}

impl CreateCalcNeighbors {

    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {
        progress.announce("Calculate neighbors for tiles");

        calculate_tile_neighbors(target, progress)
    }
}

impl Task for CreateCalcNeighbors {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(target, progress)

        })?;

        target.save(progress)


    }
}

pub(crate) struct LoadedSource {
    extent: Extent,
    limits: ElevationLimits,
    post_processes: Vec<TerrainTask>
}

pub(crate) trait LoadCreateSource {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError>;

}

trait LoadedCreateSource {

    fn load_extents(&self) -> Extent;

    fn load_limits(&self) -> ElevationLimits;

    fn into_post_processes(self) -> Option<Vec<TerrainTask>>;
}



subcommand_def!{
    /// Creates voronoi tiles in the same extent as a heightmap with zero elevation
    pub struct FromHeightmap {

        /// The path to the heightmap containing the elevation data
        pub source: PathBuf,

        #[command(subcommand)]
        /// A processing command to run after creation and elevation sampling. (see 'terrain' command)
        pub post_process: Option<TerrainCommand>,


    }
}

impl LoadCreateSource for FromHeightmap {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {
        progress.announce(&format!("Loading {}",self.source.to_string_lossy()));

        let source = RasterMap::open(self.source)?;

        let extent = source.bounds()?.extent();

        progress.start_unknown_endpoint(|| "Calculating min/max from raster.");
        let limits = source.compute_min_max(1,true)?;
        progress.finish(|| "Min/max calculated.");

        // the post_processes always starts with loading the samples from the source
        let mut post_processes = vec![TerrainTask::SampleElevation(SampleElevationLoaded::new(source))];

        if let Some(process) = self.post_process {
            progress.announce("Loading terrain processes.");

            post_processes.extend(process.load_terrain_task(random, progress)?);

        };

        Ok(LoadedSource {
            extent,
            limits,
            post_processes
        })
    }

}

subcommand_def!{
    /// Creates voronoi tiles in the given extent with zero elevation
    pub struct Blank {

        /// the height (from north to south) in degrees of the world extents
        pub height: f64,

        /// the width in degrees of the world extents
        pub width: f64,

        #[arg(allow_negative_numbers=true)]
        /// the latitude of the southern border of the world extents
        pub south: f64, 

        #[arg(allow_negative_numbers=true)]
        /// the longitude of the western border of the world extents
        pub west: f64,

        #[arg(long,allow_negative_numbers=true,default_value="-11000")]
        /// minimum elevation for heightmap
        pub min_elevation: f64,

        #[arg(long,default_value="9000")]
        /// maximum elevation for heightmap
        pub max_elevation: f64,

        #[command(subcommand)]
        /// A processing command to run after creation. (see 'terrain' command)
        pub post_process: Option<TerrainCommand>,


    }
}

impl LoadCreateSource for Blank {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {

        let extent = Extent::new_with_dimensions(self.west, self.south, self.width, self.height);

        let limits = ElevationLimits::new(self.min_elevation,self.max_elevation)?;
        // load these earlier so we can fail quickly on loading error.
        let post_processes = if let Some(process) = self.post_process {
            progress.announce("Loading terrain processes.");

            process.load_terrain_task(random, progress)?

        } else {
            Vec::new()
        };

        Ok(LoadedSource {
            extent,
            limits,
            post_processes,
        })

    }

}


#[derive(Subcommand)]
#[command(subcommand_value_name="SOURCE")]
#[command(subcommand_help_heading("Sources"))]
#[command(disable_help_subcommand(true))]
pub enum CreateSource {
    FromHeightmap(FromHeightmap),
    Blank(Blank)
}

impl LoadCreateSource for CreateSource {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {
        match self {
            CreateSource::FromHeightmap(source) => source.load(random,progress),
            CreateSource::Blank(source) => source.load(random,progress),
        }
    }

}



subcommand_def!{
    /// Creates the random tiles and initial elevations for a world.
    #[command(hide=true)]
    pub struct CreateTiles {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="10000")]
        /// The rough number of tiles to generate for the image
        pub tiles: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        pub seed: Option<u64>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        pub overwrite: bool,

        #[command(subcommand)]
        pub source: CreateSource,

    }
}

impl CreateTiles {

    fn run_with_parameters<Random: Rng, Progress: ProgressObserver>(extent: Extent, limits: ElevationLimits, tiles: usize, overwrite: bool, random: &mut Random, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {
        let voronois = generate_random_tiles(random, extent, tiles, progress)?;
    
        progress.announce("Create tiles from voronoi polygons");

        load_tile_layer(target, overwrite, voronois, &limits, progress)    
    }

}


impl Task for CreateTiles {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(self.seed);

        let loaded_source = self.source.load(&mut random, progress)?;

        let mut target = WorldMap::create_or_edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(loaded_source.extent, loaded_source.limits, self.tiles, self.overwrite, &mut random, target, progress)

        })?;

        target.save(progress)

    }
}


subcommand_def!{
    /// Creates the random tiles and initial elevations for a world.
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="10000")]
        /// The rough number of tiles to generate for the image
        pub tiles: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        pub seed: Option<u64>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        pub overwrite: bool,

        #[command(subcommand)]
        pub source: CreateSource,

    }
}


impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(self.seed);

        let loaded_source = self.source.load(&mut random, progress)?; 

        let mut target = WorldMap::create_or_edit(self.target_arg.target)?;

        Self::run_default(self.tiles,self.overwrite,loaded_source, &mut target, &mut random, progress)

    }
}

impl Create {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(tiles: usize, overwrite_tiles: bool, loaded_source: LoadedSource, target: &mut WorldMap, random: &mut Random, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {
            CreateTiles::run_with_parameters(loaded_source.extent, loaded_source.limits, tiles, overwrite_tiles, random, target, progress)?;

            CreateCalcNeighbors::run_with_parameters(target, progress)?;

            TerrainTask::process_terrain(&loaded_source.post_processes, random, target,progress)?;

            Ok(())

    

        })?;

        target.save(progress)
    }
}

