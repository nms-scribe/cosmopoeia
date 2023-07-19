use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::VoronoiGenerator;
use crate::world_map::VoronoiTile;


    

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevVoronoiFromTriangles {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevVoronoiFromTriangles {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let mut triangles = target.triangles_layer()?;
    
        let mut generator = VoronoiGenerator::new(triangles.read_triangles());
    
        progress.start_unknown_endpoint(|| "Generating voronoi.");
        
        generator.start()?;
    
        progress.finish(|| "Voronoi generated.");

        progress.start(|| ("Copying voronoi.",generator.size_hint().1));
        
        let voronoi: Vec<Result<VoronoiTile,CommandError>> = generator.collect();
    
        target.load_tile_layer(self.overwrite, voronoi.into_iter(), &mut progress)
    
    
    }
}

