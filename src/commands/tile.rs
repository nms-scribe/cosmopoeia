use std::path::PathBuf;

use clap::Args;

use super::Task;
use crate::algorithms::tiles::calculate_tile_neighbors;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::algorithms::tiles::calculate_coastline;

// These are tasks from early in the process that could work with converted heightmaps or generated terrain.

subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct CalcNeighbors {

        /// The path to the world map GeoPackage file
        target: PathBuf,


    }
}

impl Task for CalcNeighbors {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Calculate neighbors for tiles");

            calculate_tile_neighbors(target, &mut progress)
        })?;

        target.save(&mut progress)


    }
}


subcommand_def!{
    /// Calculates neighbors for tiles
    pub(crate) struct CreateCoastline {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="100")]
        /// This number is used for generating points to make curvy coastlines. The higher the number, the smoother the curves.
        bezier_scale: f64,

        #[arg(long)]
        /// If true and the coastline or oceans layers already exist in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool,

        #[arg(long)]
        /// If true and the coastline layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_coastline: bool,

        #[arg(long)]
        /// If true and the oceans layer already exists in the file, they will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite_ocean: bool,



    }
}

impl Task for CreateCoastline {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::create_or_edit(self.target)?;

        target.with_transaction(|target| {

            progress.announce("Creating coastline");

            calculate_coastline(target, self.bezier_scale, self.overwrite || self.overwrite_coastline, self.overwrite || self.overwrite_ocean, &mut progress)
        })?;

        target.save(&mut progress)


    }
}