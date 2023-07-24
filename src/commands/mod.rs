
use clap::Subcommand;

use crate::errors::CommandError;

mod gdal_dev; // called gdal_dev to avoid ambiguity with external crate
mod dev;
mod convert_heightmap;
mod gen_climate;

// NOTE: Further 'use' statements in the command macro below

pub(crate) trait Task {

    fn run(self) -> Result<(),CommandError>;

}

macro_rules! command {
    ($($command_mod: ident::$command_name: ident;)*) => {

        $(
            pub(crate) use $command_mod::$command_name;
        )*

        #[derive(Subcommand)]
        pub(crate) enum Command {
            $(
                $command_name($command_name)
            ),*
        }

        impl Task for Command {

            fn run(self) -> Result<(),CommandError> {
                match self {
                    $(Self::$command_name(a) => a.run()),*
                }
            }

        }
    };
}

// "Dev" commands are generally hidden, intended for testing during development. While they should be usable by users, they rarely are, and are hidden to keep the UI clean.

command!{
    gdal_dev::DevGdalVersion;
    gdal_dev::DevGdalInfo;
    gdal_dev::DevGdalDrivers;
    dev::DevPointsFromHeightmap;
    dev::DevPointsFromExtent;
    dev::DevTrianglesFromPoints;
    dev::DevVoronoiFromTriangles;
    dev::DevVoronoiNeighbors;
    dev::DevSampleHeightsToVoronoi;
    dev::DevSampleOceanToVoronoi;
    convert_heightmap::ConvertHeightmap;
    gen_climate::GenClimate;
    gen_climate::GenClimateTemperature;
    gen_climate::GenClimateWind;
    gen_climate::GenClimatePrecipitation;
}


