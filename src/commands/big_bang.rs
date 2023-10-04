use clap::Args;
use rand::Rng;

use crate::subcommand_def;
use crate::commands::create::Source;
use crate::commands::Task;
use crate::commands::TargetArg;
use crate::progress::ProgressObserver;
use crate::errors::CommandError;
use crate::world_map::WorldMap;
use crate::algorithms::naming::NamerSet;
use crate::commands::create::LoadSource;
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
use crate::commands::TileCountArg;
use crate::commands::RandomSeedArg;
use crate::commands::OverwriteAllArg;
use crate::commands::BezierScaleArg;
use crate::commands::TemperatureRangeArg;
use crate::commands::WindsArg;
use crate::commands::PrecipitationArg;
use crate::commands::NamerArg;
use crate::commands::SizeVarianceArg;
use crate::commands::RiverThresholdArg;
use crate::commands::ExpansionFactorArg;
use crate::commands::CulturesGenArg;
use crate::commands::SubnationPercentArg;
use crate::commands::TownCountsArg;
use crate::commands::LakeBufferScaleArg;
use crate::utils::random::random_number_generator;


#[derive(Args)]
pub struct PrimitiveArgs {

    #[clap(flatten)]
    pub tile_count_arg: TileCountArg,

    #[clap(flatten)]
    pub temperature_arg: TemperatureRangeArg,

    #[clap(flatten)]
    pub wind_arg: WindsArg,

    #[clap(flatten)]
    pub precipitation_arg: PrecipitationArg,

    #[clap(flatten)]
    pub bezier_scale_arg: BezierScaleArg,

    #[clap(flatten)]
    pub lake_buffer_scale_arg: LakeBufferScaleArg,

    #[clap(flatten)]
    pub river_threshold_arg: RiverThresholdArg,

    #[clap(flatten)]
    pub size_variance_arg: SizeVarianceArg,

    #[clap(flatten)]
    pub expansion_factor_arg: ExpansionFactorArg,

    #[clap(flatten)]
    pub town_counts_arg: TownCountsArg,

    #[clap(flatten)]
    pub subnation_percent_arg: SubnationPercentArg,

    #[clap(flatten)]
    pub overwrite_all_arg: OverwriteAllArg,

}

subcommand_def!{
    /// Generates a world with all of the steps.
    pub struct BigBang {

        #[clap(flatten)]
        target_arg: TargetArg,

        #[clap(flatten)]
        namer_arg: NamerArg,

        #[clap(flatten)]
        pub cultures_arg: CulturesGenArg,
    
        #[clap(flatten)]
        pub random_seed_arg: RandomSeedArg,

        #[clap(flatten)]
        pub primitive_args: PrimitiveArgs,

        #[command(subcommand)]
        pub source: Source,


    }
}

impl Task for BigBang {

    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {

        let mut random = random_number_generator(&self.random_seed_arg);

        let mut loaded_namers = NamerSet::load_from(self.namer_arg, &mut random, progress)?;

        let loaded_source = self.source.load(&mut random, progress)?; 

        Self::run_default(&mut random,&self.primitive_args,&self.cultures_arg,&mut loaded_namers,loaded_source,self.target_arg,progress)

    }
}

impl BigBang {


    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(random: &mut Random, primitive_args: &PrimitiveArgs, cultures: &CulturesGenArg, namers: &mut NamerSet, loaded_source: LoadedSource, target_arg: TargetArg, progress: &mut Progress) -> Result<(), CommandError> {

        let mut target = WorldMap::create_or_edit(target_arg.target)?;

        Create::run_default(&primitive_args.tile_count_arg, &primitive_args.overwrite_all_arg.overwrite_tiles(), loaded_source, &mut target, random, progress)?;

        GenClimate::run_default(&primitive_args.temperature_arg, &primitive_args.wind_arg, &primitive_args.precipitation_arg, &mut target, progress)?;

        // FUTURE: If I don't do the next line, I get an error in the next command parts from SQLite that 'coastlines' table is locked. If I remove the 
        // algorithm immediately following, I get the same error for a different table instead. The previous algorithms don't even touch those items, and if the
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

        GenWater::run_default(&primitive_args.bezier_scale_arg, &primitive_args.lake_buffer_scale_arg, &primitive_args.overwrite_all_arg.overwrite_coastline(), &primitive_args.overwrite_all_arg.overwrite_ocean(), &primitive_args.overwrite_all_arg.overwrite_lakes(), &primitive_args.overwrite_all_arg.overwrite_rivers(), &mut target, progress)?;

        GenBiome::run_default(&primitive_args.overwrite_all_arg.overwrite_biomes(), &primitive_args.bezier_scale_arg, &mut target, progress)?;

        // The 'namer_set' here is not loaded, it's only used to verify that a namer exists for a culture while creating. Just to be clear, I'm not loading the namers twice, they are only loaded in `get_lookup_and_namers` below.
        GenPeople::run_default(&primitive_args.river_threshold_arg, cultures, namers, &primitive_args.size_variance_arg, &primitive_args.overwrite_all_arg.overwrite_cultures(), &primitive_args.expansion_factor_arg, &primitive_args.bezier_scale_arg, &mut target, random, progress)?;

        // CultureForNations implements everything that all the algorithms need.
        let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForNations>(progress)?;
    
        GenTowns::run_default(random, &culture_lookup, namers, &primitive_args.town_counts_arg, &primitive_args.river_threshold_arg, &primitive_args.overwrite_all_arg.overwrite_towns(), &mut target, progress)?;

        GenNations::run_default(random, &culture_lookup, namers, &primitive_args.size_variance_arg, &primitive_args.river_threshold_arg, &primitive_args.expansion_factor_arg, &primitive_args.bezier_scale_arg, &primitive_args.overwrite_all_arg.overwrite_nations(), &mut target, progress)?;

        GenSubnations::run_default(random, &culture_lookup, namers, &primitive_args.subnation_percent_arg, &primitive_args.overwrite_all_arg.overwrite_subnations(), &primitive_args.bezier_scale_arg, &mut target, progress)

    }
}