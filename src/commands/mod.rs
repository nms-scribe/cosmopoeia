use std::path::PathBuf;
use core::ops::Range;

use clap::Subcommand;
use clap::Parser;
use clap::Args;
use serde::Serialize;
use serde::Deserialize;
use paste::paste;
use ordered_float::OrderedFloat;
use rangemap::RangeMap;


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
use crate::utils::ArgRange;


pub(crate) trait Task {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError>;

}


#[macro_export]
/// Defines a runnable command-line command or subcommand enum
macro_rules! command_def {
    ($(#[$attr:meta])* $visibility: vis $struct_name: ident {$($(#[$command_attr:meta])* $command_name: ident),*}) => {

        #[derive(Subcommand)]
        $(#[$attr])*
        $visibility enum $struct_name {
            $(
                $(#[$command_attr])*
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
    /// Primary cosmopoeia commands
    pub MainCommand {
        /// Commands used for examining link with gdal library
        Gdal,
        /// Support commands used mostly during development
        Dev,
        /// Creates a world map.
        Create,
        /// Support command for calculating tile neighbors after creation
        CreateCalcNeighbors,
        /// Support command to create tiles without calculating neighbors or processing elevations
        CreateTiles,
        /// Runs a terrain process on the world to manipulate elevations or ocean status
        Terrain,
        /// Generates climate data for a world
        GenClimate,
        /// Generates water features for a world
        GenWater,
        /// Generates biomes for a world
        GenBiome,
        /// Generates populations and cultures for a world
        GenPeople,
        /// Generates towns, cities and other urban centers for a world
        GenTowns,
        /// Generates nations for a world
        GenNations,
        /// Generates subnations (provinces and other administrative divisions) for a world
        GenSubnations,
        /// Creates a world map, generates natural features, and populates it with nations and subnations
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
        #[command(author,help_template = $crate::command_help_template!())] 
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

#[derive(Args)]
pub struct TemperatureRangeArg {
        /// The rough temperature (in celsius) at the equator
        #[arg(long,default_value="25",allow_hyphen_values=true)]
        pub equator_temp: i8,

        /// The rough temperature (in celsius) at the poles
        #[arg(long,default_value="-15",allow_hyphen_values=true)]
        pub polar_temp: i8,

}

fn parse_wind_range(value: &str) -> Result<(Range<OrderedFloat<f64>>, u16), &'static str> {
    const HELP_MESSAGE: &str = "Format for wind range is `S..N:Direction`, where south and north are south and north (not inclusive) latitude and direction is clockwise degrees from north.";
    // I already parse out a range for ArgRange. However, I only allow exclusive ranges here, since that's
    // how I map them.
    if let Some((range,direction)) = value.split_once(':') {
        let range = range.parse().map_err(|_| HELP_MESSAGE)?;
        let direction = direction.parse().map_err(|_| HELP_MESSAGE)?;
        let range = match range {
            ArgRange::Exclusive(min, max) => OrderedFloat(min)..OrderedFloat(max),
            ArgRange::Inclusive(_,_) | ArgRange::Single(_) => return Err(HELP_MESSAGE)
        };
        Ok((range,direction))
    
    } else {
        Err(HELP_MESSAGE)
    }
}

#[derive(Args)]
pub struct WindsArg {
    
    #[arg(long,default_value="225")]
    /// Wind direction above latitude 60 N
    pub north_polar_wind: u16,

    #[arg(long,default_value="45")]
    /// Wind direction from latitude 30 N to 60 N
    pub north_middle_wind: u16,

    #[arg(long,default_value="225")]
    /// Wind direction from the equator to latitude 30 N
    pub north_tropical_wind: u16,

    #[arg(long,default_value="315")]
    /// Wind direction from the equator to latitude 30 S
    pub south_tropical_wind: u16,

    #[arg(long,default_value="135")]
    /// Wind direction from latitude 30 S to 60 S
    pub south_middle_wind: u16,

    #[arg(long,default_value="315")]
    /// Wind direction below latitude 60 S
    pub south_polar_wind: u16,

    #[arg(long,allow_hyphen_values=true,value_parser(parse_wind_range))]
    /// Specify a range of latitudes and a wind direction (S lat..N lat:Direction), later mappings will override earlier.
    pub wind_range: Vec<(Range<OrderedFloat<f64>>, u16)>


}

impl WindsArg {

    pub(crate) fn to_range_map(&self) -> RangeMap<OrderedFloat<f64>, u16> {
        let mut result = RangeMap::new();
        result.insert(OrderedFloat(-90.0)..OrderedFloat(-60.0),self.south_polar_wind);
        result.insert(OrderedFloat(-60.0)..OrderedFloat(-30.0),self.south_middle_wind);
        result.insert(OrderedFloat(-30.0)..OrderedFloat(0.0),self.south_tropical_wind);
        result.insert(OrderedFloat(0.0)..OrderedFloat(30.0),self.north_tropical_wind);
        result.insert(OrderedFloat(30.0)..OrderedFloat(60.0),self.north_middle_wind);
        // note that the last one is set at 90.1 since the range map is not inclusive
        result.insert(OrderedFloat(60.0)..OrderedFloat(90.1),self.north_polar_wind);

        for range in &self.wind_range {
            result.insert(range.0.clone(),range.1)
        }
        result

    }
}

#[derive(Args)]
pub struct PrecipitationArg {

    #[arg(long,default_value="1")]
    /// Amount of global moisture on a scale of 0-5
    pub precipitation_factor: f64,

}

#[derive(Args)]
pub struct NamerArg {

    #[arg(long,required=true)]
    /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
    pub namers: Vec<PathBuf>,

    #[arg(long)]
    /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
    pub default_namer: Option<String>,


}


fn validate_size_variance(value: &str) -> Result<f64,String> {
    // FUTURE: Maybe someday clap will support float ranges
    const LOW: f64 = 0.0;
    const HIGH: f64 = 10.0;

    let value = value.parse::<f64>().map_err(|_| format!("Argument '{value}' must be a float."))?;
    if (LOW..=HIGH).contains(&value) {
        Ok(value)
    } else {
        Err(format!("Argument must be between {LOW} and {HIGH}."))
    }
}

#[derive(Args)]
pub struct SizeVarianceArg {

    #[arg(long,default_value("1"),value_parser(validate_size_variance))]
    /// A number, clamped to 0-10, which controls how much cultures can vary in size
    pub size_variance: f64,


}

#[derive(Args)]
pub struct RiverThresholdArg {

    #[arg(long,default_value="10")]
    /// A waterflow threshold above which the tile will count as a river
    pub river_threshold: f64,


}

#[derive(Args)]
pub struct ExpansionFactorArg {

    #[arg(long,default_value("1"))]
    /// A number, usually ranging from 0.1 to 2.0, which limits how far cultures and nations will expand. The higher the number, the fewer neutral lands.
    pub expansion_factor: f64

}

#[derive(Args)]
pub struct CulturesGenArg {

    #[arg(long,required(true))] 
    /// Files to load culture sets from, more than one may be specified to load multiple culture sets.
    pub cultures: Vec<PathBuf>,

    #[arg(long,default_value("10"))]
    /// The number of cultures to generate
    pub culture_count: usize,


}

#[derive(Args)]
pub struct SubnationPercentArg {

    #[arg(long,default_value("20"))]
    /// The percent of towns in each nation to use for subnations
    pub subnation_percentage: f64,


}

#[derive(Args)]
pub struct TownCountsArg {
    #[arg(long,default_value="20")]
    /// The number of national capitals to create
    pub capital_count: usize,

    #[arg(long)]
    /// The number of non-capital towns to create
    pub town_count: Option<usize>,


}


#[derive(Args)]
pub struct LakeBufferScaleArg {
    #[arg(long,default_value="2")]
    /// This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes.
    pub lake_buffer_scale: f64


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

    const fn overwrite_tiles(&self) -> OverwriteTilesArg {
        OverwriteTilesArg {
            overwrite_tiles: self.overwrite_tiles_arg.overwrite_tiles || self.overwrite_all
        }
    }

    const fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    const fn overwrite_ocean(&self) -> OverwriteOceanArg {
        OverwriteOceanArg {
            overwrite_ocean: self.overwrite_ocean_arg.overwrite_ocean || self.overwrite_all
        }
    }

    const fn overwrite_lakes(&self) -> OverwriteLakesArg {
        OverwriteLakesArg {
            overwrite_lakes: self.overwrite_lakes_arg.overwrite_lakes || self.overwrite_all
        }
    }

    const fn overwrite_rivers(&self) -> OverwriteRiversArg {
        OverwriteRiversArg {
            overwrite_rivers: self.overwrite_rivers_arg.overwrite_rivers || self.overwrite_all
        }
    }

    const fn overwrite_biomes(&self) -> OverwriteBiomesArg {
        OverwriteBiomesArg {
            overwrite_biomes: self.overwrite_biomes_arg.overwrite_biomes || self.overwrite_all
        }
    }

    const fn overwrite_cultures(&self) -> OverwriteCulturesArg {
        OverwriteCulturesArg {
            overwrite_cultures: self.overwrite_cultures_arg.overwrite_cultures || self.overwrite_all
        }
    }

    const fn overwrite_towns(&self) -> OverwriteTownsArg {
        OverwriteTownsArg {
            overwrite_towns: self.overwrite_towns_arg.overwrite_towns || self.overwrite_all
        }
    }

    const fn overwrite_nations(&self) -> OverwriteNationsArg {
        OverwriteNationsArg {
            overwrite_nations: self.overwrite_nations_arg.overwrite_nations || self.overwrite_all
        }
    }

    const fn overwrite_subnations(&self) -> OverwriteSubnationsArg {
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

    const fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    const fn overwrite_ocean(&self) -> OverwriteOceanArg {
        OverwriteOceanArg {
            overwrite_ocean: self.overwrite_ocean_arg.overwrite_ocean || self.overwrite_all
        }
    }

    const fn overwrite_lakes(&self) -> OverwriteLakesArg {
        OverwriteLakesArg {
            overwrite_lakes: self.overwrite_lakes_arg.overwrite_lakes || self.overwrite_all
        }
    }

    const fn overwrite_rivers(&self) -> OverwriteRiversArg {
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


    const fn overwrite_coastline(&self) -> OverwriteCoastlineArg {
        OverwriteCoastlineArg {
            overwrite_coastline: self.overwrite_coastline_arg.overwrite_coastline || self.overwrite_all
        }
    }

    const fn overwrite_ocean(&self) -> OverwriteOceanArg {
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