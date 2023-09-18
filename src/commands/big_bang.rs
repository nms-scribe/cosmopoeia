use std::path::PathBuf;

use clap::Args;
use rand::Rng;

use crate::subcommand_def;
use crate::commands::create::CreateSource;
use crate::commands::Task;
use crate::commands::TargetArg;
use crate::progress::ProgressObserver;
use crate::errors::CommandError;
use crate::world_map::WorldMap;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::naming::NamerSetSource;
use crate::algorithms::naming::NamerSet;
use crate::commands::create::LoadCreateSource;
use crate::commands::create::LoadedSource;
use crate::commands::create::Create;
use crate::commands::gen_climate::GenClimate;
use crate::commands::gen_water::GenWater;
use crate::commands::gen_biome::GenBiome;
use crate::commands::gen_people::GenPeople;
use crate::world_map::CultureForNations;
use crate::commands::gen_towns::GenTowns;
use crate::commands::gen_nations::GenNations;
use crate::commands::gen_subnations::GenSubnations;
use super::TileCountArg;
use super::RandomSeedArg;
use super::OverwriteAllArg;
use super::BezierScaleArg;


#[derive(Args)]
pub struct PrimitiveArgs {

    #[clap(flatten)]
    pub tile_count_arg: TileCountArg,

    /// The rough temperature (in celsius) at the equator
    #[arg(long,default_value="25",allow_hyphen_values=true)]
    pub equator_temp: i8,

    /// The rough temperature (in celsius) at the poles
    #[arg(long,default_value="-15",allow_hyphen_values=true)]
    pub polar_temp: i8,

    #[arg(long,default_value="225")]
    /// Wind direction above latitude 60 N
    pub north_polar_wind: u16,

    #[arg(long,default_value="45")]
    /// Wind direction from latitude 30 N to 60 N
    pub north_middle_wind: u16,

    #[arg(long,default_value="225")]
    /// Wind direction from the equator to latitude 30 N
    pub north_tropical_wind: u16,

    #[arg(long,default_value="315")]
    /// Wind direction from the equator to latitude 30 S
    pub south_tropical_wind: u16,

    #[arg(long,default_value="135")]
    /// Wind direction from latitude 30 S to 60 S
    pub south_middle_wind: u16,

    #[arg(long,default_value="315")]
    /// Wind direction below latitude 60 S
    pub south_polar_wind: u16,

    #[arg(long,default_value="100")]
    /// Amount of moisture on a scale of 0-500
    pub moisture_factor: u16,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[arg(long,default_value="2")]
    /// This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes.
    pub lake_buffer_scale: f64,

    #[arg(long,default_value="10")]
    /// A waterflow threshold above which the tile will count as a river
    pub river_threshold: f64,

    #[arg(long,default_value("10"))]
    /// The number of cultures to generate
    pub culture_count: usize,

    #[arg(long,default_value("1"))]
    /// A number, clamped to 0-10, which controls how much cultures and nations can vary in size
    pub size_variance: f64,

    #[arg(long,default_value("1"))]
    /// A number, usually ranging from 0.1 to 2.0, which limits how far cultures and nations will expand. The higher the number, the less neutral lands.
    pub limit_factor: f64,

    #[arg(long,default_value="20")]
    /// The number of national capitals to create
    pub capital_count: usize,

    #[arg(long)]
    /// The number of non-capital towns to create. If not specified, an appropriate number of towns will be guessed from population and map size.
    pub town_count: Option<usize>,

    #[arg(long,default_value("20"))]
    /// The percent of towns in each nation to use for subnations
    pub subnation_percentage: f64,

    #[clap(flatten)]
    pub overwrite_all_arg: OverwriteAllArg,

    #[arg(long)]
    /// The name generator to use for naming nations and towns in tiles without a culture, or one will be randomly chosen
    pub default_namer: Option<String>
    
}

subcommand_def!{
    /// Generates a world with all of the steps.
    pub struct BigBang {

        #[clap(flatten)]
        target_arg: TargetArg,

        #[arg(long,required(true))] 
        /// Files to load culture sets from, more than one may be specified to load multiple culture sets.
        pub cultures: Vec<PathBuf>,

        #[arg(long,required(true))]
        /// Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones.
        pub namers: Vec<PathBuf>,

        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub primitive_args: PrimitiveArgs,

        #[command(subcommand)]
        pub source: CreateSource,


    }
}

impl Task for BigBang {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = crate::utils::random_number_generator(self.random_seed_arg);

        let namer_set = NamerSetSource::from_files(self.namers)?;

        let mut loaded_namers = NamerSet::load_from(namer_set, self.primitive_args.default_namer.clone(), &mut random, progress)?;

