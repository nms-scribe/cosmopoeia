use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::progress::ProgressObserver;

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

        // TODO: This is just playing around with layers that have no geometries, until we have the algorithm figured out.

        target.with_transaction(|target| {
            let mut biomes = target.create_biomes_layer(self.overwrite)?;

            biomes.add_biome("Tropical".to_owned())?;
            biomes.add_biome("Temperate".to_owned())?;

            Ok(())
        })?;

        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        target.save()?;

        let mut biomes = target.biomes_layer()?;
        for biome in biomes.list_biomes()? {
            println!("{}",biome)
        }
    
        progress.finish(|| "Layer Saved.");

        Ok(())


    }
}