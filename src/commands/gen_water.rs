use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::progress::ProgressObserver;

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFlowage {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterFlowage {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.generate_flowage(&mut progress)?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        progress.finish(|| "Layer Saved.");

        Ok(())


    }
}