use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::naming::NamerSet;
use crate::algorithms::civilization::generate_towns;
use crate::world_map::CultureForTowns;
use crate::world_map::CultureForNations;
use crate::algorithms::civilization::generate_nations;
use crate::utils::random_number_generator;
use crate::algorithms::civilization::expand_nations;
use crate::algorithms::civilization::normalize_nations;

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenCivilTowns {

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

impl Task for GenCivilTowns {

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
    pub(crate) struct GenCivilCreateNations {

        /// The path to the world map GeoPackage file
        target: PathBuf,

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

        #[arg(long,default_value("1"))]
        /// A number, clamped to 0-10, which controls how much cultures can vary in size
        size_variance: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool



    }
}

impl Task for GenCivilCreateNations {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForNations,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating nations");
            generate_nations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.size_variance, self.overwrite, &mut progress)
        })?;

        target.save(&mut progress)

    }
}


subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenCivilExpandNations {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

        #[arg(long,default_value("1"))]
        /// A number, usually ranging from 0.1 to 2.0, which limits how far cultures will expand. The higher the number, the less neutral lands.
        limit_factor: f64

    }
}

impl Task for GenCivilExpandNations {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Applying nations to tiles");

            expand_nations(target, self.river_threshold, self.limit_factor, &mut progress)
        })?;

        target.save(&mut progress)

    }
}


subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenCivilNormalizeNations {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl Task for GenCivilNormalizeNations {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Normalizing nation borders");

            normalize_nations(target, &mut progress)
        })?;

        target.save(&mut progress)

    }
}



subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenCivilNations {

        /// The path to the world map GeoPackage file
        target: PathBuf,

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

        #[arg(long,default_value("1"))]
        /// A number, clamped to 0-10, which controls how much cultures can vary in size
        size_variance: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

        #[arg(long,default_value("1"))]
        /// A number, usually ranging from 0.1 to 2.0, which limits how far cultures will expand. The higher the number, the less neutral lands.
        limit_factor: f64,

        #[arg(long)]
        /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool



    }
}

impl Task for GenCivilNations {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForNations,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating nations");
            generate_nations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.size_variance, self.overwrite, &mut progress)?;

            progress.announce("Applying nations to tiles");

            expand_nations(target, self.river_threshold, self.limit_factor, &mut progress)?;

            progress.announce("Normalizing nation borders");

            normalize_nations(target, &mut progress)

        })?;

        target.save(&mut progress)

    }
}


