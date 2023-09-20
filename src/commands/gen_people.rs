use clap::Args;
use clap::Subcommand;
use rand::Rng;

use crate::commands::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;
use crate::algorithms::population::generate_populations;
use crate::algorithms::cultures::generate_cultures;
use crate::algorithms::cultures::expand_cultures;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::naming::NamerSet;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::utils::random_number_generator;
use crate::algorithms::tiles::CultureTheme;
use crate::algorithms::curves::curvify_layer_by_theme;
use crate::world_map::WorldMapTransaction;
use crate::commands::TargetArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteCulturesArg;
use crate::commands::BezierScaleArg;
use crate::commands::NamerArg;
use crate::commands::SizeVarianceArg;
use crate::commands::RiverThresholdArg;
use crate::commands::ExpansionFactorArg;
use crate::commands::CulturesGenArg;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Population {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub river_threshold_arg: RiverThresholdArg,
        
    }
}

impl Task for Population {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.river_threshold_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Population {
    fn run_with_parameters<Progress: ProgressObserver>(estuary_threshold: &RiverThresholdArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating population");
        generate_populations(target, estuary_threshold, progress)
    }
    
}


subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct CreateCultures {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub cultures_arg: CulturesGenArg,

        #[clap(flatten)]
        pub namer_arg: NamerArg,
        
        #[clap(flatten)]
        pub size_variance_arg: SizeVarianceArg,

        #[clap(flatten)]
        pub river_threshold_arg: RiverThresholdArg,
    
        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_cultures_arg: OverwriteCulturesArg,
    
    
    }
}

impl Task for CreateCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(&self.random_seed_arg);

        let mut loaded_namers = NamerSet::load_from(self.namer_arg, &mut random, progress)?;

        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(&mut random, &self.cultures_arg, &mut loaded_namers, &self.size_variance_arg, &self.river_threshold_arg, &self.overwrite_cultures_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl CreateCultures {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver>(random: &mut Random, cultures_arg: &CulturesGenArg, namers: &mut NamerSet, size_variance: &SizeVarianceArg, river_threshold: &RiverThresholdArg, overwrite_cultures: &OverwriteCulturesArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {

        progress.announce("Generating cultures");
        let cultures = CultureSet::from_files(&cultures_arg.cultures,random,namers)?;

        generate_cultures(target, random, &cultures, namers, cultures_arg.culture_count, size_variance, river_threshold, overwrite_cultures, progress)
    }
    
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct ExpandCultures {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub river_threshold_arg: RiverThresholdArg,
    
        #[clap(flatten)]
        pub expansion_factor_arg: ExpansionFactorArg,

    }
}

impl Task for ExpandCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;
        target.with_transaction(|transaction| {
            Self::run_with_parameters(&self.river_threshold_arg, &self.expansion_factor_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl ExpandCultures {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: &RiverThresholdArg, limit_factor: &ExpansionFactorArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying cultures to tiles");
    
        expand_cultures(target, river_threshold, limit_factor, progress)
    }
    
}

subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub struct DissolveCultures {

        #[clap(flatten)]
        pub target_arg: TargetArg,

    }
}

impl Task for DissolveCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(transaction, progress)
        })?;

        target.save(progress)

    }
}

impl DissolveCultures {
    fn run_with_parameters<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating culture polygons");
    
        dissolve_tiles_by_theme::<_,CultureTheme>(target, progress)
    }
    
}



subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub struct CurvifyCultures {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub bezier_scale_arg: BezierScaleArg,

    }
}

impl Task for CurvifyCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;
        let bezier_scale = self.bezier_scale_arg;

        target.with_transaction(|transaction| {
            Self::run_with_parameters(&bezier_scale, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl CurvifyCultures {

    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: &BezierScaleArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making culture polygons curvy");
    
        curvify_layer_by_theme::<_,CultureTheme>(target, bezier_scale, progress)
    }
    
}



command_def!{
    #[command(disable_help_subcommand(true))]
    pub PeopleCommand {
        Population,
        CreateCultures,
        ExpandCultures,
        DissolveCultures,
        CurvifyCultures
    }
}


#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[clap(flatten)]
    pub cultures_arg: CulturesGenArg,

    #[clap(flatten)]
    pub river_threshold_arg: RiverThresholdArg,

    #[clap(flatten)]
    pub expansion_factor_arg: ExpansionFactorArg,

    #[clap(flatten)]
    pub namer_arg: NamerArg,

    #[clap(flatten)]
    pub size_variance_arg: SizeVarianceArg,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[clap(flatten)]
    pub random_seed_arg: RandomSeedArg,

    #[clap(flatten)]
    pub overwrite_cultures_arg: OverwriteCulturesArg,


}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenPeople {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<PeopleCommand>

    }
}

impl Task for GenPeople {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {

            let mut random = random_number_generator(&default_args.random_seed_arg);

            let mut loaded_namers = NamerSet::load_from(default_args.namer_arg, &mut random, progress)?;
    
            let mut target = WorldMap::edit(default_args.target_arg.target)?;
    
            Self::run_default(
                &default_args.river_threshold_arg, 
                &default_args.cultures_arg, 
                &mut loaded_namers, 
                &default_args.size_variance_arg, 
                &default_args.overwrite_cultures_arg, 
                &default_args.expansion_factor_arg, 
                &default_args.bezier_scale_arg, 
                &mut target, 
                &mut random, 
                progress
            )
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenPeople {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(river_threshold: &RiverThresholdArg, cultures: &CulturesGenArg, namers: &mut NamerSet, size_variance: &SizeVarianceArg, overwrite_cultures: &OverwriteCulturesArg, limit_factor: &ExpansionFactorArg, bezier_scale: &BezierScaleArg, target: &mut WorldMap, random: &mut Random, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {
            Population::run_with_parameters(river_threshold, transaction, progress)?;
    
            CreateCultures::run_with_parameters(random, cultures, namers, size_variance, river_threshold, overwrite_cultures, transaction, progress)?;
    
            ExpandCultures::run_with_parameters(river_threshold, limit_factor, transaction, progress)?;
    
            DissolveCultures::run_with_parameters(transaction, progress)?;
    
            CurvifyCultures::run_with_parameters(bezier_scale, transaction, progress)
    
        })?;
    
        target.save(progress)
    }
    
}

