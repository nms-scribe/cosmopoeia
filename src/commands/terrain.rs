use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::raster::RasterMap;
use crate::algorithms::raster_sampling::OceanSamplingMethod;
use crate::algorithms::raster_sampling::sample_ocean_on_tiles;


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct TerrainSampleOceanBelow {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// Source file
        source: PathBuf,

        /// Elevation below which the terrain will be marked as ocean
        elevation: f64

    }
}

impl Task for TerrainSampleOceanBelow {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let source = RasterMap::open(self.source)?;

        target.with_transaction(|target| {

            progress.announce("Sample ocean data from heightmap");


            sample_ocean_on_tiles(target, &source, OceanSamplingMethod::Below(self.elevation), &mut progress)
    

        })?;

        target.save(&mut progress)


    }
}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct TerrainSampleOceanMasked {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        /// Source file
        source: PathBuf,

    }
}

impl Task for TerrainSampleOceanMasked {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        let source = RasterMap::open(self.source)?;

        target.with_transaction(|target| {

            progress.announce("Sample ocean data from heightmap");

            sample_ocean_on_tiles(target, &source, OceanSamplingMethod::AllData, &mut progress)
    

        })?;

        target.save(&mut progress)


    }
}