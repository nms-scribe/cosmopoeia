use clap::Args;
use clap::Subcommand;

use super::Task;
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

subcommand_def!{
    /// Generates temperature data
    #[command(hide=true)]
    pub struct Temperature {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        /// The rough temperature (in celsius) at the equator
        #[arg(long,default_value="25",allow_hyphen_values=true)]
        pub equator_temp: i8,

        /// The rough temperature (in celsius) at the poles
        #[arg(long,default_value="-15",allow_hyphen_values=true)]
        pub polar_temp: i8,

    }
}

impl Task for Temperature {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(self.equator_temp, self.polar_temp, target, progress)
        })?;

        target.save(progress)
        
    
    }
}

impl Temperature {
    fn run_with_parameters<Progress: ProgressObserver>(equator_temp: i8, polar_temp: i8, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating temperatures");

        generate_temperatures(target, equator_temp,polar_temp,progress)
    }
}



subcommand_def!{
    /// Generates wind data
    #[command(hide=true)]
    pub struct Winds {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="225")]
        /// Wind direction above latitude 60 N
        pub north_polar: u16,

        #[arg(long,default_value="45")]
        /// Wind direction from latitude 30 N to 60 N
        pub north_middle: u16,

        #[arg(long,default_value="225")]
        /// Wind direction from the equator to latitude 30 N
        pub north_tropical: u16,

        #[arg(long,default_value="315")]
        /// Wind direction from the equator to latitude 30 S
        pub south_tropical: u16,

        #[arg(long,default_value="135")]
        /// Wind direction from latitude 30 S to 60 S
        pub south_middle: u16,

        #[arg(long,default_value="315")]
        /// Wind direction below latitude 60 S
        pub south_polar: u16,

    }
}

impl Task for Winds {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        let winds = [
            self.north_polar as i32,
            self.north_middle as i32,
            self.north_tropical as i32,
            self.south_tropical as i32,
            self.south_middle as i32,
            self.south_polar as i32
        ];

        target.with_transaction(|target| {

            Self::run_with_parameters(winds, target, progress)

        })?;

        target.save(progress)
    
    }
}

impl Winds {
    fn run_with_parameters<Progress: ProgressObserver>(winds: [i32; 6], target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating winds");
    
        generate_winds(target, winds, progress)
    }
    
}


subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub struct Precipitation {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="100")]
        /// Amount of moisture on a scale of 0-500
        pub moisture: u16,


    }
}

impl Task for Precipitation {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(self.moisture, target, progress)

        })?;

        target.save(progress)
    
    }
}

impl Precipitation {
    fn run_with_parameters<Progress: ProgressObserver>(moisture: u16, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating precipitation");

        generate_precipitation(target, moisture, progress)
    }
}

command_def!{
    #[command(disable_help_subcommand(true))]
    pub ClimateCommand {
        Temperature,
        Winds,
        Precipitation
    }
}

#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    /// The rough temperature (in celsius) at the equator
    #[arg(long,default_value="25",allow_hyphen_values=true)]
    pub equator_temp: i8,

    /// The rough temperature (in celsius) at the poles
    #[arg(long,default_value="-15",allow_hyphen_values=true)]
    pub polar_temp: i8,

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

    #[arg(long,default_value="100")]
    /// Amount of moisture on a scale of 0-500
    pub moisture_factor: u16,

}


subcommand_def!{
    /// Generates temperature, wind, and precipitation data.
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenClimate {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<ClimateCommand>


    }
}

impl Task for GenClimate {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(command) = self.command {
            command.run(progress)
        } else if let Some(all) = self.default_args {
            let mut target = WorldMap::edit(all.target_arg.target)?;

            let winds = [
                all.north_polar_wind as i32,
                all.north_middle_wind as i32,
                all.north_tropical_wind as i32,
                all.south_tropical_wind as i32,
                all.south_middle_wind as i32,
                all.south_polar_wind as i32
            ];
    
            Self::run_default(all.equator_temp, all.polar_temp, winds, all.moisture_factor, &mut target, progress)

        } else {
            unreachable!("Command should have been called with one of the arguments")
        }
    


    }
}

impl GenClimate {
    pub(crate) fn run_default<Progress: ProgressObserver>(equator_temp: i8, polar_temp: i8, winds: [i32; 6], moisture_factor: u16, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {
    
            Temperature::run_with_parameters(equator_temp, polar_temp, target, progress)?;
    
            Winds::run_with_parameters(winds, target, progress)?;
    
            Precipitation::run_with_parameters(moisture_factor, target, progress)
    
        })?;
            
        target.save(progress)
    }
    
}




