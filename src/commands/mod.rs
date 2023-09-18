use std::path::PathBuf;

use clap::Subcommand;
use clap::Parser;
use clap::Args;
use serde::Serialize;
use serde::Deserialize;
use paste::paste;

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

#[derive(Args,Serialize,Deserialize)]
pub struct ElevationSourceArg {
    /// The path to the heightmap containing the elevation data
    pub source: PathBuf,

}

#[derive(Args,Serialize,Deserialize)]
pub struct OceanSourceArg {
    /// The path to the heightmap containing the ocean data
    pub source: PathBuf,

}

#[derive(Args)]
pub struct ElevationLimitsArg {
    #[arg(long,allow_negative_numbers=true,default_value="-11000")]
    /// minimum elevation for heightmap
    pub min_elevation: f64,

    #[arg(long,default_value="9000")]
    /// maximum elevation for heightmap
    pub max_elevation: f64,


}

#[derive(Args)]
pub struct TileCountArg {
    #[arg(long,default_value="10000")]
    /// The rough number of tiles to generate for the image
    pub tile_count: usize,

}


#[derive(Args)]
pub struct RandomSeedArg {
    #[arg(long)]
    /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
    pub seed: Option<u64>,
}

#[derive(Args)]
pub struct BezierScaleArg {
    #[arg(long,default_value="100")]
    /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
    pub bezier_scale: f64,

}

macro_rules! overwrite_arg {
    ($layer: ident) => {
        paste!{
            // TODO: Check documentation for this
            #[derive(Args)]
            pub struct [<Overwrite $layer:camel Arg>] {
                #[arg(long)]
                /// If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
                pub [<overwrite_ $layer:lower>]: bool,
            }
    
        }
                
    };
}


overwrite_arg!(tiles);
overwrite_arg!(coastline);
overwrite_arg!(ocean);
overwrite_arg!(lakes);
overwrite_arg!(rivers);
overwrite_arg!(biomes);
overwrite_arg!(cultures);
overwrite_arg!(towns);
overwrite_arg!(nations);
overwrite_arg!(subnations);


#[derive(Args)]
pub struct OverwriteAllArg {

    #[clap(flatten)]
    pub overwrite_tiles_arg: OverwriteTilesArg,

    #[clap(flatten)]
    pub overwrite_coastline_arg: OverwriteCoastlineArg,

    #[clap(flatten)]
    pub overwrite_ocean_arg: OverwriteOceanArg,

    #[clap(flatten)]
    pub overwrite_lakes_arg: OverwriteLakesArg,

    #[clap(flatten)]
    pub overwrite_rivers_arg: OverwriteRiversArg,

    #[clap(flatten)]
    pub overwrite_biomes_arg: OverwriteBiomesArg,

    #[clap(flatten)]
    pub overwrite_cultures_arg: OverwriteCulturesArg,

    #[clap(flatten)]
    pub overwrite_towns_arg: OverwriteTownsArg,

    #[clap(flatten)]
    pub overwrite_nations_arg: OverwriteNationsArg,

    #[clap(flatten)]
    pub overwrite_subnations_arg: OverwriteSubnationsArg,

    #[arg(long)]
    /// If true and any layer already exists in the file, it will be overwritten. This overrides all of the other 'overwrite_' switches to true.
    pub overwrite_all: bool,
    
}

impl OverwriteAllArg {

    fn overwrite_tiles(&self) -> OverwriteTilesArg {
        OverwriteTilesArg {
            overwrite_tiles: self.overwrite_tiles_arg.overwrite_tiles || self.overwrite_all
        }
    }

    fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    fn overwrite_ocean(&self) -> OverwriteOceanArg {
        OverwriteOceanArg {
            overwrite_ocean: self.overwrite_ocean_arg.overwrite_ocean || self.overwrite_all
        }
    }

    fn overwrite_lakes(&self) -> OverwriteLakesArg {
        OverwriteLakesArg {
            overwrite_lakes: self.overwrite_lakes_arg.overwrite_lakes || self.overwrite_all
        }
    }

    fn overwrite_rivers(&self) -> OverwriteRiversArg {
        OverwriteRiversArg {
            overwrite_rivers: self.overwrite_rivers_arg.overwrite_rivers || self.overwrite_all
        }
    }

    fn overwrite_biomes(&self) -> OverwriteBiomesArg {
        OverwriteBiomesArg {
            overwrite_biomes: self.overwrite_biomes_arg.overwrite_biomes || self.overwrite_all
        }
    }

    fn overwrite_cultures(&self) -> OverwriteCulturesArg {
        OverwriteCulturesArg {
            overwrite_cultures: self.overwrite_cultures_arg.overwrite_cultures || self.overwrite_all
        }
    }

    fn overwrite_towns(&self) -> OverwriteTownsArg {
        OverwriteTownsArg {
            overwrite_towns: self.overwrite_towns_arg.overwrite_towns || self.overwrite_all
        }
    }

    fn overwrite_nations(&self) -> OverwriteNationsArg {
        OverwriteNationsArg {
            overwrite_nations: self.overwrite_nations_arg.overwrite_nations || self.overwrite_all
        }
    }

    fn overwrite_subnations(&self) -> OverwriteSubnationsArg {
        OverwriteSubnationsArg {
            overwrite_subnations: self.overwrite_subnations_arg.overwrite_subnations || self.overwrite_all
        }
    }

}


#[derive(Args)]
pub struct OverwriteAllWaterArg {

    #[clap(flatten)]
    pub overwrite_coastline_arg: OverwriteCoastlineArg,

    #[clap(flatten)]
    pub overwrite_ocean_arg: OverwriteOceanArg,

    #[clap(flatten)]
    pub overwrite_lakes_arg: OverwriteLakesArg,

    #[clap(flatten)]
    pub overwrite_rivers_arg: OverwriteRiversArg,

    #[arg(long)]
    /// If true and any layer already exists in the file, it will be overwritten. This overrides all of the other 'overwrite_' switches to true.
    pub overwrite_all: bool,
    
}

impl OverwriteAllWaterArg {

    fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    fn overwrite_ocean(&self) -> OverwriteOceanArg {
        OverwriteOceanArg {
            overwrite_ocean: self.overwrite_ocean_arg.overwrite_ocean || self.overwrite_all
        }
    }

    fn overwrite_lakes(&self) -> OverwriteLakesArg {
        OverwriteLakesArg {
            overwrite_lakes: self.overwrite_lakes_arg.overwrite_lakes || self.overwrite_all
        }
    }

    fn overwrite_rivers(&self) -> OverwriteRiversArg {
        OverwriteRiversArg {
            overwrite_rivers: self.overwrite_rivers_arg.overwrite_rivers || self.overwrite_all
        }
    }


}


#[derive(Args)]
pub struct OverwriteAllOceanArg {

    #[clap(flatten)]
    pub overwrite_coastline_arg: OverwriteCoastlineArg,

    #[clap(flatten)]
    pub overwrite_ocean_arg: OverwriteOceanArg,

    #[arg(long)]
    /// If true and any layer already exists in the file, it will be overwritten. This overrides all of the other 'overwrite_' switches to true.
    pub overwrite_all: bool,
    
}

impl OverwriteAllOceanArg {


    fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    fn overwrite_ocean(&self) -> OverwriteOceanArg {
        OverwriteOceanArg {
            overwrite_ocean: self.overwrite_ocean_arg.overwrite_ocean || self.overwrite_all
        }
    }

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