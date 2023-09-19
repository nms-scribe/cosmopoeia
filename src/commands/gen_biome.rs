use clap::Args;
use clap::Subcommand;

use crate::commands::Task;
use crate::commands::TargetArg;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::algorithms::biomes::fill_biome_defaults;
use crate::algorithms::biomes::apply_biomes;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::algorithms::tiles::BiomeTheme;
use crate::algorithms::curves::curvify_layer_by_theme;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::BiomeMatrix;
use crate::commands::OverwriteBiomesArg;
use crate::commands::BezierScaleArg;

subcommand_def!{
    /// Creates default biome layer
    #[command(hide=true)]
    pub struct Data {

        #[clap(flatten)]
        pub target_arg: TargetArg,


        #[clap(flatten)]
        pub overwrite_biomes_arg: OverwriteBiomesArg,

    }
}

impl Task for Data {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(&self.overwrite_biomes_arg, target, progress)

        })?;

        target.save(progress)
    }
}

impl Data {

    fn run_with_parameters<Progress: ProgressObserver>(overwrite: &OverwriteBiomesArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {

        progress.announce("Filling biome defaults");

        fill_biome_defaults(target, overwrite, progress)
    }
}

subcommand_def!{
    /// Applies data from biomes layer to the tiles
    #[command(hide=true)]
    pub struct Apply {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for Apply {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        let biomes = target.biomes_layer()?.get_matrix(progress)?;

        target.with_transaction(|target| {

            Self::run(target, biomes, progress)

        })?;

        target.save(progress)


    }
}

impl Apply {

    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, biomes: BiomeMatrix, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying biomes to tiles");
    
        apply_biomes(target, biomes, progress)
    }
    
}


subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub struct Dissolve {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for Dissolve {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(target, progress)
        })?;

        target.save(progress)

    }
}

impl Dissolve {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating biome polygons");
    
        dissolve_tiles_by_theme::<_,BiomeTheme>(target, progress)
    }
    
}



subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub struct Curvify {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub bezier_scale_arg: BezierScaleArg,

    }
}

impl Task for Curvify {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(&self.bezier_scale_arg, target, progress)
        })?;

        target.save(progress)

    }
}

impl Curvify {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making biome polygons curvy");

        curvify_layer_by_theme::<_,BiomeTheme>(target, bezier_scale, progress)
    }
}


command_def!{
    #[command(disable_help_subcommand(true))]
    pub BiomeCommand {
        Data,
        Apply,
        Dissolve,
        Curvify
    }
}

#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[clap(flatten)]
    pub overwrite_biomes_arg: OverwriteBiomesArg,
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenBiome {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<BiomeCommand>

    }
}

impl Task for GenBiome {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(args) = self.default_args {
            let mut target = WorldMap::edit(args.target_arg.target)?;

            Self::run_default(&args.overwrite_biomes_arg, &args.bezier_scale_arg, &mut target, progress)
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenBiome {
    pub(crate) fn run_default<Progress: ProgressObserver>(ovewrite_biomes: &OverwriteBiomesArg, bezier_scale: &BezierScaleArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {            
            Data::run_with_parameters(ovewrite_biomes, target, progress)

        })?;
        let biomes = target.biomes_layer()?.get_matrix(progress)?;
        target.with_transaction(|target| {            
            Apply::run(target, biomes, progress)?;

            Dissolve::run_with_parameters(target, progress)?;

            Curvify::run_with_parameters(bezier_scale, target, progress)

        })?;

        target.save(progress)
    }
}