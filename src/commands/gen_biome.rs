use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::progress::ProgressObserver;

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

        target.fill_biome_defaults(self.overwrite,&mut progress)?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        progress.finish(|| "Layer Saved.");

        Ok(())


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

        let biomes = target.get_biome_matrix(&mut progress)?;

        target.apply_biomes(biomes,&mut progress)?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        progress.finish(|| "Layer Saved.");

        Ok(())


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

        target.fill_biome_defaults(self.overwrite,&mut progress)?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        progress.finish(|| "Layer Saved.");

        let biomes = target.get_biome_matrix(&mut progress)?;

        target.apply_biomes(biomes,&mut progress)?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        progress.finish(|| "Layer Saved.");

        Ok(())

    }
}