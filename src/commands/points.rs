use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::generate_points_from_heightmap;
use crate::utils::random_number_generator;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub struct DevPointsFromHeightmap {
        source: PathBuf,

        target: PathBuf,

        #[arg(long)]
        target_driver: String,

        #[arg(long)]
        /// The rough number of pixels horizontally separating each point [Default: a value that places about 10k points]
        spacing: Option<f64>,

        #[arg(long)]
        /// Seeds for the random number generator (up to 32), note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Vec<u8>
    }
}

impl Task for DevPointsFromHeightmap {

    fn run(self) -> Result<(),CommandError> {
        let source = RasterMap::open(self.source)?;
        let mut target = WorldMap::create(&self.target_driver,self.target)?;
        generate_points_from_heightmap(source,&mut target,self.spacing,&mut random_number_generator(self.seed),&mut Some(&mut ConsoleProgressBar::new()))?;
        Ok(())
    }
}

