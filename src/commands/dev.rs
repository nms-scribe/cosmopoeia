use std::path::PathBuf;

use clap::Args;
use clap::Subcommand;
use rand::Rng;

use crate::commands::Task;
use crate::commands::TargetArg;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random::random_number_generator;
use crate::utils::extent::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::algorithms::random_points::PointGenerator;
use crate::progress::WatchableIterator;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::utils::point::ToGeometryCollection;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::random_points::load_points_layer;
use crate::algorithms::triangles::load_triangles_layer;
use crate::algorithms::tiles::load_tile_layer;
use crate::algorithms::naming::NamerSetSource;
use crate::algorithms::naming::NamerSet;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::culture_sets::CultureSetItem;
use crate::world_map::ElevationLimits;
use crate::command_def;
use crate::progress::ProgressObserver;
use crate::commands::ElevationSourceArg;
use crate::commands::ElevationLimitsArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteTilesArg;
use crate::commands::NamerArg;
use crate::world_map::TypedFeature;


subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    pub struct PointsFromHeightmap {

        #[clap(flatten)]
        pub heightmap_arg: ElevationSourceArg,

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        pub points: usize,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        pub overwrite: bool
    }
}

impl Task for PointsFromHeightmap {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {
        let source = RasterMap::open(self.heightmap_arg.source)?;
        let extent = source.bounds()?.extent();
        let mut target = WorldMap::create_or_edit(self.target_arg.target)?;
        let random = random_number_generator(&self.random_seed_arg);
        let generator = PointGenerator::new(random, extent, self.points);

        target.with_transaction(|transaction| {
            progress.announce("Generating random points");

            load_points_layer(transaction, self.overwrite, generator, progress)
        })?;

        target.save(progress)?;
        
        Ok(())
    }
}

subcommand_def!{
    /// Creates a random points vector layer given an extent
    pub struct PointsFromExtent {
        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(allow_hyphen_values=true)]
        pub west: f64,

        #[arg(allow_hyphen_values=true)]
        pub south: f64,

        #[arg(allow_hyphen_values=true)]
        pub north: f64,

        #[arg(allow_hyphen_values=true)]
        pub east: f64,

        #[arg(long)]
        /// The rough number of pixels horizontally separating each point [Default: a value that places about 10k points]
        pub spacing: Option<f64>,

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        pub points: usize,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for PointsFromExtent {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {
        let extent = Extent::new(self.west,self.south,self.east,self.north);
        let mut target = WorldMap::create_or_edit(self.target_arg.target)?;
        let random = random_number_generator(&self.random_seed_arg);
        let generator = PointGenerator::new(random, extent, self.points);
        
        target.with_transaction(|transaction| {
            progress.announce("Generating random points");

            load_points_layer(transaction, self.overwrite, generator, progress)
        })?;

        target.save(progress)?;

        Ok(())
    }
}

subcommand_def!{
    /// Creates delaunay triangles from a points layer
    pub struct TrianglesFromPoints {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        pub overwrite: bool
    }
}

impl Task for TrianglesFromPoints {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut target = WorldMap::edit(self.target_arg.target)?;

        let mut points = target.points_layer()?;
    
        let mut generator = DelaunayGenerator::new(points.read_features().map(|f| f.geometry()).to_geometry_collection(progress)?);
    
        progress.announce("Generating delaunay triangles");

        generator.start(progress)?;
    
        target.with_transaction(|transaction| {
            load_triangles_layer(transaction, self.overwrite, generator, progress)
        })?;

        target.save(progress)
        
        
    }
}

subcommand_def!{
    /// Creates voronoi tiles out of a delaunay triangles layer
    pub struct VoronoiFromTriangles {

        #[clap(flatten)]
        pub target_arg: TargetArg,

        #[clap(flatten)]
        pub heightmap_arg: ElevationSourceArg,

        #[clap(flatten)]
        pub elevation_limits_arg: ElevationLimitsArg,

        #[clap(flatten)]
        pub overwrite_tiles_arg: OverwriteTilesArg,

    }
}

impl Task for VoronoiFromTriangles {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let extent = {
            let source = RasterMap::open(self.heightmap_arg.source)?;
            source.bounds()?.extent()
        };

