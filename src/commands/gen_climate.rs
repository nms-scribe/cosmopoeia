use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;


subcommand_def!{
    /// Generates temperature values for climates
    #[command(hide=true)]
    pub(crate) struct GenClimateTemperature {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// The rough temperature (in celsius) at the equator
        #[arg(long,default_value="25",allow_hyphen_values=true)]
        equator_temp: i8,

        /// The rough temperature (in celsius) at the poles
        #[arg(long,default_value="-15",allow_hyphen_values=true)]
        polar_temp: i8,

    }
}

impl Task for GenClimateTemperature {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;
        
        target.generate_temperatures(self.equator_temp,self.polar_temp,&mut progress)
    
    }
}



subcommand_def!{
    /// Generates wind directions for climate
    #[command(hide=true)]
    pub(crate) struct GenClimateWind {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="225")]
        // Wind direction above latitude 60 N
        north_polar: u16,

        #[arg(long,default_value="45")]
        // Wind direction from latitude 30 N to 60 N
        north_middle: u16,

        #[arg(long,default_value="225")]
        // Wind direction from the equator to latitude 30 N
        north_tropical: u16,

        #[arg(long,default_value="315")]
        // Wind direction from the equator to latitude 30 S
        south_tropical: u16,

        #[arg(long,default_value="135")]
        // Wind direction from latitude 30 S to 60 S
        south_middle: u16,

        #[arg(long,default_value="315")]
        // Wind direction below latitude 60 S
        south_polar: u16,

    }
}

impl Task for GenClimateWind {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let winds = [
            self.north_polar as f64,
            self.north_middle as f64,
            self.north_tropical as f64,
            self.south_tropical as f64,
            self.south_middle as f64,
            self.south_polar as f64
        ];

        target.generate_winds(winds,&mut progress)
    
    }
}


subcommand_def!{
    /// Generates precipitation values for climates
    #[command(hide=true)]
    pub(crate) struct GenClimatePrecipitation {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// Amount of moisture on a scale of 0-500
        moisture: u16,


    }
}

impl Task for GenClimatePrecipitation {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.generate_precipitation(self.moisture,&mut progress)
    
    }
}



subcommand_def!{
    /// Generates temperature, wind, and precipitation data.
    pub(crate) struct GenClimate {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// The rough temperature (in celsius) at the equator
        #[arg(long,default_value="25",allow_hyphen_values=true)]
        equator_temp: i8,

        /// The rough temperature (in celsius) at the poles
        #[arg(long,default_value="-15",allow_hyphen_values=true)]
        polar_temp: i8,

        #[arg(long,default_value="225")]
        // Wind direction above latitude 60 N
        north_polar_wind: u16,

        #[arg(long,default_value="45")]
        // Wind direction from latitude 30 N to 60 N
        north_middle_wind: u16,

        #[arg(long,default_value="225")]
        // Wind direction from the equator to latitude 30 N
        north_tropical_wind: u16,

        #[arg(long,default_value="315")]
        // Wind direction from the equator to latitude 30 S
        south_tropical_wind: u16,

        #[arg(long,default_value="135")]
        // Wind direction from latitude 30 S to 60 S
        south_middle_wind: u16,

        #[arg(long,default_value="315")]
        // Wind direction below latitude 60 S
        south_polar_wind: u16,

        #[arg(long,default_value="100")]
        /// Amount of moisture on a scale of 0-500
        moisture_factor: u16,


    }
}

impl Task for GenClimate {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;
        
        target.generate_temperatures(self.equator_temp,self.polar_temp,&mut progress)?;
    
        let winds = [
            self.north_polar_wind as f64,
            self.north_middle_wind as f64,
            self.north_tropical_wind as f64,
            self.south_tropical_wind as f64,
            self.south_middle_wind as f64,
            self.south_polar_wind as f64
        ];

        target.generate_winds(winds,&mut progress)?;

        target.generate_precipitation(self.moisture_factor,&mut progress)
    


    }
}




