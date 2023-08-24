use std::path::PathBuf;
use std::io::stdout;

use clap::Args;
use rand::Rng;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::utils::random_number_generator;
use crate::utils::Extent;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ConsoleProgressBar;
use crate::algorithms::random_points::PointGenerator;
use crate::progress::WatchableIterator;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::utils::ToGeometryCollection;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::random_points::load_points_layer;
use crate::algorithms::triangles::load_triangles_layer;
use crate::algorithms::tiles::load_tile_layer;
use crate::world_map::NewTile;
use crate::algorithms::naming::NamerSet;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::culture_sets::CultureSource;


subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub(crate) struct DevPointsFromHeightmap {
        // Path to the source height map to get extents from
        source: PathBuf,

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        points: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

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
        let generator = PointGenerator::new(random, extent, self.points);

        target.with_transaction(|target| {
            progress.announce("Generating random points");

            load_points_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)?;
        
        Ok(())
    }
}

subcommand_def!{
    /// Creates a random points vector layer given an extent
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

        #[arg(long,default_value="10000")]
        /// The rough number of pixels to generate for the image
        points: usize,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

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
        let generator = PointGenerator::new(random, extent, self.points);
        
        target.with_transaction(|target| {
            progress.announce("Generating random points");

            load_points_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)?;

        Ok(())
    }
}

subcommand_def!{
    /// Creates delaunay triangles from a points layer
    #[command(hide=true)]
    pub(crate) struct DevTrianglesFromPoints {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevTrianglesFromPoints {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let mut target = WorldMap::edit(self.target)?;

        let mut points = target.points_layer()?;
    
        let mut generator = DelaunayGenerator::new(points.read_geometries().to_geometry_collection(&mut progress)?);
    
        progress.announce("Generating delaunay triangles");

        generator.start(&mut progress)?;
    
        target.with_transaction(|target| {
            load_triangles_layer(target, self.overwrite, generator, &mut progress)
        })?;

        target.save(&mut progress)
        
        
    }
}

subcommand_def!{
    /// Creates voronoi tiles out of a delaunay triangles layer
    #[command(hide=true)]
    pub(crate) struct DevVoronoiFromTriangles {

        /// The path to the world map GeoPackage file
        target: PathBuf,

        #[arg(long)]
        extents: PathBuf,

        #[arg(long)]
        /// If true and the layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists.
        overwrite: bool
    }
}

impl Task for DevVoronoiFromTriangles {

    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        let extent = {
            let source = RasterMap::open(self.extents)?;
            source.bounds()?.extent()
        };

        let mut target = WorldMap::edit(self.target)?;

        let mut triangles = target.triangles_layer()?;
    
        let mut generator = VoronoiGenerator::new(triangles.read_geometries(),extent)?;

        progress.announce("Create tiles from voronoi polygons");
    
        generator.start(&mut progress)?;
    
        let voronoi: Vec<Result<NewTile,CommandError>> = generator.watch(&mut progress,"Copying voronoi.","Voronoi copied.").collect();

        target.with_transaction(|target| {
            load_tile_layer(target,self.overwrite,voronoi.into_iter(),&mut progress)
        })?;

        target.save(&mut progress)
    
    
    }
}


subcommand_def!{
    /// Tool for testing name generator data
    #[command(hide=true)]
    pub(crate) struct DevNamers {

        /// Files to load namer-data from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        namer_data: Vec<PathBuf>,

        #[arg(long)]
        // if true, the namer will load the defaults before any of the passed files.
        defaults: bool,

        #[arg(long)]
        /// If this is set, text files loaded as namer_data will be parsed as markov seed lists. Otherwise, they will be list-picker generators.
        text_is_markov: bool,

        #[arg(long)]
        /// The name of a namer to generate from. If not specified, all namers will be tested.
        language: Option<String>,

        #[arg(long)]
        /// Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt.
        seed: Option<u64>,

        #[arg(long)]
        /// If true, the command will serialize the namer data into one JSON document rather than test the naming.
        write_json: bool,

        #[arg(long)]
        /// If true, the command will serialize the namer data into a "deflated" JSON document rather than test the naming.
        write_deflated: bool


    }
}


impl Task for DevNamers {


    fn run(self) -> Result<(),CommandError> {

        let mut progress = ConsoleProgressBar::new();

        fn test_namer<Random: Rng>(namers: &mut NamerSet, language: &String, progress: &mut ConsoleProgressBar, rng: &mut Random) {
            let mut namer = namers.load_one(language,progress).unwrap();
            println!("language: {language}");
            println!("    name: {}",namer.make_name(rng));
            println!("   state: {}",namer.make_state_name(rng));
        
        }
        
        let mut namers = NamerSet::from_files(self.namer_data, self.defaults)?;

        if self.write_deflated {
            namers.to_deflated_json(stdout())?;

        } else if self.write_json {
            print!("{}",namers.to_json()?)

        } else {
            let mut random = random_number_generator(self.seed);

            if let Some(key) = self.language {
                test_namer(&mut namers, &key, &mut progress, &mut random)
            } else {
                let mut languages = namers.list_names();
                languages.sort(); // so the tests are reproducible.
                for language in languages {
                    test_namer(&mut namers, &language, &mut progress, &mut random)
                }
    
            }
    
        }


        Ok(())

    
    
    }
}



subcommand_def!{
    /// Tool for testing name generator data
    #[command(hide=true)]
    pub(crate) struct DevCultures {

        /// Files to load culture data from, more than one may be specified to load multiple cultures into the set.
        culture_data: Vec<PathBuf>,

        #[arg(long)]
        /// If true, the command will serialize the namer data into one JSON document rather than test the naming.
        write_json: bool,


    }
}


impl Task for DevCultures {


    fn run(self) -> Result<(),CommandError> {

        fn test_culture(culture: &CultureSource) {
            println!("{}",culture.name());
        
        }
        
        let mut cultures = CultureSet::empty();
        for file in self.culture_data {
            cultures.extend_from_file(file)?;
        }

        if self.write_json {
            print!("{}",cultures.to_json()?)

        } else {
            
            for culture in &cultures {
                test_culture(&culture)
            }
    
    
        }


        Ok(())

    
    
    }
}
