
use clap::Args;
use clap::Subcommand;
use rand::Rng;

use crate::commands::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random::random_number_generator;
use crate::utils::extent::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::algorithms::tiles::generate_random_tiles;
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::algorithms::terrain::SampleElevationLoaded;
use crate::algorithms::terrain::TerrainTask;
use crate::world_map::property_layer::ElevationLimits;
use crate::world_map::WorldMapTransaction;
use crate::commands::TargetArg;
use crate::commands::ElevationSourceArg;
use crate::commands::terrain::Command as TerrainCommand;
use crate::commands::ElevationLimitsArg;
use crate::commands::TileCountArg;
use crate::commands::WorldShapeArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteTilesArg;

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

        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(transaction, progress)

        })?;

        target.save(progress)


    }
}

pub(crate) struct LoadedSource {
    extent: Extent,
    limits: ElevationLimits,
    post_processes: Vec<TerrainTask>
}

pub(crate) trait LoadSource {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError>;

}

#[derive(Args)]
pub struct PostProcessArg {

    #[command(subcommand)]
    /// A processing command to run after creation and elevation sampling.
    pub post_process: Option<TerrainCommand>,


}


subcommand_def!{
    /// Creates voronoi tiles in the same extent as a heightmap with zero elevation
    pub struct FromHeightmap {

        #[clap(flatten)]
        pub heightmap_arg: ElevationSourceArg,

        #[clap(flatten)]
        pub post_process_arg: PostProcessArg

    }
}

impl LoadSource for FromHeightmap {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {
        progress.announce(&format!("Loading {}",self.heightmap_arg.source.to_string_lossy()));

        let source = RasterMap::open(self.heightmap_arg.source)?;

        let extent = source.bounds()?.extent();

        progress.start_unknown_endpoint(|| "Calculating min/max from raster.");
        let limits = source.compute_min_max(1,true)?;
        progress.finish(|| "Min/max calculated.");

        // the post_processes always starts with loading the samples from the source
        let mut post_processes = vec![TerrainTask::SampleElevation(SampleElevationLoaded::new(source))];

        if let Some(process) = self.post_process_arg.post_process {
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

#[derive(Args)]
pub struct ExtentsArg {

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

}

subcommand_def!{
    /// Creates voronoi tiles in the given extent with zero elevation
    pub struct Blank {

        #[clap(flatten)]
        pub extent_arg: ExtentsArg,

        #[clap(flatten)]
        pub elevation_limits_arg: ElevationLimitsArg,

        #[clap(flatten)]
        pub post_process_arg: PostProcessArg


    }
}

impl LoadSource for Blank {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {

        let extent = Extent::new_with_dimensions(self.extent_arg.west, self.extent_arg.south, self.extent_arg.width, self.extent_arg.height);

        let limits = ElevationLimits::new(self.elevation_limits_arg.min_elevation,self.elevation_limits_arg.max_elevation)?;
        // load these earlier so we can fail quickly on loading error.
        let post_processes = if let Some(process) = self.post_process_arg.post_process {
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
pub enum Source {
    FromHeightmap(FromHeightmap),
    Blank(Blank)
}

impl LoadSource for Source {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError> {
        match self {
            Self::FromHeightmap(source) => source.load(random,progress),
            Self::Blank(source) => source.load(random,progress),
        }
    }

}

subcommand_def!{
    /// Creates the random tiles and initial elevations for a world.
    #[command(hide=true)]
    pub struct CreateTiles {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub world_shape_arg: WorldShapeArg,

        #[clap(flatten)]
        pub tile_count_arg: TileCountArg,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_tiles_arg: OverwriteTilesArg,

        #[command(subcommand)]
        pub source: Source,

    }
}

impl CreateTiles {

    fn run_with_parameters<Random: Rng, Progress: ProgressObserver>(extent: Extent, limits: &ElevationLimits, world_shape: &WorldShapeArg, tiles: &TileCountArg, overwrite: &OverwriteTilesArg, random: &mut Random, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {
        let voronois = generate_random_tiles(random, extent, world_shape.world_shape.clone(), tiles.tile_count, progress)?;
    
        progress.announce("Create tiles from voronoi polygons");

        load_tile_layer(target, overwrite, voronois, limits, &world_shape.world_shape, progress)    
    }

}


impl Task for CreateTiles {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(&self.random_seed_arg);

        let loaded_source = self.source.load(&mut random, progress)?;

        let mut target = WorldMap::create_or_edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(loaded_source.extent, &loaded_source.limits, &self.world_shape_arg, &self.tile_count_arg, &self.overwrite_tiles_arg, &mut random, transaction, progress)

        })?;

        target.save(progress)

    }
}


subcommand_def!{
    /// Creates the random tiles and initial elevations for a world.
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub tile_count_arg: TileCountArg,

        #[clap(flatten)]
        pub world_shape_arg: WorldShapeArg,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_tiles_arg: OverwriteTilesArg,

        #[command(subcommand)]
        pub source: Source,

    }
}


impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(&self.random_seed_arg);

        let loaded_source = self.source.load(&mut random, progress)?; 

        let mut target = WorldMap::create_or_edit(&self.target_arg.target)?;

        Self::run_default(&self.tile_count_arg,&self.world_shape_arg,&self.overwrite_tiles_arg,loaded_source, &mut target, &mut random, progress)

    }
}

impl Create {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(tiles: &TileCountArg, world_shape: &WorldShapeArg, overwrite_tiles: &OverwriteTilesArg, loaded_source: LoadedSource, target: &mut WorldMap, random: &mut Random, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {
            CreateTiles::run_with_parameters(loaded_source.extent, &loaded_source.limits, world_shape, tiles, overwrite_tiles, random, transaction, progress)?;

            CreateCalcNeighbors::run_with_parameters(transaction, progress)?;

            TerrainTask::process_terrain(&loaded_source.post_processes, random, transaction,progress)?;

            Ok(())

    

        })?;

        target.save(progress)
    }
}

