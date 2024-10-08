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
use crate::world_map::culture_layer::CultureForNations;
use crate::commands::gen_towns::GenTowns;
use crate::commands::gen_nations::GenNations;
use crate::commands::gen_subnations::GenSubnations;
use crate::commands::TileCountArg;
use crate::commands::WorldShapeArg;
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
use crate::commands::OverrideBiomeCriteriaArg;
use crate::utils::random::random_number_generator;


#[derive(Args)]
pub struct PrimitiveArgs {

    #[clap(flatten)]
    pub tile_count: TileCountArg,

    #[clap(flatten)]
    pub world_shape: WorldShapeArg,

    #[clap(flatten)]
    pub temperature: TemperatureRangeArg,

    #[clap(flatten)]
    pub wind: WindsArg,

    #[clap(flatten)]
    pub precipitation: PrecipitationArg,

    #[clap(flatten)]
    pub bezier_scale: BezierScaleArg,

    #[clap(flatten)]
    pub lake_buffer_scale: LakeBufferScaleArg,

    #[clap(flatten)]
    pub river_threshold: RiverThresholdArg,

    #[clap(flatten)]
    pub override_biome_criteria: OverrideBiomeCriteriaArg,

    #[clap(flatten)]
    pub size_variance: SizeVarianceArg,

    #[clap(flatten)]
    pub expansion_factor: ExpansionFactorArg,

    #[clap(flatten)]
    pub town_counts: TownCountsArg,

    #[clap(flatten)]
    pub subnation_percent: SubnationPercentArg,

    #[clap(flatten)]
    pub overwrite_all: OverwriteAllArg,

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

        Self::run_default(&mut random,&self.primitive_args,&self.cultures_arg,&mut loaded_namers,loaded_source,&self.target_arg,progress)

    }
}

impl BigBang {


    pub(crate) fn run_default<Random: Rng, Progress: ProgressObserver>(random: &mut Random, primitive_args: &PrimitiveArgs, cultures: &CulturesGenArg, namers: &mut NamerSet, loaded_source: LoadedSource, target_arg: &TargetArg, progress: &mut Progress) -> Result<(), CommandError> {

        let mut target = WorldMap::create_or_edit(&target_arg.target)?;

        Create::run_default(&primitive_args.tile_count, &primitive_args.world_shape, &primitive_args.overwrite_all.overwrite_tiles(), loaded_source, &mut target, random, progress)?;

        GenClimate::run_default(&primitive_args.temperature, &primitive_args.wind, &primitive_args.precipitation, &mut target, progress)?;

        GenWater::run_default(&primitive_args.bezier_scale, &primitive_args.lake_buffer_scale, &primitive_args.overwrite_all.overwrite_coastline(), &primitive_args.overwrite_all.overwrite_ocean(), &primitive_args.overwrite_all.overwrite_lakes(), &primitive_args.overwrite_all.overwrite_rivers(), &mut target, progress)?;

        GenBiome::run_default(&primitive_args.override_biome_criteria,&primitive_args.overwrite_all.overwrite_biomes(), &primitive_args.bezier_scale, &mut target, progress)?;

        // The 'namer_set' here is not loaded, it's only used to verify that a namer exists for a culture while creating. Just to be clear, I'm not loading the namers twice, they are only loaded in `get_lookup_and_namers` below.
        GenPeople::run_default(&primitive_args.river_threshold, cultures, namers, &primitive_args.size_variance, &primitive_args.overwrite_all.overwrite_cultures(), &primitive_args.expansion_factor, &primitive_args.bezier_scale, &mut target, random, progress)?;

        // CultureForNations implements everything that all the algorithms need.
        let culture_lookup = target.cultures_layer()?.read_features().into_named_entities_index::<_,CultureForNations>(progress)?;
    
        GenTowns::run_default(random, &culture_lookup, namers, &primitive_args.town_counts, &primitive_args.river_threshold, &primitive_args.overwrite_all.overwrite_towns(), &mut target, progress)?;

        GenNations::run_default(random, &culture_lookup, namers, &primitive_args.size_variance, &primitive_args.river_threshold, &primitive_args.expansion_factor, &primitive_args.bezier_scale, &primitive_args.overwrite_all.overwrite_nations(), &mut target, progress)?;

        GenSubnations::run_default(random, &culture_lookup, namers, &primitive_args.subnation_percent, &primitive_args.overwrite_all.overwrite_subnations(), &primitive_args.bezier_scale, &mut target, progress)

    }
}