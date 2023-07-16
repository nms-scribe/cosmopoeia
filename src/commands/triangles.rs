use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::generate_delaunary_triangles_from_points;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub struct DevTrianglesFromPoints {

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
        let mut target = WorldMap::edit(self.target)?;
        generate_delaunary_triangles_from_points(&mut target,self.overwrite,self.tolerance,&mut Some(&mut ConsoleProgressBar::new()))?;
        Ok(())
    }
}

