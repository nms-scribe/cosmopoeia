use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::random_points::PointGenerator;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::utils::ToGeometryCollection;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::raster_sampling::OceanSamplingMethod;
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::algorithms::tiles::calculate_coastline;
use crate::algorithms::raster_sampling::sample_elevations_on_tiles;
use crate::algorithms::raster_sampling::sample_ocean_on_tiles;


subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use with nfmt
    pub(crate) struct ConvertHeightmap {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// specifies a layer to sample ocean status from, if specified, all data cells on the layer will be considered ocean, unless you specify another ocean option. If not specified only non-data cells will be considered ocean unless you specify another option.
        #[arg(long)]
        ocean: Option<PathBuf>,

        /// if specified, tiles which have no elevation data are considered ocean.
        #[arg(long)]
        ocean_no_data: bool,

        /// if specified, tiles below the specified value are considered ocean.
        #[arg(long)]
        ocean_below: Option<f64>,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy coastlines. The higher the number, the smoother the curves.
        bezier_scale: f64,

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

impl Task for ConvertHeightmap {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let extent = source.bounds()?.extent();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let random = random_number_generator(self.seed);


        // point generator

        let mut points = PointGenerator::new(random, extent.clone(), self.tiles);
        
        // triangle calculator

        let mut triangles = DelaunayGenerator::new(points.to_geometry_collection(&mut progress)?);

        progress.announce("Generate random points");
    
        triangles.start(&mut progress)?;
    
        // voronoi calculator

        let mut voronois = VoronoiGenerator::new(triangles,extent)?;
    
        progress.announce("Generate delaunay triangles");
    
        voronois.start(&mut progress)?;
    
        target.with_transaction(|target| {

            progress.announce("Create tiles from voronoi polygons");

            load_tile_layer(target, self.overwrite, voronois, &mut progress)?;

            // TODO: Some of the following could be done at the same time. Instead of iterating through all the tiles
            // three times, just iterate once and 1) sample elevations, 2) sample ocean and 3) calculate neighbors.
            // I just have to find a way to do that all at once while still being able to keep it separate for testing.

            progress.announce("Sample elevations from heightmap");

            sample_elevations_on_tiles(target, &source, &mut progress)?;

            // ocean layer, need to do this here because we might be moving source
            let (ocean,ocean_method) = if let Some(ocean) = self.ocean {
                (RasterMap::open(ocean)?,
                match (self.ocean_no_data,self.ocean_below) {
                    (true, None) => OceanSamplingMethod::NoData,
                    (true, Some(a)) => OceanSamplingMethod::NoDataAndBelow(a),
                    (false, None) => OceanSamplingMethod::AllData,
                    (false, Some(a)) => OceanSamplingMethod::Below(a),
                })
            } else {
                (source,match (self.ocean_no_data,self.ocean_below) {
                    (true, None) => OceanSamplingMethod::NoData,
                    (true, Some(a)) => OceanSamplingMethod::NoDataAndBelow(a),
                    (false, None) => OceanSamplingMethod::NoData,
                    (false, Some(a)) => OceanSamplingMethod::Below(a),
                })
            };


            progress.announce("Sample ocean data from heightmap");


            sample_ocean_on_tiles(target, &ocean, ocean_method, &mut progress)?;
    
            // calculate neighbors
            progress.announce("Calculate neighbors for tiles");


            calculate_tile_neighbors(target, &mut progress)?;

            progress.announce("Creating coastline");

            calculate_coastline(target, self.bezier_scale, self.overwrite, self.overwrite, &mut progress)

        })?;

        target.save(&mut progress)

    
    }
}



subcommand_def!{
    /// Converts a heightmap into voronoi tiles for use in nfmt, but doesn't fill in any data.
    pub(crate) struct ConvertHeightmapVoronoi {

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

impl Task for ConvertHeightmapVoronoi {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let extent = source.bounds()?.extent();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let random = random_number_generator(self.seed);

        // point generator

        let mut points = PointGenerator::new(random, extent.clone(), self.tiles);
        
        // triangle calculator

        let mut triangles = DelaunayGenerator::new(points.to_geometry_collection(&mut progress)?);

        progress.announce("Generate random points");
    
        triangles.start(&mut progress)?;
    
        // voronoi calculator

        // TODO: What if we didn't bother with voronois? The triangles could be tiles as well, we just need to find their centroid (not circumcenter, which isn't always inside the tile). I don't know if the coastlines will look less game-like or not.

        let mut voronois = VoronoiGenerator::new(triangles,extent)?;

        progress.announce("Generate delaunay triangles");

        voronois.start(&mut progress)?;
    
        target.with_transaction(|target| {
            progress.announce("Create tiles from voronoi polygons");


            load_tile_layer(target, self.overwrite, voronois, &mut progress)
        })?;


        target.save(&mut progress)
    
    }
}


subcommand_def!{
    /// Samples elevation data from a heightmap into tiles
    pub(crate) struct ConvertHeightmapSample {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,
    }
}

impl Task for ConvertHeightmapSample {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let mut target = WorldMap::create_or_edit(self.target)?;

        progress.announce("Sample elevations from heightmap");

        // sample elevations
        target.with_transaction(|target| {
            sample_elevations_on_tiles(target, &source, &mut progress)
        })?;

        target.save(&mut progress)
    
    }
}



subcommand_def!{
    /// Samples ocean flag from a heightmap into tiles
    pub(crate) struct ConvertHeightmapOcean {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// specifies a layer to sample ocean status from, if specified, all data cells on the layer will be considered ocean, unless you specify another ocean option. If not specified only non-data cells will be considered ocean unless you specify another option.
        #[arg(long)]
        ocean: Option<PathBuf>,

        /// if specified, tiles which have no elevation data are considered ocean.
        #[arg(long)]
        ocean_no_data: bool,

        /// if specified, tiles below the specified value are considered ocean.
        #[arg(long)]
        ocean_below: Option<f64>,


    }
}

impl Task for ConvertHeightmapOcean {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let mut target = WorldMap::create_or_edit(self.target)?;

        // ocean layer
        let (ocean,ocean_method) = if let Some(ocean) = self.ocean {
            (RasterMap::open(ocean)?,
            match (self.ocean_no_data,self.ocean_below) {
                (true, None) => OceanSamplingMethod::NoData,
                (true, Some(a)) => OceanSamplingMethod::NoDataAndBelow(a),
                (false, None) => OceanSamplingMethod::AllData,
                (false, Some(a)) => OceanSamplingMethod::Below(a),
            })
        } else {
            (source,match (self.ocean_no_data,self.ocean_below) {
                (true, None) => OceanSamplingMethod::NoData,
                (true, Some(a)) => OceanSamplingMethod::NoDataAndBelow(a),
                (false, None) => OceanSamplingMethod::NoData,
                (false, Some(a)) => OceanSamplingMethod::Below(a),
            })
        };


        // calculate neighbors

        target.with_transaction(|target| {

            progress.announce("Sampling ocean data from heightmap");

            sample_ocean_on_tiles(target, &ocean, ocean_method, &mut progress)?;

            progress.announce("Calculating neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)
        })?;

        target.save(&mut progress)

    
    }
}

