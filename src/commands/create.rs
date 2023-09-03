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
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::algorithms::terrain::TerrainSettings;
use crate::algorithms::terrain::SampleElevationWithRaster;



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


            load_tile_layer(target, self.overwrite, voronois, &mut progress)?;

            progress.announce("Calculate neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)?;

            progress.announce("Sampling elevations from raster");

            // even though this is technically a terrain command, it's something that is probably expected
            let settings = TerrainSettings::from_raster(&source, &mut progress)?;
            let process = SampleElevationWithRaster::new(source);
            process.process_terrain(settings,target,&mut progress)

        })?;


        target.save(&mut progress)
    
    }
}
