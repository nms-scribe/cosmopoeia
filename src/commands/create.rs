use std::path::PathBuf;

use clap::Args;
use rand::Rng;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::utils::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::progress::ProgressObserver;
use crate::algorithms::random_points::PointGenerator;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::utils::ToGeometryCollection;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::algorithms::terrain::SampleElevationWithRaster;
use crate::world_map::ElevationLimits;

fn generate_random_tiles<Random: Rng, Progress: ProgressObserver>(random: Random, extent: Extent, tile_count: usize, progress: &mut Progress) -> Result<VoronoiGenerator<DelaunayGenerator>, CommandError> {

    let mut points = PointGenerator::new(random, extent.clone(), tile_count);
    let mut triangles = DelaunayGenerator::new(points.to_geometry_collection(progress)?);
    
    progress.announce("Generate random points");
    triangles.start(progress)?;
    let mut voronois = VoronoiGenerator::new(triangles,extent)?;
    
    progress.announce("Generate delaunay triangles");
    voronois.start(progress)?;
    
    Ok(voronois)
}



subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct CreateSourceFromHeightmap {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

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
        overwrite: bool


    }
}

impl Task for CreateSourceFromHeightmap {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let random = random_number_generator(self.seed);

        let source = RasterMap::open(self.source)?;

        progress.start_unknown_endpoint(|| "Calculating min/max from raster.");
        let limits = source.compute_min_max(1,true)?;
        progress.finish(|| "Min/max calculated.");

        let extent = source.bounds()?.extent();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let voronois = generate_random_tiles(random, extent, self.tiles, &mut progress)?;
    
        target.with_transaction(|target| {
            progress.announce("Create tiles from voronoi polygons");


            load_tile_layer(target, self.overwrite, voronois, limits, &mut progress)
        })?;


        target.save(&mut progress)
    
    }
}

subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct CreateSourceBlank {

        /// the height (from north to south) in degrees of the world extents
        height: f64,

        /// the width in degrees of the world extents
        width: f64,

        /// the latitude of the southern border of the world extents
        south: f64, 

        /// the longitude of the western border of the world extents
        west: f64,

        #[arg(long,allow_negative_numbers=true,default_value="-11000")]
        /// minimum elevation for heightmap
        min_elevation: f64,

        #[arg(long,default_value="9000")]
        /// maximum elevation for heightmap
        max_elevation: f64,

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
        overwrite: bool


    }
}

impl Task for CreateSourceBlank {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let random = random_number_generator(self.seed);

        let extent = Extent::new_with_dimensions(self.west, self.south, self.width, self.height);

        let limits = ElevationLimits::new(self.min_elevation,self.max_elevation)?;

        let mut target = WorldMap::create_or_edit(self.target)?;

        let voronois = generate_random_tiles(random, extent, self.tiles, &mut progress)?;
    
        target.with_transaction(|target| {
            progress.announce("Create tiles from voronoi polygons");

            load_tile_layer(target, self.overwrite, voronois, limits, &mut progress)
        })?;


        target.save(&mut progress)
    
    }
}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct CreateCalcNeighbors {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl Task for CreateCalcNeighbors {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Calculate neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)
        })?;

        target.save(&mut progress)


    }
}



subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct CreateFromHeightmap {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

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
        overwrite: bool


    }
}

impl Task for CreateFromHeightmap {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let random = random_number_generator(self.seed);

        let source = RasterMap::open(self.source)?;

        progress.start_unknown_endpoint(|| "Calculating min/max from raster.");
        let limits = source.compute_min_max(1,true)?;
        progress.finish(|| "Min/max calculated.");

        let extent = source.bounds()?.extent();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let voronois = generate_random_tiles(random, extent, self.tiles, &mut progress)?;
    
    
        target.with_transaction(|target| {
            progress.announce("Create tiles from voronoi polygons");

            load_tile_layer(target, self.overwrite, voronois, limits, &mut progress)?;

            progress.announce("Calculate neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)?;

            progress.announce("Sampling elevations from raster");

            let process = SampleElevationWithRaster::new(source);
            process.process_terrain(target,&mut progress)

        })?;


        target.save(&mut progress)
    
    }
}



subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct CreateBlank {

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
        overwrite: bool


    }
}

impl Task for CreateBlank {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let random = random_number_generator(self.seed);

        let extent = Extent::new_with_dimensions(self.west, self.south, self.width, self.height);

        let limits = ElevationLimits::new(self.min_elevation,self.max_elevation)?;

        let mut target = WorldMap::create_or_edit(self.target)?;

        let voronois = generate_random_tiles(random, extent, self.tiles, &mut progress)?;
    
    
        target.with_transaction(|target| {
            progress.announce("Create tiles from voronoi polygons");

            load_tile_layer(target, self.overwrite, voronois, limits, &mut progress)?;

            progress.announce("Calculate neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)

        })?;


        target.save(&mut progress)
    
    }
}
