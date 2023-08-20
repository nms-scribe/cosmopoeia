use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::biomes::fill_biome_defaults;
use crate::algorithms::biomes::apply_biomes;

subcommand_def!{
    /// Creates default biome layer
    pub(crate) struct GenBiomeData {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the biome layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool

    }
}

impl Task for GenBiomeData {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Filling biome defaults:");

            fill_biome_defaults(target, self.overwrite, &mut progress)

        })?;

        target.save(&mut progress)
    }
}

subcommand_def!{
    /// Applies data from biomes layer to the tiles
    pub(crate) struct GenBiomeApply {

        /// The path to the world map GeoPackage file
        target: PathBuf,

    }
}

impl Task for GenBiomeApply {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let biomes = target.biomes_layer()?.get_matrix(&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Applying biomes to tiles:");

            apply_biomes(target, biomes, &mut progress)

        })?;

        target.save(&mut progress)


    }
}

subcommand_def!{
    /// Generates precipitation data (requires wind and temperatures)
    pub(crate) struct GenBiome {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the biome layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool

    }
}

impl Task for GenBiome {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Filling biome defaults:");

            fill_biome_defaults(target, self.overwrite, &mut progress)
        })?;

        let biomes = target.biomes_layer()?.get_matrix(&mut progress)?;

        target.with_transaction(|target| {

            progress.announce("Applying biomes to tiles:");

            apply_biomes(target, biomes, &mut progress)
        })?;

        target.save(&mut progress)
    }
}