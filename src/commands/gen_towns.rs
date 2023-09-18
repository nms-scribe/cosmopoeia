use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use super::Task;
use crate::command_def;
use crate::algorithms::towns::populate_towns;
use crate::algorithms::towns::generate_towns;
use crate::world_map::CultureForTowns;
use crate::algorithms::naming::NamerSetSource;
use crate::world_map::WorldMap;
use crate::utils::random_number_generator;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::CultureSchema;
use crate::world_map::EntityLookup;
use crate::algorithms::naming::NamerSet;
use crate::world_map::NamedEntity;
use crate::world_map::CultureWithNamer;
use crate::commands::TargetArg;
use super::RandomSeedArg;
use super::OverwriteTownsArg;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Create {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="20")]
        /// The number of national capitals to create
        pub capital_count: usize,

        #[arg(long)]
        /// The number of non-capital towns to create
        pub town_count: Option<usize>,

        #[arg(long,required=true)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        pub namers: Vec<PathBuf>,

        #[arg(long)]
        /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
        pub default_namer: Option<String>,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub overwrite_towns_arg: OverwriteTownsArg,
    
    }
}

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.random_seed_arg);

        let mut target = WorldMap::edit(self.target_arg.target)?;

        let namers = NamerSetSource::from_files(self.namers)?;

        let mut loaded_namers = NamerSet::load_from(namers, self.default_namer, &mut random, progress)?;

        let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForTowns>(progress)?;

        
        target.with_transaction(|target| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, self.capital_count, self.town_count, self.overwrite_towns_arg, target, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, capital_count: usize, town_count: Option<usize>, overwrite_towns: OverwriteTownsArg, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating towns");
        generate_towns(target, random, &culture_lookup, loaded_namers, capital_count, town_count, overwrite_towns, progress)
    }
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub struct Populate {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="10")] 
        /// A waterflow threshold above which the tile will count as a river
        pub river_threshold: f64,

    }
}

impl Task for Populate {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(self.river_threshold, target, progress)
        })?;

        target.save(progress)

    }
}

impl Populate {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
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

    #[arg(long,default_value="20")]
    /// The number of national capitals to create
    pub capital_count: usize,

    #[arg(long)]
    /// The number of non-capital towns to create
    pub town_count: Option<usize>,

    #[arg(long,required=true)]
    /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
    pub namers: Vec<PathBuf>,

    #[arg(long)]
    /// The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
    pub default_namer: Option<String>,

    #[clap(flatten)]
    pub random_seed_arg: RandomSeedArg,

    #[arg(long,default_value="10")]
    /// A waterflow threshold above which the tile will count as a river
    pub river_threshold: f64,

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
        
            let mut random = random_number_generator(default_args.random_seed_arg);

            let mut target = WorldMap::edit(default_args.target_arg.target)?;
    
            let namers = NamerSetSource::from_files(default_args.namers)?;

            let mut loaded_namers = NamerSet::load_from(namers, default_args.default_namer, &mut random, progress)?;

            let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForTowns>(progress)?;
    
    
            Self::run_default(&mut random, &culture_lookup, &mut loaded_namers, default_args.capital_count, default_args.town_count, default_args.river_threshold, default_args.overwrite_towns_arg, &mut target, progress)
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenTowns {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut NamerSet, capital_count: usize, town_count: Option<usize>, river_threshold: f64, overwrite_towns: OverwriteTownsArg, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {

            Create::run_with_parameters(random, culture_lookup, loaded_namers, capital_count, town_count, overwrite_towns, target, progress)?;

            Populate::run_with_parameters(river_threshold, target, progress)

        })?;

        target.save(progress)
    }
}