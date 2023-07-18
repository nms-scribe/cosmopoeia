use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::DelaunayGenerator;
use crate::utils::ToGeometryCollection;

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevTrianglesFromPoints {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// Optional snapping tolerance to pass to delaunay algorithm
        #[arg(long)]
        tolerance: Option<f64>,

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

