
use clap::Subcommand;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;

mod gdal_dev; // called gdal_dev to avoid ambiguity with external crate
mod dev;
mod create;
mod terrain;
mod gen_climate;
mod gen_water;
mod gen_biome;
mod gen_people;
mod gen_towns;
mod gen_nations;
mod gen_subnations;

use gdal_dev::Gdal;
use dev::Dev;
use create::Create;
use create::CreateCalcNeighbors;
use create::CreateTiles;
use terrain::Terrain;
use gen_climate::GenClimate;
use gen_water::GenWater;
use gen_biome::GenBiome;
use gen_people::GenPeople;
use gen_towns::GenTowns;
use gen_nations::GenNations;
use gen_subnations::GenSubnations;


pub(crate) trait Task {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError>;

}

#[macro_export]
macro_rules! command_def {
    ($struct_name: ident {$($command_name: ident),*}) => {

        #[derive(Subcommand)]
        pub(crate) enum $struct_name {
            $(
                $command_name($command_name)
            ),*
        }

        impl Task for $struct_name {

            fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {
                match self {
                    $(Self::$command_name(a) => a.run(progress)),*
                }
            }

        }
    };
}

// "Dev" commands are generally hidden, intended for testing during development. While they should be usable by users, they rarely are, and are hidden to keep the UI clean.

command_def!{
    MainCommand {
        Gdal,
        Dev,
        Create,
        CreateCalcNeighbors,
        CreateTiles,
        Terrain,
        GenClimate,
        GenWater,
        GenBiome,
        GenPeople,
        GenTowns,
        GenNations,
        GenSubnations
    }
}

