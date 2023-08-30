use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::naming::NamerSet;
use crate::world_map::CultureForNations;
use crate::utils::random_number_generator;
use crate::algorithms::subnations::generate_subnations;
use crate::algorithms::subnations::expand_subnations;
use crate::algorithms::subnations::fill_empty_subnations;
use crate::algorithms::subnations::normalize_subnations;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::algorithms::tiles::SubnationTheme;
use crate::algorithms::curves::curvify_layer_by_theme;



subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenSubnationsCreate {

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

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        subnation_percentage: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool



    }
}

impl Task for GenSubnationsCreate {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForNations,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating subnations");
            generate_subnations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.subnation_percentage, self.overwrite, &mut progress)
        })?;

        target.save(&mut progress)

    }
}



subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenSubnationsExpand {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        subnation_percentage: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,




    }
}

impl Task for GenSubnationsExpand {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;
        

        target.with_transaction(|target| {
            progress.announce("Applying subnations to tiles");

            expand_subnations(target, &mut random, self.subnation_percentage, &mut progress)
        })?;

        target.save(&mut progress)

    }
}




subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenSubnationsFillEmpty {

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

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        subnation_percentage: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,




    }
}

impl Task for GenSubnationsFillEmpty {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;
        
        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForNations,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {
            progress.announce("Creating new subnations to fill rest of nations");

            fill_empty_subnations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.subnation_percentage, &mut progress)
        })?;

        target.save(&mut progress)

    }
}



subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenSubnationsNormalize {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl Task for GenSubnationsNormalize {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Normalizing subnation borders");

            normalize_subnations(target, &mut progress)
        })?;

        target.save(&mut progress)

    }
}


subcommand_def!{
    /// Generates polygons in cultures layer
    pub(crate) struct GenSubnationsDissolve {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenSubnationsDissolve {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Creating subnation polygons");

            dissolve_tiles_by_theme::<_,SubnationTheme>(target, &mut progress)
        })?;

        target.save(&mut progress)

    }
}



subcommand_def!{
    /// Generates polygons in cultures layer
    pub(crate) struct GenSubnationsCurvify {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
        bezier_scale: f64,

    }
}

impl Task for GenSubnationsCurvify {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            progress.announce("Making subnation polygons curvy");

            // FUTURE: Technically, subnations have to follow the curves of their owning nations as priority over their own. 
            // Right now, it doesn't seem to make a big difference if you have the nation borders thick enough. But it
            // may become important later.
            curvify_layer_by_theme::<_,SubnationTheme>(target, self.bezier_scale, &mut progress)
        })?;

        target.save(&mut progress)

    }
}



subcommand_def!{
    /// Generates background population of tiles
    pub(crate) struct GenSubnations {

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

        #[arg(long,default_value("20"))]
        /// The percent of towns in each nation to use for subnations
        subnation_percentage: f64,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
        bezier_scale: f64,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

    }
}


impl Task for GenSubnations {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        progress.announce("Preparing");

        let (culture_lookup,mut loaded_namers) = target.cultures_layer()?.get_lookup_and_load_namers::<CultureForNations,_>(namers,self.default_namer.clone(),&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Generating subnations");
            generate_subnations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.subnation_percentage, self.overwrite, &mut progress)?;

            progress.announce("Applying subnations to tiles");

            expand_subnations(target, &mut random, self.subnation_percentage, &mut progress)?;

            progress.announce("Creating new subnations to fill rest of nations");

            fill_empty_subnations(target, &mut random, &culture_lookup, &mut loaded_namers, &self.default_namer, self.subnation_percentage, &mut progress)?;

            progress.announce("Normalizing subnation borders");

            normalize_subnations(target, &mut progress)?;

            progress.announce("Creating subnation polygons");

            dissolve_tiles_by_theme::<_,SubnationTheme>(target, &mut progress)?;

            progress.announce("Making subnation polygons curvy");

            curvify_layer_by_theme::<_,SubnationTheme>(target, self.bezier_scale, &mut progress)


        })?;

        target.save(&mut progress)

    }
}