        let culture_set = CultureSet::from_files(self.cultures,&mut random,&mut loaded_namers)?;

        let loaded_source = self.source.load(&mut random, progress)?; 

        Self::run_default(&mut random,self.primitive_args,culture_set,&mut loaded_namers,loaded_source,self.target_arg,progress)

    }
}

impl BigBang {


    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(random: &mut Random, primitive_args: PrimitiveArgs, cultures: CultureSet, namers: &mut NamerSet, loaded_source: LoadedSource, target_arg: TargetArg, progress: &mut Progress) -> Result<(), CommandError> {

        let mut target = WorldMap::create_or_edit(&target_arg.target)?;

        Create::run_default(primitive_args.tile_count_arg, primitive_args.overwrite_all_arg.overwrite_tiles(), loaded_source, &mut target, random, progress)?;

        let winds = [
            primitive_args.north_polar_wind as i32,
            primitive_args.north_middle_wind as i32,
            primitive_args.north_tropical_wind as i32,
            primitive_args.south_tropical_wind as i32,
            primitive_args.south_middle_wind as i32,
            primitive_args.south_polar_wind as i32
        ];

        GenClimate::run_default(primitive_args.equator_temp, primitive_args.polar_temp, winds, primitive_args.moisture_factor, &mut target, progress)?;

        // FUTURE: If I don't do the next line, I get an error in the next command parts from SQLite that 'coastlines' table is locked. If I remove the next
        // algorithm, I get the same error for a different table instead. The previous algorithms don't even touch those items, and if the
        // file already exists (which it did when I was running this error), 'create_or_edit' is the same as 'edit', so there isn't some
        // special case create locking going on.
        // - Maybe some future version of gdal or the gdal crate will fix this. If it does it's a simple matter of removing this line.
        // - I do not know if there's another way to fix it, but this was my first thought, and it works, and I don't want to go any further because I'm being triggered with memories of Windows 2000 DLL and ActiveX code integrations where this sort of thing was the only answer. Shudder.
        /* The specific error messages:
            ERROR 1: sqlite3_exec(DROP TABLE "rtree_coastlines_geom") failed: database table is locked
            ERROR 1: sqlite3_exec(DROP TABLE "coastlines") failed: database table is locked
            ERROR 1: sqlite3_exec(CREATE TABLE "coastlines" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON)) failed: table "coastlines" already exists
            ERROR 1: sqlite3_exec(CREATE VIRTUAL TABLE "rtree_coastlines_geom" USING rtree(id, minx, maxx, miny, maxy)) failed: table "rtree_coastlines_geom" already exists
            gdal: OGR method 'OGR_L_CreateFeature' returned error: '6'

         */
        let mut target = target.reedit()?;

        GenWater::run_default(&primitive_args.bezier_scale_arg, primitive_args.lake_buffer_scale, primitive_args.overwrite_all_arg.overwrite_coastline(), primitive_args.overwrite_all_arg.overwrite_ocean(), primitive_args.overwrite_all_arg.overwrite_lakes(), primitive_args.overwrite_all_arg.overwrite_rivers(), &mut target, progress)?;

        GenBiome::run_default(primitive_args.overwrite_all_arg.overwrite_biomes(), &primitive_args.bezier_scale_arg, &mut target, progress)?;

        // The 'namer_set' here is not loaded, it's only used to verify that a namer exists for a culture while creating. Just to be clear, I'm not loading the namers twice, they are only loaded in `get_lookup_and_namers` below.
        GenPeople::run_default(primitive_args.river_threshold, cultures, &namers, primitive_args.culture_count, primitive_args.size_variance, primitive_args.overwrite_all_arg.overwrite_cultures(), primitive_args.limit_factor, &primitive_args.bezier_scale_arg, &mut target, random, progress)?;

        // CultureForNations implements everything that all the algorithms need.
        let culture_lookup = target.cultures_layer()?.read_features().to_named_entities_index::<_,CultureForNations>(progress)?;
    
        GenTowns::run_default(random, &culture_lookup, namers, primitive_args.capital_count, primitive_args.town_count, primitive_args.river_threshold, primitive_args.overwrite_all_arg.overwrite_towns(), &mut target, progress)?;

        GenNations::run_default(random, &culture_lookup, namers, primitive_args.size_variance, primitive_args.river_threshold, primitive_args.limit_factor, &primitive_args.bezier_scale_arg, primitive_args.overwrite_all_arg.overwrite_nations(), &mut target, progress)?;

        GenSubnations::run_default(random, culture_lookup, namers, primitive_args.subnation_percentage, primitive_args.overwrite_all_arg.overwrite_subnations(), &primitive_args.bezier_scale_arg, &mut target, progress)

    }
}