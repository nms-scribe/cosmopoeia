use clap::Args;
use clap::Subcommand;
use rand::Rng;

use crate::commands::Task;
use crate::command_def;
use crate::algorithms::towns::populate_towns;
use crate::algorithms::towns::generate_towns;
use crate::world_map::culture_layer::CultureForTowns;
use crate::world_map::WorldMap;
use crate::utils::random::random_number_generator;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::culture_layer::CultureSchema;
use crate::typed_map::entities::EntityLookup;
use crate::algorithms::naming::NamerSet;
use crate::typed_map::entities::NamedEntity;
use crate::world_map::culture_layer::CultureWithNamer;
use crate::commands::TargetArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteTownsArg;
use crate::commands::NamerArg;
use crate::commands::RiverThresholdArg;
use crate::commands::TownCountsArg;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub town_counts_arg: TownCountsArg,

        #[clap(flatten)]
        pub namer_arg: NamerArg,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_towns_arg: OverwriteTownsArg,
    
    }
}

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(&self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;

        let mut loaded_namers = NamerSet::load_from(self.namer_arg, &mut random, progress)?;

        let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForTowns>(progress)?;

        
        target.with_transaction(|transaction| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, &self.town_counts_arg, &self.overwrite_towns_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, count_arg: &TownCountsArg, overwrite_towns: &OverwriteTownsArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating towns");
        generate_towns(target, random, culture_lookup, loaded_namers, count_arg, overwrite_towns, progress)
    }
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Populate {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub river_threshold_arg: RiverThresholdArg,
        
    }
}

impl Task for Populate {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {

            Self::run_with_parameters(&self.river_threshold_arg, transaction, progress)
        })?;

        target.save(progress)

    }
}

impl Populate {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: &RiverThresholdArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Populating towns");
        populate_towns(target, river_threshold, progress)
    }
}


command_def!{
    #[command(disable_help_subcommand(true))]
    pub TownCommand {
        Create,
        Populate
    }
}


#[derive(Args)]
pub struct DefaultArgs {

    #[clap(flatten)]
    pub target_arg: TargetArg,

    #[clap(flatten)]
    pub town_counts_arg: TownCountsArg,

    #[clap(flatten)]
    pub namer_arg: NamerArg,

    #[clap(flatten)]
    pub random_seed_arg: RandomSeedArg,

    #[clap(flatten)]
    pub river_threshold_arg: RiverThresholdArg,

    #[clap(flatten)]
    pub overwrite_towns_arg: OverwriteTownsArg,


}


subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub struct GenTowns {

        #[clap(flatten)]
        pub default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        pub command: Option<TownCommand>

    }
}

impl Task for GenTowns {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {
        
            let mut random = random_number_generator(&default_args.random_seed_arg);

            let mut target = WorldMap::edit(default_args.target_arg.target)?;
    
            let mut loaded_namers = NamerSet::load_from(default_args.namer_arg, &mut random, progress)?;

            let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForTowns>(progress)?;
    
    
            Self::run_default(&mut random, &culture_lookup, &mut loaded_namers, &default_args.town_counts_arg, &default_args.river_threshold_arg, &default_args.overwrite_towns_arg, &mut target, progress)
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenTowns {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, count_args: &TownCountsArg, river_threshold: &RiverThresholdArg, overwrite_towns: &OverwriteTownsArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|transaction| {

            Create::run_with_parameters(random, culture_lookup, loaded_namers, count_args, overwrite_towns, transaction, progress)?;

            Populate::run_with_parameters(river_threshold, transaction, progress)

        })?;

        target.save(progress)
    }
}