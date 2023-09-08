use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::terrain::TerrainProcessCommand; 
use crate::algorithms::terrain::TerrainProcess;
use crate::utils::random_number_generator;


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct Terrain {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[command(subcommand)]
        process: TerrainProcessCommand,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// Instead of processing, display the serialized value for inclusion in a recipe file.
        serialize: bool

    }
}

impl Task for Terrain {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            if self.serialize {
                println!("{}",self.process.to_json()?);
            } else {
                progress.announce("Loading terrain processes.");

                let processes = self.process.load_terrain_processes(&mut random, &mut progress)?;

                TerrainProcess::process_terrain(&processes,&mut random,target,&mut progress)?;
            }
    
            Ok(())

        })?;

        target.save(&mut progress)


    }
}