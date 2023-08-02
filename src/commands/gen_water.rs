use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFlow {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterFlow {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.generate_water_flow(&mut progress)?;

        Ok(())


    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFill {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the lakes layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to follow lake shoreline curves. The higher the number, the smoother the curves.
        bezier_scale: f64,

        #[arg(long,default_value="2")]
        /// This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes.
        buffer_scale: f64



    }
}

impl Task for GenWaterFill {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let (tile_map,tile_queue) = target.get_tile_map_and_queue_for_water_fill(&mut progress)?;

        let lakes = target.generate_water_fill(tile_map,tile_queue,self.bezier_scale,self.buffer_scale,&mut progress)?;

        target.load_lakes(lakes,self.overwrite,&mut progress)?;

        Ok(())

    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterConnectRivers {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the river_segments layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to follow river curves. The higher the number, the smoother the curves.
        bezier_scale: f64

    }
}

impl Task for GenWaterConnectRivers {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let segments = target.generate_water_connect_rivers(self.bezier_scale,&mut progress)?;

        target.load_rivers(segments,self.overwrite,&mut progress)?;

        Ok(())

    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWater {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to follow river curves and make curvy lakes. The higher the number, the smoother the curves.
        bezier_scale: f64,

        #[arg(long,default_value="2")]
        /// This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes.
        buffer_scale: f64,

        #[arg(long)]
        /// If true and the rivers or lakes layers already exist in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[arg(long)]
        /// If true and the rivers or lakes layers already exist in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_rivers: bool,

        #[arg(long)]
        /// If true and the rivers or lakes layers already exist in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_lakes: bool,




    }
}

impl Task for GenWater {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let (tile_map,tile_queue) = target.generate_water_flow(&mut progress)?;

        let lakes = target.generate_water_fill(tile_map,tile_queue,self.bezier_scale,self.buffer_scale,&mut progress)?;

        target.load_lakes(lakes,self.overwrite_lakes || self.overwrite,&mut progress)?;

        let segments = target.generate_water_connect_rivers(self.bezier_scale,&mut progress)?;

        target.load_rivers(segments,self.overwrite_rivers || self.overwrite,&mut progress)?;

        Ok(())

    }
}
