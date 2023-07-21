use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::utils::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::PointGenerator;
use crate::progress::ProgressObserver;
use crate::algorithms::DelaunayGenerator;
use crate::utils::ToGeometryCollection;
use crate::algorithms::VoronoiGenerator;
use crate::world_map::VoronoiTile;
use crate::world_map::OceanSamplingMethod;


subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevPointsFromHeightmap {
        // Path to the source height map to get extents from
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        points: usize,

        #[arg(long)]
        /// Seeds for the random number generator (up to 32), note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Vec<u8>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevPointsFromHeightmap {

    fn run(self) -> Result<(),CommandError> {
        let source = RasterMap::open(self.source)?;
        let extent = source.bounds()?.extent();
        let mut target = WorldMap::create_or_edit(self.target)?;
        let random = random_number_generator(self.seed);
        let mut progress = ConsoleProgressBar::new();
        let generator = PointGenerator::new(random, extent, self.points);
        
        target.load_points_layer(self.overwrite, generator, &mut Some(&mut progress))?;

        Ok(())
    }
}

subcommand_def!{
    /// Creates a random points vector layer given an extent
    #[command(hide=true)]
    pub(crate) struct DevPointsFromExtent {
        #[arg(allow_hyphen_values=true)]
        west: f64,

        #[arg(allow_hyphen_values=true)]
        south: f64,

        #[arg(allow_hyphen_values=true)]
        north: f64,

        #[arg(allow_hyphen_values=true)]
        east: f64,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// The rough number of pixels horizontally separating each point [Default: a value that places about 10k points]
        spacing: Option<f64>,

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        points: usize,

        #[arg(long)]
        /// Seeds for the random number generator (up to 32), note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Vec<u8>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevPointsFromExtent {

    fn run(self) -> Result<(),CommandError> {
        let extent = Extent::new(self.west,self.south,self.east,self.north);
        let mut target = WorldMap::create_or_edit(self.target)?;
        let random = random_number_generator(self.seed);
        let mut progress = ConsoleProgressBar::new();
        let generator = PointGenerator::new(random, extent, self.points);
        
        target.load_points_layer(self.overwrite, generator, &mut Some(&mut progress))?;

        Ok(())
    }
}

subcommand_def!{
    /// Creates delaunay triangles from a points layer
    #[command(hide=true)]
    pub(crate) struct DevTrianglesFromPoints {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevTrianglesFromPoints {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let mut points = target.points_layer()?;
    
        let mut generator = DelaunayGenerator::new(points.read_points().to_geometry_collection(&mut progress)?);
    
        progress.start_unknown_endpoint(|| "Generating triangles.");
        
        generator.start()?;
    
        progress.finish(|| "Triangles generated.");
    
        target.load_triangles_layer(self.overwrite, generator, &mut progress)
    
    
    }
}

subcommand_def!{
    /// Creates voronoi tiles out of a delaunay triangles layer
    #[command(hide=true)]
    pub(crate) struct DevVoronoiFromTriangles {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        extents: PathBuf,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevVoronoiFromTriangles {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let extent = {
            let source = RasterMap::open(self.extents)?;
            source.bounds()?.extent()
        };

        let mut target = WorldMap::edit(self.target)?;

        let mut triangles = target.triangles_layer()?;
    
        let mut generator = VoronoiGenerator::new(triangles.read_triangles(),extent)?;
    
        progress.start_unknown_endpoint(|| "Generating voronoi.");
        
        generator.start()?;
    
        progress.finish(|| "Voronoi generated.");

        progress.start(|| ("Copying voronoi.",generator.size_hint().1));
        
        let voronoi: Vec<Result<VoronoiTile,CommandError>> = generator.collect();
    
        target.load_tile_layer(self.overwrite, voronoi.into_iter(), &mut progress)
    
    
    }
}


subcommand_def!{
    /// Calculates neighbors for a voronoi tile layer
    #[command(hide=true)]
    pub(crate) struct DevVoronoiNeighbors {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for DevVoronoiNeighbors {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.calculate_tile_neighbors(&mut progress)

    }
}

subcommand_def!{
    /// Samples heights from an elevation map onto a voronoi tiles layer
    #[command(hide=true)]
    pub(crate) struct DevSampleHeightsToVoronoi {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for DevSampleHeightsToVoronoi {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let mut target = WorldMap::edit(self.target)?;

        target.sample_elevations_on_tiles(&source,&mut progress)
    
    }
}

subcommand_def!{
    /// Samples heights from an elevation map onto a voronoi tiles layer
    #[command(hide=true)]
    pub(crate) struct DevSampleOceanToVoronoi {

        /// The path to the heightmap containing the elevation data
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// If there is no data at the specified location, it is considered ocean
        #[arg(long)]
        no_data: bool,

        // If the value is below this value, it is considered ocean. If not specified, all data will be ocean.
        #[arg(long)]
        below: Option<f64>,

    }
}

impl Task for DevSampleOceanToVoronoi {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let source = RasterMap::open(self.source)?;

        let mut target = WorldMap::edit(self.target)?;

        let ocean_method = match (self.no_data,self.below) {
            (true, None) => OceanSamplingMethod::NoData,
            (true, Some(a)) => OceanSamplingMethod::NoDataAndBelow(a),
            (false, None) => OceanSamplingMethod::AllData,
            (false, Some(a)) => OceanSamplingMethod::Below(a),
        };


        target.sample_ocean_on_tiles(&source,ocean_method,&mut progress)
    
    }
}


