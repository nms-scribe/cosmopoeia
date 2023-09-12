use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::algorithms::water_flow::generate_water_flow;
use crate::algorithms::water_fill::generate_water_fill;
use crate::algorithms::rivers::generate_water_rivers;
use crate::algorithms::water_distance::generate_water_distance;
use crate::algorithms::grouping::calculate_grouping;
use crate::algorithms::tiles::calculate_coastline;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::TileForWaterFill;
use crate::world_map::TileSchema;
use crate::world_map::EntityIndex;


subcommand_def!{
    /// Calculates neighbors for tiles
    #[command(hide=true)]
    pub(crate) struct Coastline {

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

impl Task for Coastline {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            Self::run(self.bezier_scale, self.overwrite || self.overwrite_coastline, self.overwrite || self.overwrite_ocean, target, progress)
        })?;

        target.save(progress)


    }
}

impl Coastline {


    fn run<Progress: ProgressObserver>(bezier_scale: f64, overwrite_coastline: bool, overwrite_ocean: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating coastline");

        calculate_coastline(target, bezier_scale, overwrite_coastline, overwrite_ocean, progress)
    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub(crate) struct Flow {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for Flow {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run(target, progress)
        })?;

        target.save(progress)

    }
}

impl Flow {
    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(EntityIndex<TileSchema, TileForWaterFill>, Vec<(u64, f64)>), CommandError> {
        progress.announce("Calculating water flow");
        generate_water_flow(target, progress)
    }
    
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub(crate) struct Lakes {

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

impl Task for Lakes {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        let (tile_map,tile_queue) = target.tiles_layer()?.get_index_and_queue_for_water_fill(progress)?;

        target.with_transaction(|target| {
            Self::run(tile_map, tile_queue, self.bezier_scale, self.buffer_scale, self.overwrite, target, progress)

        })?;

        target.save(progress)
    }
}

impl Lakes {
    fn run<Progress: ProgressObserver>(tile_map: EntityIndex<TileSchema, TileForWaterFill>, tile_queue: Vec<(u64, f64)>, lake_bezier_scale: f64, lake_buffer_scale: f64, overwrite_layer: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Filling lakes");
        generate_water_fill(target, tile_map, tile_queue, lake_bezier_scale, lake_buffer_scale, overwrite_layer, progress)
    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub(crate) struct Rivers {

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

impl Task for Rivers {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run(self.bezier_scale, self.overwrite, progress, target)
        })?;

        target.save(progress)

    }
}

impl Rivers {
    fn run<Progress: ProgressObserver>(bezier_scale: f64, overwrite_layer: bool, progress: &mut Progress, target: &mut WorldMapTransaction<'_>) -> Result<(), CommandError> {

        progress.announce("Generating rivers");
        generate_water_rivers(target, bezier_scale, overwrite_layer, progress)

    }
}


subcommand_def!{
    /// Calculates shortest distance to shoreline and some other water information for every tile, in count of tiles
    #[command(hide=true)]
    pub(crate) struct ShoreDistance {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for ShoreDistance {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run(target, progress)
        })?;

        target.save(progress)

    }
}

impl ShoreDistance {

    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Calculating distance from shores");
        generate_water_distance(target, progress)
    }

}

subcommand_def!{
    /// Calculate grouping types for land tiles
    #[command(hide=true)]
    pub(crate) struct Grouping {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for Grouping {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run(target, progress)
        })?;

        target.save(progress)

    }
}

impl Grouping {
    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Delineating land and water bodies");
        calculate_grouping(target, progress)
    }
    
}


command_def!{
    WaterCommand {
        Coastline,
        Flow,
        Lakes,
        Rivers,
        ShoreDistance,
        Grouping
    }
}


#[derive(Args)]
struct DefaultArgs {

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

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(args_conflicts_with_subcommands = true)]
    pub(crate) struct GenWater {

        #[clap(flatten)]
        default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        command: Option<WaterCommand>


    }
}

impl Task for GenWater {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(args) = self.default_args {
            let mut target = WorldMap::edit(args.target)?;

            run(args.bezier_scale, 
                args.buffer_scale, 
                args.overwrite || args.overwrite_coastline, 
                args.overwrite || args.overwrite_ocean, 
                args.overwrite_lakes || args.overwrite, 
                args.overwrite_rivers || args.overwrite, 
                &mut target, 
                progress)
    
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

fn run<Progress: ProgressObserver>(bezier_scale: f64, lake_buffer_scale: f64, overwrite_coastline: bool, overwrite_ocean: bool, overwrite_lakes: bool, overwrite_rivers: bool, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
    target.with_transaction(|target| {
    
        Coastline::run(bezier_scale, overwrite_coastline, overwrite_ocean, target, progress)?;

        let (tile_map,tile_queue) = Flow::run(target, progress)?;

        Lakes::run(tile_map, tile_queue, bezier_scale, lake_buffer_scale, overwrite_lakes, target, progress)?;

        Rivers::run(bezier_scale, overwrite_rivers, progress, target)?;

        ShoreDistance::run(target, progress)?;

        Grouping::run(target, progress)
    
    })?;
    
    target.save(progress)
}
