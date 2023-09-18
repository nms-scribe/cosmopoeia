use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::algorithms::naming::NamerSetSource;
use crate::world_map::CultureForNations;
use crate::world_map::CultureSchema;
use crate::utils::random_number_generator;
use crate::algorithms::subnations::generate_subnations;
use crate::algorithms::subnations::expand_subnations;
use crate::algorithms::subnations::fill_empty_subnations;
use crate::algorithms::subnations::normalize_subnations;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::algorithms::tiles::SubnationTheme;
use crate::algorithms::curves::curvify_layer_by_theme;
use crate::world_map::WorldMapTransaction;
use crate::world_map::EntityLookup;
use crate::algorithms::naming::NamerSet;
use crate::world_map::NamedEntity;
use crate::world_map::CultureWithNamer;
use crate::world_map::CultureWithType;
use crate::commands::TargetArg;
use super::RandomSeedArg;
use super::OverwriteSubnationsArg;
use super::BezierScaleArg;



subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,required=true)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        pub namers: Vec<PathBuf>,

        #[arg(long)]
        /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
        pub default_namer: Option<String>,

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        pub subnation_percentage: f64,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_subnations_arg: OverwriteSubnationsArg,
    
    
    }
}

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;

        let namers = NamerSetSource::from_files(self.namers)?;

        let mut loaded_namers = NamerSet::load_from(namers, self.default_namer, &mut random, progress)?;

        let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForNations>(progress)?;


        target.with_transaction(|target| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, self.subnation_percentage, self.overwrite_subnations_arg, target, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, subnation_percentage: f64, overwrite_subnations: OverwriteSubnationsArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating subnations");
                
        generate_subnations(target, random, culture_lookup, loaded_namers, subnation_percentage, overwrite_subnations, progress)
    }
    
}



subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Expand {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        pub subnation_percentage: f64,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,


    }
}

impl Task for Expand {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;
        

        target.with_transaction(|target| {
            Self::run_with_parameters(&mut random, self.subnation_percentage, target, progress)
        })?;

        target.save(progress)

    }
}

impl Expand {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver>(random: &mut Random, subnation_percentage: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying subnations to tiles");
    
        expand_subnations(target, random, subnation_percentage, progress)
    }
    
}




subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct FillEmpty {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,required=true)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        pub namers: Vec<PathBuf>,

        #[arg(long)]
        /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
        pub default_namer: Option<String>,

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        pub subnation_percentage: f64,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

    }
}

impl Task for FillEmpty {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;
        
        let namers = NamerSetSource::from_files(self.namers)?;

        let mut loaded_namers = NamerSet::load_from(namers, self.default_namer, &mut random, progress)?;

        let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForNations>(progress)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, self.subnation_percentage, target, progress)
        })?;

        target.save(progress)

    }
}

impl FillEmpty {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, subnation_percentage: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating new subnations to fill rest of nations");
    
        fill_empty_subnations(target, random, culture_lookup, loaded_namers, subnation_percentage, progress)
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

        target.with_transaction(|target| {
            Self::run_with_parameters(target, progress)
        })?;

        target.save(progress)

    }
}

impl Normalize {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Normalizing subnation borders");
    
        normalize_subnations(target, progress)
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
        progress.announce("Creating subnation polygons");

        dissolve_tiles_by_theme::<_,SubnationTheme>(target, progress)
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

        let bezier_scale = self.bezier_scale_arg;
        target.with_transaction(|target| {
            Self::run_with_parameters(&bezier_scale, target, progress)
        })?;

        target.save(progress)

    }
}

impl Curvify {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making subnation polygons curvy");
    
        // FUTURE: Technically, subnations have to follow the curves of their owning nations as priority over their own. 
        // Right now, it doesn't seem to make a big difference if you have the nation borders thick enough. But it
        // may become important later.
        curvify_layer_by_theme::<_,SubnationTheme>(target, &bezier_scale, progress)
    }
    
}

command_def!{
    #[command(disable_help_subcommand(true))]
    pub SubnationCommand {
        Create,
        Expand,
        FillEmpty,
        Normalize,
        Dissolve,
        Curvify
    }
}


#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[arg(long,required=true)]
    /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
    pub namers: Vec<PathBuf>,

    #[arg(long)]
    /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
    pub default_namer: Option<String>,

    #[arg(long,default_value("20"))]
    /// The percent of towns in each nation to use for subnations
    pub subnation_percentage: f64,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[clap(flatten)]
    pub random_seed_arg: RandomSeedArg,

    #[clap(flatten)]
    pub overwrite_subnations_arg: OverwriteSubnationsArg,

}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenSubnations {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<SubnationCommand>

    }
}


impl Task for GenSubnations {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        if let Some(default_args) = self.default_args {

            let mut random = random_number_generator(default_args.random_seed_arg);

            let mut target = WorldMap::edit(default_args.target_arg.target)?;

            let namers = NamerSetSource::from_files(default_args.namers)?;

            let mut loaded_namers = NamerSet::load_from(namers, default_args.default_namer, &mut random, progress)?;

            let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForNations>(progress)?;
    
            Self::run_default(&mut random, culture_lookup, &mut loaded_namers, default_args.subnation_percentage, default_args.overwrite_subnations_arg, &default_args.bezier_scale_arg, &mut target, progress)

        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }
    }
}


impl GenSubnations {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, subnation_percentage: f64, overwrite_subnations: OverwriteSubnationsArg, bezier_scale: &BezierScaleArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {

            Create::run_with_parameters(random, &culture_lookup, loaded_namers, subnation_percentage, overwrite_subnations, target, progress)?;

            Expand::run_with_parameters(random, subnation_percentage, target, progress)?;

            FillEmpty::run_with_parameters(random, &culture_lookup, loaded_namers, subnation_percentage, target, progress)?;

            Normalize::run_with_parameters(target, progress)?;

            Dissolve::run_with_parameters(target, progress)?;

            Curvify::run_with_parameters(bezier_scale, target, progress)


        })?;

        target.save(progress)
    }
}

