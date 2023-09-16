use std::path::PathBuf;

use clap::Subcommand;
use clap::Parser;
use clap::Args;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;


mod gdal_dev; // called gdal_dev to avoid ambiguity with external crate
mod dev;
mod create;
pub(crate) mod terrain;
mod gen_climate;
mod gen_water;
mod gen_biome;
mod gen_people;
mod gen_towns;
mod gen_nations;
mod gen_subnations;
mod big_bang;

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
use big_bang::BigBang;


pub(crate) trait Task {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError>;

}


#[macro_export]
/// Defines a runnable command-line command or subcommand enum
macro_rules! command_def {
    ($(#[$attr:meta])* $visibility: vis $struct_name: ident {$($command_name: ident),*}) => {

        #[derive(Subcommand)]
        $(#[$attr])*
        $visibility enum $struct_name {
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
    pub MainCommand {
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
        GenSubnations,
        BigBang
    }
}



#[macro_export]
/// The default command help template for subcommands
macro_rules! command_help_template {
    () => {
        "{about-section}\n{usage-heading}\n{tab}{usage}\n\n{all-args}\n\nVersion: {version}\nAuthor:  {author}"
    };
}

#[macro_export]
/// Defines a subcommand struct using standard attributes
macro_rules! subcommand_def {
    (#[doc = $about: literal] $(#[$attr:meta])* $visibility: vis struct $name: ident $body: tt) => {
        #[derive(Args)]
        #[command(author,help_template = crate::command_help_template!())] 
        #[doc = $about]
        $(#[$attr])*
        $visibility struct $name $body
                
    };
}

#[derive(Args)]
pub struct TargetArg {
    /// The path to the world map GeoPackage file
    pub target: PathBuf

}




#[derive(Parser)]
#[command(author, version, long_about = None, help_template = command_help_template!())]
#[command(propagate_version = true)]
/// N M Sheldon's Fantasy Mapping Tools
pub struct Cosmopoeia {

    #[command(subcommand)]
    pub command: MainCommand

}

impl Cosmopoeia {

    pub(crate) fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        self.command.run(progress)

    }

}