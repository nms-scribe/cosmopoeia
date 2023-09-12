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
use crate::algorithms::terrain::TerrainCommand;
use crate::world_map::ElevationLimits;
use crate::world_map::WorldMapTransaction;

// I don't form the subcommands for this quite the same, since I already have a subcommand for specifying the source.

subcommand_def!{
    /// Calculates neighbors for tiles
    #[command(hide=true)]
    pub(crate) struct CreateCalcNeighbors {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl CreateCalcNeighbors {

    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {
        progress.announce("Calculate neighbors for tiles");

        calculate_tile_neighbors(target, progress)
    }
}

impl Task for CreateCalcNeighbors {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            Self::run(target, progress)

        })?;

        target.save(progress)


    }
}

struct LoadedSource {
    extent: Extent,
    limits: ElevationLimits,
    post_processes: Vec<TerrainTask>
}

trait LoadCreateSource {

    fn load<Random: Rng, Progress: ProgressObserver>(self, random: &mut Random, progress: &mut Progress) -> Result<LoadedSource,CommandError>;

}

trait LoadedCreateSource {

    fn load_extents(&self) -> Extent;

    fn load_limits(&self) -> ElevationLimits;

    fn into_post_processes(self) -> Option<Vec<TerrainTask>>;
}



subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct FromHeightmap {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        #[command(subcommand)]
        /// A processing command to run after creation and elevation sampling. (see 'terrain' command)
        post_process: Option<TerrainCommand>,


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
        let mut post_processes = vec![SampleElevationLoaded::new(source)];

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
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct Blank {

        /// the height (from north to south) in degrees of the world extents
        height: f64,

        /// the width in degrees of the world extents
        width: f64,

        #[arg(allow_negative_numbers=true)]
        /// the latitude of the southern border of the world extents
        south: f64, 

        #[arg(allow_negative_numbers=true)]
        /// the longitude of the western border of the world extents
        west: f64,

        #[arg(long,allow_negative_numbers=true,default_value="-11000")]
        /// minimum elevation for heightmap
        min_elevation: f64,

        #[arg(long,default_value="9000")]
        /// maximum elevation for heightmap
        max_elevation: f64,

        #[command(subcommand)]
        /// A processing command to run after creation. (see 'terrain' command)
        post_process: Option<TerrainCommand>,


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
enum CreateSource {
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
    pub(crate) struct CreateTiles {
        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10000")]
        /// The rough number of tiles to generate for the image
        tiles: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[command(subcommand)]
        source: CreateSource,

    }
}

impl CreateTiles {

    fn run<Random: Rng, Progress: ProgressObserver>(extent: Extent, limits: ElevationLimits, tiles: usize, overwrite: bool, random: &mut Random, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {
        let voronois = generate_random_tiles(random, extent, tiles, progress)?;
    
        progress.announce("Create tiles from voronoi polygons");

        load_tile_layer(target, overwrite, voronois, &limits, progress)    
    }

}


impl Task for CreateTiles {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(self.seed);

        let loaded_source = self.source.load(&mut random, progress)?; // TODO: for the heightmap, we load the source raster, otherwise I'm not certain.

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            Self::run(loaded_source.extent, loaded_source.limits, self.tiles, self.overwrite, &mut random, target, progress)

        })?;

        target.save(progress)

    }
}


subcommand_def!{
    /// Creates the random tiles and initial elevations for a world.
    pub(crate) struct Create {
        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10000")]
        /// The rough number of tiles to generate for the image
        tiles: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[command(subcommand)]
        source: CreateSource,

    }
}


impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(self.seed);

        let loaded_source = self.source.load(&mut random, progress)?; // TODO: for the heightmap, we load the source raster, otherwise I'm not certain.

        let mut target = WorldMap::create_or_edit(self.target)?;

        Self::run(self.tiles,self.overwrite,loaded_source, &mut target, &mut random, progress)

    }
}

impl Create {
    fn run<Random: Rng, Progress: ProgressObserver>(tiles: usize, overwrite: bool, loaded_source: LoadedSource, target: &mut WorldMap, random: &mut Random, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {
            CreateTiles::run(loaded_source.extent, loaded_source.limits, tiles, overwrite, random, target, progress)?;

            CreateCalcNeighbors::run(target, progress)?;

            TerrainTask::process_terrain(&loaded_source.post_processes, random, target,progress)?;

            Ok(())

    

        })?;

        target.save(progress)
    }
}

