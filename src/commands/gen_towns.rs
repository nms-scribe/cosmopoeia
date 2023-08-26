use std::path::PathBuf;
use clap::Args;

use super::Task;
use crate::algorithms::towns::populate_towns;
use crate::algorithms::towns::generate_towns;
use crate::world_map::CultureForTowns;
use crate::algorithms::naming::NamerSet;
use crate::world_map::WorldMap;
use crate::utils::random_number_generator;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenTownsCreate {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="20")]
        /// The number of national capitals to create
        capital_count: usize,

        #[arg(long)]
        /// The number of non-capital towns to create
        town_count: Option<usize>,

        #[arg(long)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

        #[arg(long)]
        /// if specified, the built-in namers will not be loaded.
        no_builtin_namers: bool,

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

impl Task for GenTownsCreate {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForTowns,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating towns");
            generate_towns(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.capital_count, self.town_count, self.overwrite, &mut progress)
        })?;

        target.save(&mut progress)

    }
}

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenTownsPopulate {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10")] // TODO: This default should be a constant somewhere.
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

    }
}

impl Task for GenTownsPopulate {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Populating towns");
            populate_towns(target, self.river_threshold, &mut progress)
        })?;

        target.save(&mut progress)

    }
}

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenTowns {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="20")]
        /// The number of national capitals to create
        capital_count: usize,

        #[arg(long)]
        /// The number of non-capital towns to create
        town_count: Option<usize>,

        #[arg(long)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

        #[arg(long)]
        /// if specified, the built-in namers will not be loaded.
        no_builtin_namers: bool,

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
}

impl Task for GenTowns {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForTowns,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating towns");
            generate_towns(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.capital_count, self.town_count, self.overwrite, &mut progress)?;

            progress.announce("Populating towns");
            populate_towns(target, self.river_threshold, &mut progress)

        })?;

        target.save(&mut progress)

    }
}