        let limits = ElevationLimits::new(self.elevation_limits_arg.min_elevation,self.elevation_limits_arg.max_elevation)?;

        let mut target = WorldMap::edit(self.target_arg.target)?;

        target.with_transaction(|transaction| {
            let mut triangles = transaction.edit_triangles_layer()?;
    
            let mut generator = VoronoiGenerator::new(triangles.read_features().map(|f| f.geometry()),extent)?;
    
            progress.announce("Create tiles from voronoi polygons");
        
            generator.start(progress)?;
        
            // I need to collect this because I can't borrow the transaction as mutable with an active iterator.
            #[allow(clippy::needless_collect)]
            let voronoi: Vec<_> = generator.watch(progress,"Copying voronoi.","Voronoi copied.").collect();
    
            load_tile_layer(transaction,&self.overwrite_tiles_arg,voronoi.into_iter(),&limits,progress)
        })?;

        target.save(progress)
    
    
    }
}


subcommand_def!{
    /// Tool for testing name generator data
    pub struct Namers {

        #[clap(flatten)]
        pub namer_arg: NamerArg,

        #[arg(long)]
        /// If this is set, text files loaded as namer_data will be parsed as markov seed lists. Otherwise, they will be list-picker generators.
        pub text_is_markov: bool,

        #[arg(long)]
        /// The name of a namer to generate from. If not specified, all namers will be tested.
        pub language: Option<String>,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[arg(long)]
        /// If true, the command will serialize the namer data into one JSON document rather than test the naming.
        pub write_json: bool,

    }
}


impl Task for Namers {


    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        fn test_namer<Random: Rng>(namers: &mut NamerSet, language: &String, rng: &mut Random) {
            let namer = namers.get_mut(Some(language)).expect("Someone called this function with a namer set that didn't contain the provided language key.");
            println!("language: {language}");
            println!("    name: {}",namer.make_name(rng));
            println!("   state: {}",namer.make_state_name(rng));
        
        }
        
        if self.write_json {
            let namers = NamerSetSource::from_files(self.namer_arg.namers)?;

            print!("{}",namers.to_json()?)

        } else {
            let mut random = random_number_generator(&self.random_seed_arg);
            let mut namers = NamerSet::load_from(self.namer_arg, &mut random, progress)?;

            if let Some(key) = self.language {
                test_namer(&mut namers, &key, &mut random)
            } else {
                let mut languages = namers.list_names();
                languages.sort(); // so the tests are reproducible.
                for language in languages {
                    test_namer(&mut namers, &language, &mut random)
                }
    
            }
    
        }


        Ok(())

    
    
    }
}



subcommand_def!{
    /// Tool for testing name generator data
    pub struct Cultures {

        /// Files to load culture data from, more than one may be specified to load multiple cultures into the set.
        pub culture_data: Vec<PathBuf>,

        #[clap(flatten)]
        pub namer_arg: NamerArg,

        #[arg(long)]
        /// If true, the command will serialize the namer data into one JSON document rather than test the naming.
        pub write_json: bool,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,



    }
}


impl Task for Cultures {


    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        fn test_culture(culture: &CultureSetItem) {
            println!("{}",culture.name());
        
        }

        let mut random = random_number_generator(&self.random_seed_arg);

        let mut loaded_namers = NamerSet::load_from(self.namer_arg, &mut random, progress)?;

        let cultures = CultureSet::from_files(&self.culture_data,&mut random,&mut loaded_namers)?;

        if self.write_json {
            print!("{}",cultures.to_json()?)

        } else {
            
            for culture in &cultures {
                test_culture(culture)
            }
    
    
        }


        Ok(())

    
    
    }
}


command_def!(
    #[command(disable_help_subcommand(true))]
    pub DevCommand {
        PointsFromHeightmap,
        PointsFromExtent,
        TrianglesFromPoints,
        VoronoiFromTriangles,
        Namers,
        Cultures
    }
);


subcommand_def!{
    /// Runs some tasks intended for testing and debugging
    #[command(hide=true)]
    pub struct Dev {
        #[command(subcommand)]
        pub command: DevCommand

    }
}

impl Task for Dev {
    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {
        self.command.run(progress)
    }
}
