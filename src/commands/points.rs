use std::path::PathBuf;

use clap::Args;
use rand::rngs::StdRng;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::utils::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::PointGenerator;

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevPointsFromHeightmap {
        // Path to the source height map to get extents from
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// The rough number of pixels horizontally separating each point [Default: a value that places about 10k points]
        spacing: Option<f64>,

        #[arg(long)]
        /// Seeds for the random number generator (up to 32), note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Vec<u8>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevPointsFromHeightmap {

    fn run(self) -> Result<(),CommandError> {
        let source = RasterMap::open(self.source)?;
        let extent = source.bounds()?.extent();
        let mut target = WorldMap::create_or_edit(self.target)?;
        let random = random_number_generator(self.seed);
        let mut progress = ConsoleProgressBar::new();
        let spacing = PointGenerator::<StdRng>::default_spacing_for_extent(self.spacing,&extent);
        let generator = PointGenerator::new(random, extent, spacing);
        
        target.load_points_layer(self.overwrite, generator, &mut Some(&mut progress))?;

        Ok(())
    }
}

subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevPointsFromExtent {
        #[arg(allow_hyphen_values=true)]
        west: f64,

        #[arg(allow_hyphen_values=true)]
        south: f64,

        #[arg(allow_hyphen_values=true)]
        north: f64,

        #[arg(allow_hyphen_values=true)]
        east: f64,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// The rough number of pixels horizontally separating each point [Default: a value that places about 10k points]
        spacing: Option<f64>,

        #[arg(long)]
        /// Seeds for the random number generator (up to 32), note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Vec<u8>,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevPointsFromExtent {

    fn run(self) -> Result<(),CommandError> {
        let extent = Extent::new(self.west,self.south,self.east,self.north);
        let mut target = WorldMap::create_or_edit(self.target)?;
        let random = random_number_generator(self.seed);
        let mut progress = ConsoleProgressBar::new();
        let spacing = PointGenerator::<StdRng>::default_spacing_for_extent(self.spacing,&extent);
        let generator = PointGenerator::new(random, extent, spacing);
        
        target.load_points_layer(self.overwrite, generator, &mut Some(&mut progress))?;

        Ok(())
    }
}

