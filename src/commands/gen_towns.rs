use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use super::Task;
use crate::command_def;
use crate::algorithms::towns::populate_towns;
use crate::algorithms::towns::generate_towns;
use crate::world_map::CultureForTowns;
use crate::algorithms::naming::NamerSet;
use crate::world_map::WorldMap;
use crate::utils::random_number_generator;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::CultureSchema;
use crate::world_map::EntityLookup;
use crate::algorithms::naming::LoadedNamers;
use crate::world_map::NamedEntity;
use crate::world_map::CultureWithNamer;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Create {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="20")]
        /// The number of national capitals to create
        capital_count: usize,

        #[arg(long)]
        /// The number of non-capital towns to create
        town_count: Option<usize>,

        #[arg(long,required=true)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

        // TODO: If I ever fill up the whole thing with cultures, then there shouldn't be any towns without a culture, and I can get rid of this.
        #[arg(long)]
        /// The name generator to use for naming towns in tiles without a culture
        default_namer: String,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool



    }
}

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers)?;

        let (culture_lookup,mut loaded_namers) = CultureSchema::get_lookup_and_namers::<CultureForTowns,_>(namers, self.default_namer, &mut target, progress)?;
        
        target.with_transaction(|target| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, self.capital_count, self.town_count, self.overwrite, target, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut LoadedNamers, capital_count: usize, town_count: Option<usize>, overwrite_towns: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating towns");
        generate_towns(target, random, &culture_lookup, loaded_namers, capital_count, town_count, overwrite_towns, progress)
    }
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Populate {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10")] // TODO: This default should be a constant somewhere.
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

    }
}

impl Task for Populate {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

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
    TownCommand {
        Create,
        Populate
    }
}


#[derive(Args)]
struct DefaultArgs {
    /// The path to the world map GeoPackage file
    target: PathBuf,

    #[arg(long,default_value="20")]
    /// The number of national capitals to create
    capital_count: usize,

    #[arg(long)]
    /// The number of non-capital towns to create
    town_count: Option<usize>,

    #[arg(long,required=true)]
    /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
    namers: Vec<PathBuf>,

    // TODO: If I ever fill up the whole thing with cultures, then there shouldn't be any towns without a culture, and I can get rid of this.
    #[arg(long)]
    /// The name generator to use for naming towns in tiles without a culture
    default_namer: String,

    #[arg(long)]
    /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
    seed: Option<u64>,

    #[arg(long,default_value="10")]
    /// A waterflow threshold above which the tile will count as a river
    river_threshold: f64,

    #[arg(long)]
    /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
    overwrite: bool

}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub(crate) struct GenTowns {

        #[clap(flatten)]
        default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        command: Option<TownCommand>

    }
}

impl Task for GenTowns {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {
        
            let mut random = random_number_generator(default_args.seed);

            let mut target = WorldMap::edit(default_args.target)?;
    
            let namers = NamerSet::from_files(default_args.namers)?;
    
            let (culture_lookup,mut loaded_namers) = CultureSchema::get_lookup_and_namers::<CultureForTowns,_>(namers, default_args.default_namer, &mut target, progress)?;
    
            Self::run_default(&mut random, &culture_lookup, &mut loaded_namers, default_args.capital_count, default_args.town_count, default_args.river_threshold, default_args.overwrite, &mut target, progress)
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenTowns {
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut LoadedNamers, capital_count: usize, town_count: Option<usize>, river_threshold: f64, overwrite_towns: bool, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {

            Create::run_with_parameters(random, culture_lookup, loaded_namers, capital_count, town_count, overwrite_towns, target, progress)?;

            Populate::run_with_parameters(river_threshold, target, progress)

        })?;

        target.save(progress)
    }
}