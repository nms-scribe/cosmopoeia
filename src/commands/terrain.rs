use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;

use super::Task;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::terrain::SampleElevation;
use crate::algorithms::terrain::SampleOceanBelow;
use crate::algorithms::terrain::SampleOceanMasked;
use crate::algorithms::terrain::TerrainProcess;
use crate::algorithms::terrain::Recipe;


#[derive(Subcommand)]
enum TerrainProcessCommand {
    SampleElevation(SampleElevation),
    SampleOceanMasked(SampleOceanMasked),
    SampleOceanBelow(SampleOceanBelow),
    Recipe(Recipe)
}

impl TerrainProcessCommand {

    fn into_process(self) -> TerrainProcess {
        match self {
            TerrainProcessCommand::SampleElevation(process) => TerrainProcess::SampleElevation(process),
            TerrainProcessCommand::SampleOceanMasked(process) => TerrainProcess::SampleOceanMasked(process),
            TerrainProcessCommand::SampleOceanBelow(process) => TerrainProcess::SampleOceanBelow(process),
            TerrainProcessCommand::Recipe(process) => TerrainProcess::Recipe(process)
        }

    }
}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct Terrain {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[command(subcommand)]
        process: TerrainProcessCommand,

        #[arg(long)]
        /// Instead of processing, display the serialized value for inclusion in a recipe file.
        serialize: bool

    }
}

impl Task for Terrain {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            let process = self.process.into_process();
            if self.serialize {
                println!("{}",process.to_json()?);
                Ok(())
            } else {
                process.process_terrain(target,&mut progress)
            }
    

        })?;

        target.save(&mut progress)


    }
}