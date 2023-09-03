use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;

use super::Task;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::terrain::TerrainSettings;
use crate::algorithms::terrain::SampleElevation;
use crate::algorithms::terrain::SampleOceanBelow;
use crate::algorithms::terrain::SampleOceanMasked;
use crate::algorithms::terrain::TerrainProcess;


#[derive(Subcommand)]
enum TerrainProcessCommand {
    SampleElevation(SampleElevation),
    SampleOceanMasked(SampleOceanMasked),
    SampleOceanBelow(SampleOceanBelow),
}

impl TerrainProcessCommand {

    fn into_process(self) -> TerrainProcess {
        match self {
            TerrainProcessCommand::SampleElevation(process) => TerrainProcess::SampleElevation(process),
            TerrainProcessCommand::SampleOceanMasked(process) => TerrainProcess::SampleOceanMasked(process),
            TerrainProcessCommand::SampleOceanBelow(process) => TerrainProcess::SampleOceanBelow(process),
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

        #[arg(long,allow_negative_numbers=true)]
        /// minimum elevation for heightmap
        min_elevation: f64,

        #[arg(long)]
        /// maximum elevation for heightmap
        max_elevation: f64

    }
}

impl Task for Terrain {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            let settings = TerrainSettings::new(self.min_elevation,self.max_elevation)?;
            let process = self.process.into_process();
            process.process_terrain(settings,target,&mut progress)
    

        })?;

        target.save(&mut progress)


    }
}