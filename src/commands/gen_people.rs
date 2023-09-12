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

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct Population {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10")]
        /// A waterflow threshold above which population increases along the coast
        estuary_threshold: f64

    }
}

impl Task for Population {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            Self::run_with_parameters(self.estuary_threshold, target, progress)
        })?;

        target.save(progress)

    }
}

impl Population {
    fn run_with_parameters<Progress: ProgressObserver>(estuary_threshold: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Generating population");
        generate_populations(target, estuary_threshold, progress)
    }
    
}


subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct CreateCultures {

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
        no_builtin_namers: bool,

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

impl Task for CreateCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut cultures = CultureSet::empty();
        for file in self.cultures {
            cultures.extend_from_file(file)?;
        }

        let namers = NamerSet::from_files(self.namers, !self.no_builtin_namers)?;

        let mut random = random_number_generator(self.seed);

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(&mut random, cultures, &namers, self.count, self.size_variance, self.river_threshold, self.overwrite, target, progress)
        })?;

        target.save(progress)

    }
}

impl CreateCultures {
    fn run_with_parameters<Random: Rng, Progress: ProgressObserver>(random: &mut Random, cultures: CultureSet, namers: &NamerSet, culture_count: usize, size_variance: f64, river_threshold: f64, overwrite_cultures: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        let size_variance = size_variance.clamp(0.0, 10.0);

        progress.announce("Generating cultures");
        generate_cultures(target, random, cultures, namers, culture_count, size_variance, river_threshold, overwrite_cultures, progress)
    }
    
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(hide=true)]
    pub(crate) struct ExpandCultures {

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

impl Task for ExpandCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;
        target.with_transaction(|target| {
            Self::run_with_parameters(self.river_threshold, self.limit_factor, target, progress)
        })?;

        target.save(progress)

    }
}

impl ExpandCultures {
    fn run_with_parameters<Progress: ProgressObserver>(river_threshold: f64, limit_factor: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying cultures to tiles");
    
        expand_cultures(target, river_threshold, limit_factor, progress)
    }
    
}

subcommand_def!{
    /// Generates polygons in cultures layer
    #[command(hide=true)]
    pub(crate) struct DissolveCultures {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for DissolveCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {
            Self::run_with_parameters(target, progress)
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
    pub(crate) struct CurvifyCultures {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
        bezier_scale: f64,

    }
}

impl Task for CurvifyCultures {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;
        let bezier_scale = self.bezier_scale;

        target.with_transaction(|target| {
            Self::run_with_parameters(bezier_scale, target, progress)
        })?;

        target.save(progress)

    }
}

impl CurvifyCultures {

    fn run_with_parameters<Progress: ProgressObserver>(bezier_scale: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making culture polygons curvy");
    
        curvify_layer_by_theme::<_,CultureTheme>(target, bezier_scale, progress)
    }
    
}



command_def!{
    #[command(disable_help_subcommand(true))]
    PeopleCommand {
        Population,
        CreateCultures,
        ExpandCultures,
        DissolveCultures,
        CurvifyCultures
    }
}

#[derive(Args)]
struct DefaultArgs {

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
    no_builtin_namers: bool,

    #[arg(long,default_value("10"))]
    /// The number of cultures to generate
    culture_count: usize,

    #[arg(long,default_value("1"))]
    /// A number, clamped to 0-10, which controls how much cultures can vary in size
    size_variance: f64,

    #[arg(long,default_value="100")]
    /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
    bezier_scale: f64,

    #[arg(long)]
    /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
    seed: Option<u64>,

    #[arg(long)]
    /// If true and the cultures layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
    overwrite: bool
}

subcommand_def!{
    /// Generates background population of tiles
    #[command(args_conflicts_with_subcommands = true)]
    pub(crate) struct GenPeople {

        #[clap(flatten)]
        default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        command: Option<PeopleCommand>

    }
}

impl Task for GenPeople {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(default_args) = self.default_args {
            let mut cultures = CultureSet::empty();
            for file in default_args.cultures {
                cultures.extend_from_file(file)?;
            }
    
            let namers = NamerSet::from_files(default_args.namers, !default_args.no_builtin_namers)?;
    
            let mut random = random_number_generator(default_args.seed);
    
            let mut target = WorldMap::edit(default_args.target)?;
    
            Self::run_default(
                default_args.river_threshold, 
                cultures, 
                &namers, 
                default_args.culture_count, 
                default_args.size_variance, 
                default_args.overwrite, 
                default_args.limit_factor, 
                default_args.bezier_scale, 
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
    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(river_threshold: f64, cultures: CultureSet, namers: &NamerSet, culture_count: usize, size_variance: f64, overwrite_cultures: bool, limit_factor: f64, bezier_scale: f64, target: &mut WorldMap, random: &mut Random, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {
            Population::run_with_parameters(river_threshold, target, progress)?;
    
            CreateCultures::run_with_parameters(random, cultures, namers, culture_count, size_variance, river_threshold, overwrite_cultures, target, progress)?;
    
            ExpandCultures::run_with_parameters(river_threshold, limit_factor, target, progress)?;
    
            DissolveCultures::run_with_parameters(target, progress)?;
    
            CurvifyCultures::run_with_parameters(bezier_scale, target, progress)
    
        })?;
    
        target.save(progress)
    }
    
}

