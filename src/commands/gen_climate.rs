use clap::Args;
use clap::Subcommand;

use crate::commands::Task;
use crate::commands::TargetArg;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::algorithms::climate::generate_temperatures;
use crate::algorithms::climate::generate_winds;
use crate::algorithms::climate::generate_precipitation;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::commands::TemperatureRangeArg;
use crate::commands::WindsArg;
use crate::commands::PrecipitationArg;

subcommand_def!{
    /// Generates temperature data
    pub struct Temperature {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub temperatures_arg: TemperatureRangeArg,

    }
}

impl Task for Temperature {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.temperatures_arg, transaction, progress)
        })?;

        target.save(progress)
        
    
    }
}

impl Temperature {
    fn run_with_parameters<Progress: ProgressObserver>(temperatures: &TemperatureRangeArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating temperatures");

        generate_temperatures(target, temperatures, progress)
    }
}



subcommand_def!{
    /// Generates wind data
    pub struct Winds {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub winds_arg: WindsArg,

    }
}

impl Task for Winds {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.winds_arg, transaction, progress)

        })?;

        target.save(progress)
    
    }
}

impl Winds {
    fn run_with_parameters<Progress: ProgressObserver>(winds: &WindsArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating winds");
    
        generate_winds(target, winds, progress)
    }
    
}


subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub struct Precipitation {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub precipitation_arg: PrecipitationArg,


    }
}

impl Task for Precipitation {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.precipitation_arg, transaction, progress)

        })?;

        target.save(progress)
    
    }
}

impl Precipitation {
    fn run_with_parameters<Progress: ProgressObserver>(precipitation: &PrecipitationArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating precipitation");

        generate_precipitation(target, precipitation, progress)
    }
}



subcommand_def!{
    /// Generates all climate data
    pub struct All {

        #[clap(flatten)]
        pub target_arg: TargetArg,
    
        #[clap(flatten)]
        pub temperature_arg: TemperatureRangeArg,
    
        #[clap(flatten)]
        pub winds_arg: WindsArg,
    
        #[clap(flatten)]
        pub precipitation_arg: PrecipitationArg,

    }
}

impl Task for All {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.temperature_arg, &self.winds_arg, &self.precipitation_arg, transaction, progress)

        })?;

        target.save(progress)
    
    }
}

impl All {
    fn run_with_parameters<Progress: ProgressObserver>(temperatures: &TemperatureRangeArg, winds: &WindsArg, precipitation: &PrecipitationArg, transaction: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(), CommandError> {

        Temperature::run_with_parameters(temperatures, transaction, progress)?;

        Winds::run_with_parameters(winds, transaction, progress)?;

        Precipitation::run_with_parameters(precipitation, transaction, progress)

    }
}

command_def!{
    pub ClimateCommand {
        All,
        Temperature,
        Winds,
        Precipitation
    }
}

subcommand_def!{
    /// Generates temperature, wind, and precipitation data.
    pub struct GenClimate {

        #[command(subcommand)]
        pub command: ClimateCommand,

    }
}

impl Task for GenClimate {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        self.command.run(progress)

    }
}

impl GenClimate {
    pub(crate) fn run_default<Progress: ProgressObserver>(temperatures: &TemperatureRangeArg, winds: &WindsArg, precipitation: &PrecipitationArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {

            All::run_with_parameters(temperatures, winds, precipitation, transaction, progress)
    
        })?;
            
        target.save(progress)
    }
    
}




