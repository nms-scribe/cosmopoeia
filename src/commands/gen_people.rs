use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::population::generate_populations;
use crate::algorithms::cultures::generate_cultures;
use crate::algorithms::cultures::expand_cultures;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::naming::NamerSet;
use crate::utils::random_number_generator;

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenPeoplePopulation {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which population increases along the coast
        estuary_threshold: f64

    }
}

impl Task for GenPeoplePopulation {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Generating population:");
            generate_populations(target, self.estuary_threshold, &mut progress)
        })?;

        target.save(&mut progress)

    }
}


subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenPeopleCultures {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,required(true))] 
        /// Files to load culture sets from, more than one may be specified to load multiple culture sets.
        cultures: Vec<PathBuf>,

        #[arg(long)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

        #[arg(long)]
        /// if specified, the default namers will not be loaded.
        no_default_namers: bool,

        #[arg(long,default_value("10"))]
        /// The number of cultures to generate
        count: usize,

        #[arg(long,default_value("1"))]
        /// A number, clamped to 0-10, which controls how much cultures can vary in size
        size_variance: f64,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the cultures layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool

    }
}

impl Task for GenPeopleCultures {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut cultures = CultureSet::empty();
        for file in self.cultures {
            cultures.extend_from_file(file)?;
        }

        let mut namers = if self.no_default_namers {
            NamerSet::empty()
        } else {
            NamerSet::default()?
        };
        for file in self.namers {
            namers.extend_from_file(file,false)?;
        }

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let size_variance = self.size_variance.clamp(0.0, 10.0);

        target.with_transaction(|target| {
            progress.announce("Generating cultures:");
            generate_cultures(target, &mut random, cultures, namers, self.count, size_variance, self.river_threshold, self.overwrite, &mut progress)
        })?;

        target.save(&mut progress)

    }
}

subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenPeopleExpandCultures {

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

impl Task for GenPeopleExpandCultures {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Applying cultures to tiles:");

            expand_cultures(target, self.river_threshold, self.limit_factor, &mut progress)
        })?;

        target.save(&mut progress)

    }
}


subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenPeople {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,required(true))] 
        /// Files to load culture sets from, more than one may be specified to load multiple culture sets.
        cultures: Vec<PathBuf>,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which the tile will count as a river
        river_threshold: f64,

        #[arg(long,default_value("1"))]
        /// A number, usually ranging from 0.1 to 2.0, which limits how far cultures will expand. The higher the number, the less neutral lands.
        limit_factor: f64,

        #[arg(long)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

        #[arg(long)]
        /// if specified, the default namers will not be loaded.
        no_default_namers: bool,

        #[arg(long,default_value("10"))]
        /// The number of cultures to generate
        culture_count: usize,

        #[arg(long,default_value("1"))]
        /// A number, clamped to 0-10, which controls how much cultures can vary in size
        size_variance: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the cultures layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool

    }
}

impl Task for GenPeople {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut cultures = CultureSet::empty();
        for file in self.cultures {
            cultures.extend_from_file(file)?;
        }

        let mut namers = if self.no_default_namers {
            NamerSet::empty()
        } else {
            NamerSet::default()?
        };
        for file in self.namers {
            namers.extend_from_file(file,false)?;
        }

        let size_variance = self.size_variance.clamp(0.1, 10.0);

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Generating population:");
            generate_populations(target, self.river_threshold, &mut progress)?;

            progress.announce("Generating cultures:");
            generate_cultures(target, &mut random, cultures, namers, self.culture_count, size_variance, self.river_threshold, self.overwrite, &mut progress)?;

            progress.announce("Applying cultures to tiles:");
            expand_cultures(target, self.river_threshold, self.limit_factor, &mut progress)
        })?;

        target.save(&mut progress)

    }
}
