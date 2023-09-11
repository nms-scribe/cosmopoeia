use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::algorithms::water_flow::generate_water_flow;
use crate::algorithms::water_fill::generate_water_fill;
use crate::algorithms::rivers::generate_water_rivers;
use crate::algorithms::water_distance::generate_water_distance;
use crate::algorithms::grouping::calculate_grouping;
use crate::algorithms::tiles::calculate_coastline;
use crate::progress::ProgressObserver;


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct GenWaterCoastline {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy coastlines. The higher the number, the smoother the curves.
        bezier_scale: f64,

        #[arg(long)]
        /// If true and the coastline or oceans layers already exist in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[arg(long)]
        /// If true and the coastline layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_coastline: bool,

        #[arg(long)]
        /// If true and the oceans layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_ocean: bool,



    }
}

impl Task for GenWaterCoastline {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Creating coastline");

            calculate_coastline(target, self.bezier_scale, self.overwrite || self.overwrite_coastline, self.overwrite || self.overwrite_ocean, progress)
        })?;

        target.save(progress)


    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterFlow {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterFlow {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Calculating water flow");
            generate_water_flow(target, progress)
        })?;

        target.save(progress)

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

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        let (tile_map,tile_queue) = target.tiles_layer()?.get_index_and_queue_for_water_fill(progress)?;

        target.with_transaction(|target| {
            progress.announce("Filling lakes");
            generate_water_fill(target, tile_map, tile_queue, self.bezier_scale, self.buffer_scale, self.overwrite, progress)

        })?;

        target.save(progress)
    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenWaterRivers {

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

impl Task for GenWaterRivers {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Generating rivers");
            generate_water_rivers(target, self.bezier_scale, self.overwrite, progress)
        })?;

        target.save(progress)

    }
}

subcommand_def!{
    /// Calculates shortest distance to shoreline and some other water information for every tile, in count of tiles
    pub(crate) struct GenWaterDistance {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterDistance {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Calculating distance from shores");
            generate_water_distance(target, progress)
        })?;

        target.save(progress)

    }
}


subcommand_def!{
    /// Calculate grouping types for land tiles
    pub(crate) struct GenWaterGrouping {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenWaterGrouping {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Delineating land and water bodies");
            calculate_grouping(target, progress)
        })?;

        target.save(progress)

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

        #[arg(long)]
        /// If true and the coastline layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_coastline: bool,

        #[arg(long)]
        /// If true and the oceans layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_ocean: bool,



    }
}

impl Task for GenWater {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Creating coastline");

            calculate_coastline(target, self.bezier_scale, self.overwrite || self.overwrite_coastline, self.overwrite || self.overwrite_ocean, progress)?;

            progress.announce("Calculating water flow");
            let (tile_map,tile_queue) = generate_water_flow(target, progress)?;

            progress.announce("Filling lakes");
            generate_water_fill(target, tile_map, tile_queue, self.bezier_scale, self.buffer_scale, self.overwrite_lakes || self.overwrite, progress)?;

            progress.announce("Generating rivers");
            generate_water_rivers(target, self.bezier_scale, self.overwrite_rivers || self.overwrite, progress)?;

            progress.announce("Calculating distance from shore");
            generate_water_distance(target, progress)?;

            progress.announce("Delineating land and water bodies");
            calculate_grouping(target, progress)

        })?;

        target.save(progress)?;

        Ok(())

    }
}
