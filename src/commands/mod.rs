
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
use gen_climate::GenClimateTemperature;
use gen_climate::GenClimateWind;
use gen_climate::GenClimatePrecipitation;
use gen_water::GenWater;
use gen_water::GenWaterCoastline;
use gen_water::GenWaterFlow;
use gen_water::GenWaterFill;
use gen_water::GenWaterRivers;
use gen_water::GenWaterDistance;
use gen_water::GenWaterGrouping;
use gen_biome::GenBiome;
use gen_biome::GenBiomeData;
use gen_biome::GenBiomeApply;
use gen_biome::GenBiomeDissolve;
use gen_biome::GenBiomeCurvify;
use gen_people::GenPeople;
use gen_people::GenPeoplePopulation;
use gen_people::GenPeopleCultures;
use gen_people::GenPeopleCulturesExpand;
use gen_people::GenPeopleCulturesDissolve;
use gen_people::GenPeopleCulturesCurvify;
use gen_towns::GenTowns;
use gen_towns::GenTownsCreate;
use gen_towns::GenTownsPopulate;
use gen_nations::GenNations;
use gen_nations::GenNationsCreate;
use gen_nations::GenNationsExpand;
use gen_nations::GenNationsNormalize;
use gen_nations::GenNationsDissolve;
use gen_nations::GenNationsCurvify;
use gen_subnations::GenSubnations;
use gen_subnations::GenSubnationsCreate;
use gen_subnations::GenSubnationsExpand;
use gen_subnations::GenSubnationsFillEmpty;
use gen_subnations::GenSubnationsNormalize;
use gen_subnations::GenSubnationsDissolve;
use gen_subnations::GenSubnationsCurvify;


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
        GenClimateTemperature,
        GenClimateWind,
        GenClimatePrecipitation,
        GenWater,
        GenWaterCoastline,
        GenWaterFlow,
        GenWaterFill,
        GenWaterRivers,
        GenWaterDistance,
        GenWaterGrouping,
        GenBiome,
        GenBiomeData,
        GenBiomeApply,
        GenBiomeDissolve,
        GenBiomeCurvify,
        GenPeople,
        GenPeoplePopulation,
        GenPeopleCultures,
        GenPeopleCulturesExpand,
        GenPeopleCulturesDissolve,
        GenPeopleCulturesCurvify,
        GenTowns,
        GenTownsCreate,
        GenTownsPopulate,
        GenNations,
        GenNationsCreate,
        GenNationsExpand,
        GenNationsNormalize,
        GenNationsDissolve,
        GenNationsCurvify,
        GenSubnations,
        GenSubnationsCreate,
        GenSubnationsExpand,
        GenSubnationsFillEmpty,
        GenSubnationsNormalize,
        GenSubnationsDissolve,
        GenSubnationsCurvify
    }
}

