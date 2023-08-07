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
use crate::algorithms::random_points::PointGenerator;
use crate::progress::ProgressObserver;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::utils::ToGeometryCollection;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::random_points::load_points_layer;
use crate::algorithms::triangles::load_triangles_layer;
use crate::algorithms::tiles::load_tile_layer;
use crate::world_map::NewTileEntity;


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
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

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

        target.with_transaction(|target| {
            load_points_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)?;
        
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
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

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
        
        target.with_transaction(|target| {
            load_points_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)?;

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
    
        let mut generator = DelaunayGenerator::new(points.read_geometries().to_geometry_collection(&mut progress)?);
    
        generator.start(&mut progress)?;
    
        target.with_transaction(|target| {
            load_triangles_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)
        
        
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
    
        let mut generator = VoronoiGenerator::new(triangles.read_geometries(),extent)?;
    
        generator.start(&mut progress)?;
    
        progress.start(|| ("Copying voronoi.",generator.size_hint().1));
        
        let voronoi: Vec<Result<NewTileEntity,CommandError>> = generator.collect();

        progress.finish(|| "Voronoi copied.");

        target.with_transaction(|target| {
            load_tile_layer(target,self.overwrite,voronoi.into_iter(),&mut progress)
        })?;

        target.save(&mut progress)
    
    
    }
}
