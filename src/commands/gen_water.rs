use clap::Args;
use clap::Subcommand;

use crate::commands::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::algorithms::water_flow::generate_water_flow;
use crate::algorithms::water_fill::generate_water_fill;
use crate::algorithms::water_flow::WaterFlowResult;
use crate::algorithms::rivers::generate_water_rivers;
use crate::algorithms::water_distance::generate_water_distance;
use crate::algorithms::grouping::calculate_grouping;
use crate::algorithms::tiles::calculate_coastline;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::commands::TargetArg;
use crate::commands::OverwriteCoastlineArg;
use crate::commands::OverwriteOceanArg;
use crate::commands::OverwriteLakesArg;
use crate::commands::OverwriteRiversArg;
use crate::commands::OverwriteAllOceanArg;
use crate::commands::OverwriteAllWaterArg;
use crate::commands::BezierScaleArg;
use crate::commands::LakeBufferScaleArg;


subcommand_def!{
    /// Calculates neighbors for tiles
    #[command(hide=true)]
    pub struct Coastline {

        #[clap(flatten)]
        pub target: TargetArg,

        #[clap(flatten)]
        pub bezier_scale: BezierScaleArg,

        #[clap(flatten)]
        pub overwrite_all_ocean: OverwriteAllOceanArg,

    }
}

impl Task for Coastline {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.bezier_scale, &self.overwrite_all_ocean.overwrite_coastline(), &self.overwrite_all_ocean.overwrite_ocean(), transaction, progress)
        })?;

        target.save(progress)


    }
}

impl Coastline {


    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, overwrite_coastline: &OverwriteCoastlineArg, overwrite_ocean: &OverwriteOceanArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating coastline");

        calculate_coastline(target, bezier_scale, overwrite_coastline, overwrite_ocean, progress)
    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub struct Flow {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for Flow {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        _ = target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Flow {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<WaterFlowResult,CommandError> {
        progress.announce("Calculating water flow");
        generate_water_flow(target, progress)
    }
    
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub struct Lakes {

        #[clap(flatten)]
        pub target: TargetArg,

        #[clap(flatten)]
        #[allow(clippy::struct_field_names,reason="I don't want to confuse this with other overwrite args.")]
        pub overwrite_lakes: OverwriteLakesArg,

        #[clap(flatten)]
        pub bezier_scale: BezierScaleArg,

        #[clap(flatten)]
        pub buffer_scale: LakeBufferScaleArg,



    }
}

impl Task for Lakes {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target.target)?;

        let water_flow_result = target.tiles_layer()?.get_index_and_queue_for_water_fill(progress)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(water_flow_result, &self.bezier_scale, &self.buffer_scale, &self.overwrite_lakes, transaction, progress)

        })?;

        target.save(progress)
    }
}

impl Lakes {
    fn run_with_parameters<Progress: ProgressObserver>(water_flow_result: WaterFlowResult, lake_bezier_scale: &BezierScaleArg, lake_buffer_scale: &LakeBufferScaleArg, overwrite_layer: &OverwriteLakesArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Filling lakes");
        generate_water_fill(target, water_flow_result, lake_bezier_scale, lake_buffer_scale, overwrite_layer, progress)
    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(hide=true)]
    pub struct Rivers {

        #[clap(flatten)]
        pub target: TargetArg,

        #[clap(flatten)]
        #[allow(clippy::struct_field_names,reason="I don't want to confuse this with other overwrite args.")]
        pub overwrite_rivers: OverwriteRiversArg,

        #[clap(flatten)]
        pub bezier_scale: BezierScaleArg,

    }
}

impl Task for Rivers {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(&self.bezier_scale, &self.overwrite_rivers, progress, transaction)
        })?;

        target.save(progress)

    }
}

impl Rivers {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, overwrite_layer: &OverwriteRiversArg, progress: &mut Progress, target: &mut WorldMapTransaction<'_>) -> Result<(), CommandError> {

        progress.announce("Generating rivers");
        generate_water_rivers(target, bezier_scale, overwrite_layer, progress)

    }
}


subcommand_def!{
    /// Calculates shortest distance to shoreline and some other water information for every tile, in count of tiles
    #[command(hide=true)]
    pub struct ShoreDistance {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for ShoreDistance {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl ShoreDistance {

    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Calculating distance from shores");
        generate_water_distance(target, progress)
    }

}

subcommand_def!{
    /// Calculate grouping types for land tiles
    #[command(hide=true)]
    pub struct Grouping {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for Grouping {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Grouping {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Delineating land and water bodies");
        calculate_grouping(target, progress)
    }
    
}


subcommand_def!{
    /// generates all water data
    pub struct All {

        #[clap(flatten)]
        pub target: TargetArg,
    
        #[clap(flatten)]
        pub bezier_scale: BezierScaleArg,
    
        #[clap(flatten)]
        pub buffer_scale: LakeBufferScaleArg,
    
        #[clap(flatten)]
        pub overwrite_all_water: OverwriteAllWaterArg,
    
    }
}

impl Task for All {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(&self.target.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(&self.bezier_scale,&self.buffer_scale,&self.overwrite_all_water.overwrite_coastline(),&self.overwrite_all_water.overwrite_ocean(),&self.overwrite_all_water.overwrite_lakes(),&self.overwrite_all_water.overwrite_rivers(),transaction,progress)
        })?;

        target.save(progress)

    }
}

impl All {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, lake_buffer_scale: &LakeBufferScaleArg, overwrite_coastline: &OverwriteCoastlineArg, overwrite_ocean: &OverwriteOceanArg, overwrite_lakes: &OverwriteLakesArg, overwrite_rivers: &OverwriteRiversArg, transaction: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(), CommandError> {
        Coastline::run_with_parameters(bezier_scale, overwrite_coastline, overwrite_ocean, transaction, progress)?;

        let water_flow_result = Flow::run_with_parameters(transaction, progress)?;

        Lakes::run_with_parameters(water_flow_result, bezier_scale, lake_buffer_scale, overwrite_lakes, transaction, progress)?;

        Rivers::run_with_parameters(bezier_scale, overwrite_rivers, progress, transaction)?;

        ShoreDistance::run_with_parameters(transaction, progress)?;

        Grouping::run_with_parameters(transaction, progress)
    
    }
    
}

command_def!{
    #[command(disable_help_subcommand(true))]
    pub WaterCommand {
        All,
        Coastline,
        Flow,
        Lakes,
        Rivers,
        ShoreDistance,
        Grouping
    }
}


subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenWater {

        #[command(subcommand)]
        pub command: WaterCommand


    }
}

impl Task for GenWater {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        self.command.run(progress)

    }
}

impl GenWater {
    pub(crate) fn run_default<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, lake_buffer_scale: &LakeBufferScaleArg, overwrite_coastline: &OverwriteCoastlineArg, overwrite_ocean: &OverwriteOceanArg, overwrite_lakes: &OverwriteLakesArg, overwrite_rivers: &OverwriteRiversArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {

            All::run_with_parameters(bezier_scale, lake_buffer_scale, overwrite_coastline, overwrite_ocean, overwrite_lakes, overwrite_rivers, transaction, progress)
        
        
        })?;
        
        target.save(progress)
    }
}