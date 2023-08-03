
use clap::Subcommand;

use crate::errors::CommandError;

mod gdal_dev; // called gdal_dev to avoid ambiguity with external crate
mod dev;
mod convert_heightmap;
mod gen_climate;
mod gen_water;
mod gen_biome;

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
    convert_heightmap::ConvertHeightmap;
    convert_heightmap::ConvertHeightmapVoronoi;
    convert_heightmap::ConvertHeightmapSample;
    convert_heightmap::ConvertHeightmapOcean;
    convert_heightmap::ConvertHeightmapNeighbors;
    gen_climate::GenClimate;
    gen_climate::GenClimateTemperature;
    gen_climate::GenClimateWind;
    gen_climate::GenClimatePrecipitation;
    gen_water::GenWater;
    gen_water::GenWaterFlow;
    gen_water::GenWaterFill;
    gen_water::GenWaterRivers;
    gen_biome::GenBiome;
}


