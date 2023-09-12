use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::world_map::WorldMap;
use crate::algorithms::biomes::fill_biome_defaults;
use crate::algorithms::biomes::apply_biomes;
use crate::algorithms::tiles::dissolve_tiles_by_theme;
use crate::algorithms::tiles::BiomeTheme;
use crate::algorithms::curves::curvify_layer_by_theme;
use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::world_map::BiomeMatrix;

subcommand_def!{
    /// Creates default biome layer
    #[command(hide=true)]
    pub(crate) struct Data {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the biome layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool

    }
}

impl Task for Data {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            Self::run(self.overwrite, target, progress)

        })?;

        target.save(progress)
    }
}

impl Data {

    fn run<Progress: ProgressObserver>(overwrite: bool, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {

        progress.announce("Filling biome defaults");

        fill_biome_defaults(target, overwrite, progress)
    }
}

subcommand_def!{
    /// Applies data from biomes layer to the tiles
    #[command(hide=true)]
    pub(crate) struct Apply {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for Apply {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {


        let mut target = WorldMap::edit(self.target)?;

        let biomes = target.biomes_layer()?.get_matrix(progress)?;

        target.with_transaction(|target| {

            Self::run(target, biomes, progress)

        })?;

        target.save(progress)


    }
}

impl Apply {

    // TODO: Make all of these take the progress observer thingie
    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, biomes: BiomeMatrix, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Applying biomes to tiles");
    
        apply_biomes(target, biomes, progress)
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
            Self::run(target, progress)
        })?;

        target.save(progress)

    }
}

impl Dissolve {
    fn run<Progress: ProgressObserver>(target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Creating biome polygons");
    
        dissolve_tiles_by_theme::<_,BiomeTheme>(target, progress)
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
            Self::run(self.bezier_scale, target, progress)
        })?;

        target.save(progress)

    }
}

impl Curvify {
    fn run<Progress: ProgressObserver>(bezier_scale: f64, target: &mut WorldMapTransaction<'_>, progress: &mut Progress) -> Result<(), CommandError> {
        progress.announce("Making biome polygons curvy");

        curvify_layer_by_theme::<_,BiomeTheme>(target, bezier_scale, progress)
    }
}


command_def!{
    BiomeCommand {
        Data,
        Apply,
        Dissolve,
        Curvify
    }
}

#[derive(Args)]
struct DefaultArgs {
    /// The path to the world map GeoPackage file
    target: PathBuf,

    #[arg(long,default_value="100")]
    /// This number is used for generating points to make curvy lines. The higher the number, the smoother the curves.
    bezier_scale: f64,

    #[arg(long)]
    /// If true and the biome layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
    overwrite: bool
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    #[command(args_conflicts_with_subcommands = true)]
    pub(crate) struct GenBiome {

        #[clap(flatten)]
        default_args: Option<DefaultArgs>,

        #[command(subcommand)]
        command: Option<BiomeCommand>

    }
}

impl Task for GenBiome {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        if let Some(args) = self.default_args {
            let mut target = WorldMap::edit(args.target)?;

            Self::run(args.overwrite, args.bezier_scale, &mut target, progress)
    
        } else if let Some(command) = self.command {

            command.run(progress)
        } else {
            unreachable!("Command should have been called with one of the arguments")
        }

    }
}

impl GenBiome {
    fn run<Progress: ProgressObserver>(overwrite: bool, bezier_scale: f64, target: &mut WorldMap, progress: &mut Progress) -> Result<(), CommandError> {
        target.with_transaction(|target| {            
            Data::run(overwrite, target, progress)

        })?;
        let biomes = target.biomes_layer()?.get_matrix(progress)?;
        target.with_transaction(|target| {            
            Apply::run(target, biomes, progress)?;

            Dissolve::run(target, progress)?;

            Curvify::run(bezier_scale, target, progress)

        })?;

        target.save(progress)
    }
}