use std::path::PathBuf;

use clap::Args;
use gdal::Dataset;
use gdal::DriverManager;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::terrain::generate_points_from_heightmap;
use crate::utils::random_number_generator;

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
        let source = Dataset::open(self.source)?;
        let target_driver = DriverManager::get_driver_by_name(&self.target_driver)?;
        let mut target = target_driver.create_vector_only(self.target)?;
        generate_points_from_heightmap(source,&mut target,self.spacing,&mut random_number_generator(self.seed))?;
        Ok(())
    }
}

