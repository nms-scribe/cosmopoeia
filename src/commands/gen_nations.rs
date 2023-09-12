use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use super::Task;
use crate::algorithms::nations::normalize_nations;
use crate::algorithms::nations::expand_nations;
use crate::algorithms::nations::generate_nations;
use crate::world_map::CultureForNations;
use crate::algorithms::naming::NamerSet;
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
use crate::algorithms::naming::LoadedNamers;
use crate::world_map::NamedEntity;
use crate::world_map::CultureWithNamer;
use crate::world_map::CultureWithType;

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Create {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,required=true)]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namers: Vec<PathBuf>,

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

impl Task for Create {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        let namers = NamerSet::from_files(self.namers)?;

        let (culture_lookup,mut loaded_namers) = CultureSchema::get_lookup_and_namers::<CultureForNations,_>(namers, self.default_namer, &mut target, progress)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(&mut random, &culture_lookup, &mut loaded_namers, self.size_variance, self.overwrite, target, progress)
        })?;

        target.save(progress)

    }
}

impl Create {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut LoadedNamers, size_variance: f64, overwrite_nations: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating nations");
        generate_nations(target, random, &culture_lookup, loaded_namers, size_variance, overwrite_nations, progress)
    }
    
}


subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Expand {

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

impl Task for Expand {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;
        target.with_transaction(|target| {
            Self::run_with_parameters(self.river_threshold, self.limit_factor, target, progress)
        })?;

        target.save(progress)

    }
}

impl Expand {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: f64, limit_factor: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying nations to tiles");
    
        expand_nations(target, river_threshold, limit_factor, progress)
    }
    
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Normalize {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl Task for Normalize {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(target, progress)
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
    pub(crate) struct Dissolve {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for Dissolve {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(target, progress)
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
    pub(crate) struct Curvify {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
        bezier_scale: f64,

    }
}

impl Task for Curvify {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(self.bezier_scale, target, progress)
        })?;

        target.save(progress)

    }
}

impl Curvify {
    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making nation polygons curvy");
    
        curvify_layer_by_theme::<_,NationTheme>(target, bezier_scale, progress)
    }
    
}



command_def!{
    #[command(disable_help_subcommand(true))]
    NationCommand {
        Create,
        Expand,
        Normalize,
        Dissolve,
        Curvify
    }
}


#[derive(Args)]
struct DefaultArgs {

    /// The path to the world map GeoPackage file
    target: PathBuf,

    #[arg(long,required=true)]
    /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
    namers: Vec<PathBuf>,

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

    #[arg(long,default_value="100")]
    /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
    bezier_scale: f64,

    #[arg(long)]
    /// If true and the towns layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
    overwrite: bool


}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub(crate) struct GenNations {

        #[clap(flatten)]
        default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        command: Option<NationCommand>


    }
}

impl Task for GenNations {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {
            let mut random = random_number_generator(default_args.seed);

            let mut target = WorldMap::edit(default_args.target)?;
    
            let namers = NamerSet::from_files(default_args.namers)?;
    
            let (culture_lookup,mut loaded_namers) = CultureSchema::get_lookup_and_namers::<CultureForNations,_>(namers, default_args.default_namer, &mut target, progress)?;

            Self::run_default(&mut random, &culture_lookup, &mut loaded_namers, default_args.size_variance, default_args.river_threshold, default_args.limit_factor, default_args.bezier_scale, default_args.overwrite, &mut target, progress)

        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}


impl GenNations {

    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(random: &mut Random, culture_lookup: &EntityLookup<CultureSchema, Culture>, loaded_namers: &mut LoadedNamers, size_variance: f64, river_threshold: f64, limit_factor: f64, bezier_scale: f64, overwrite_nations: bool, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {
    
            Create::run_with_parameters(random, culture_lookup, loaded_namers, size_variance, overwrite_nations, target, progress)?;
    
            Expand::run_with_parameters(river_threshold, limit_factor, target, progress)?;
    
            Normalize::run_with_parameters(target, progress)?;
    
            Dissolve::run_with_parameters(target, progress)?;
    
            Curvify::run_with_parameters(bezier_scale, target, progress)
    
        })?;
    
        target.save(progress)
    }
    

}
