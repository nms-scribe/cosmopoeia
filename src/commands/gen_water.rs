use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFlow {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterFlow {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.generate_water_flow(&mut progress)?;

        Ok(())


    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFill {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterFill {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let (tile_map,tile_queue) = target.get_tile_map_and_queue_for_water_fill(&mut progress)?;

        target.generate_water_fill(tile_map,tile_queue,&mut progress)?;

        Ok(())


    }
}