use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::population::generate_populations;

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenPeoplePopulation {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// A waterflow threshold above which population increases along the coast
        #[arg(long,default_value="10")]
        estuary_threshold: f64

    }
}

impl Task for GenPeoplePopulation {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            generate_populations(target, self.estuary_threshold, &mut progress)
        })?;

        target.save(&mut progress)

    }
}
