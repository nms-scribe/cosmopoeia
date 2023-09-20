use clap::Args;
use clap::Subcommand;
use rand::Rng;

use crate::commands::Task;
use crate::algorithms::nations::normalize_nations;
use crate::algorithms::nations::expand_nations;
use crate::algorithms::nations::generate_nations;
use crate::world_map::CultureForNations;
use crate::world_map::WorldMap;
use crate::utils::random_number_generator;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::algorithms::tiles::NationTheme;
use crate::algorithms::curves::curvify_layer_by_theme;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::EntityLookup;
use crate::world_map::CultureSchema;
use crate::algorithms::naming::NamerSet;
use crate::world_map::NamedEntity;
use crate::world_map::CultureWithNamer;
use crate::world_map::CultureWithType;
use crate::commands::TargetArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteNationsArg;
use crate::commands::BezierScaleArg;
use crate::commands::NamerArg;
use crate::commands::SizeVarianceArg;
use crate::commands::RiverThresholdArg;
use crate::commands::ExpansionFactorArg;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub namers_arg: NamerArg,

        #[clap(flatten)]
        pub size_variance_arg: SizeVarianceArg,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_nations_arg: OverwriteNationsArg,
        
    }
}

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(&self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;

        let mut loaded_namers = NamerSet::load_from(self.namers_arg, &mut random, progress)?;

        let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForNations>(progress)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, &self.size_variance_arg, &self.overwrite_nations_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, size_variance: &SizeVarianceArg, overwrite_nations: &OverwriteNationsArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating nations");
        generate_nations(target, random, culture_lookup, loaded_namers, size_variance, overwrite_nations, progress)
    }
    
}


subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Expand {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub river_threshold_arg: RiverThresholdArg,

        #[clap(flatten)]
        pub expansion_factor_arg: ExpansionFactorArg,

    }
}

impl Task for Expand {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;
        target.with_transaction(|transaction| {
            Self::run_with_parameters(&self.river_threshold_arg, &self.expansion_factor_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Expand {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: &RiverThresholdArg, limit_factor: &ExpansionFactorArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying nations to tiles");
    
        expand_nations(target, river_threshold, limit_factor, progress)
    }
    
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Normalize {

        #[clap(flatten)]
        pub target_arg: TargetArg,


    }
}

impl Task for Normalize {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Normalize {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Normalizing nation borders");
    
        normalize_nations(target, progress)
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

        target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Dissolve {

    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating nation polygons");
    
        dissolve_tiles_by_theme::<_,NationTheme>(target, progress)
    }
}



subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub struct Curvify {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub bezier_scale_arg: BezierScaleArg

    }
}

impl Task for Curvify {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(&self.bezier_scale_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Curvify {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making nation polygons curvy");
    
        curvify_layer_by_theme::<_,NationTheme>(target, bezier_scale, progress)
    }
    
}



command_def!{
    #[command(disable_help_subcommand(true))]
    pub NationCommand {
        Create,
        Expand,
        Normalize,
        Dissolve,
        Curvify
    }
}


#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[clap(flatten)]
    pub namer_arg: NamerArg,

    #[clap(flatten)]
    pub size_variance_arg: SizeVarianceArg,

    #[clap(flatten)]
    pub random_seed_arg: RandomSeedArg,

    #[clap(flatten)]
    pub river_threshold_arg: RiverThresholdArg,

    #[clap(flatten)]
    pub expansion_factor_arg: ExpansionFactorArg,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[clap(flatten)]
    pub overwrite_nations_arg: OverwriteNationsArg,


}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenNations {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<NationCommand>


    }
}

impl Task for GenNations {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {
            let mut random = random_number_generator(&default_args.random_seed_arg);

            let mut target = WorldMap::edit(default_args.target_arg.target)?;
    
            let mut loaded_namers = NamerSet::load_from(default_args.namer_arg, &mut random, progress)?;

            let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForNations>(progress)?;
    
            Self::run_default(&mut random, &culture_lookup, &mut loaded_namers, &default_args.size_variance_arg, &default_args.river_threshold_arg, &default_args.expansion_factor_arg, &default_args.bezier_scale_arg, &default_args.overwrite_nations_arg, &mut target, progress)

        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}


impl GenNations {

    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, size_variance: &SizeVarianceArg, river_threshold: &RiverThresholdArg, limit_factor: &ExpansionFactorArg, bezier_scale: &BezierScaleArg, overwrite_nations: &OverwriteNationsArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {
    
            Create::run_with_parameters(random, culture_lookup, loaded_namers, size_variance, overwrite_nations, transaction, progress)?;
    
            Expand::run_with_parameters(river_threshold, limit_factor, transaction, progress)?;
    
            Normalize::run_with_parameters(transaction, progress)?;
    
            Dissolve::run_with_parameters(transaction, progress)?;
    
            Curvify::run_with_parameters(bezier_scale, transaction, progress)
    
        })?;
    
        target.save(progress)
    }
    

}
