use crate::entity;
use crate::typed_map::features::TypedFeature;
use crate::world_map::biome_layer::BiomeMatrix;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::biome_layer::BiomeSchema;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::world_map::fields::LakeType;
use crate::world_map::water_layers::LakeForBiomes;
use crate::world_map::fields::Grouping;
use crate::world_map::tile_layer::TileSchema;
use crate::world_map::tile_layer::TileFeature;
use crate::commands::OverwriteBiomesArg;
use crate::commands::OverrideBiomeCriteriaArg;
use crate::typed_map::fields::IdRef;

pub(crate) fn fill_biome_defaults<Progress: ProgressObserver>(target: &mut WorldMapTransaction, override_criteria: &OverrideBiomeCriteriaArg, overwrite_layer: &OverwriteBiomesArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut biomes = target.create_biomes_layer(overwrite_layer)?;

    let default_biomes = BiomeSchema::get_default_biomes(override_criteria);

    progress.start_known_endpoint(|| ("Writing biomes.",default_biomes.len()));

    for data in &default_biomes {

        _ = biomes.add_biome(data)?;
    }

    progress.finish(|| "Biomes written.");

    Ok(())
}

pub(crate) fn apply_biomes<Progress: ProgressObserver>(target: &mut WorldMapTransaction, biomes: &BiomeMatrix, progress: &mut Progress) -> Result<(), CommandError> {

    // we need a lake information map
    let mut lakes_layer = target.edit_lakes_layer()?;

    let lake_map = lakes_layer.read_features().into_entities_index::<_,LakeForBiomes>(progress)?;

    let mut tiles_layer = target.edit_tile_layer()?; 

    entity!(BiomeSource: Tile {
        #[get=false] fid: IdRef,
        #[get=false] temperature: f64,
        #[get=false] water_flow: f64,
        #[get=false] precipitation: f64,
        #[get=false] lake_id: Option<IdRef>,
        #[get=false] grouping: Grouping
    });

    let tiles = tiles_layer.read_features().into_entities_vec::<_,BiomeSource>(progress)?;

    for tile in tiles.iter().watch(progress,"Applying biomes.","Biomes applied.") {

        let biome = if tile.grouping.is_ocean() {
            biomes.ocean()
        } else if tile.temperature < biomes.glacier().1 {
            &biomes.glacier().0
        } else {
            // is it a wetland?
            if (tile.water_flow > biomes.wetland().1) || 
               matches!(tile.lake_id.as_ref().map(|id| lake_map.try_get(id).map(LakeForBiomes::type_)).transpose()?, Some(LakeType::Marsh)) {
                &biomes.wetland().0
            } else {
                // The original calculation favored deserts too much
                //let moisture_band = ((tile.precipitation/5.0).floor() as usize).min(4); // 0-4
                // FUTURE: A better climate modelling system, with less ambiguous precipitation units and seasonal values
                // would allow me to use the Koppen Climate system here, which would also change how biomes are defined.
                let moisture_band = if tile.precipitation < 1.0 {
                    0
                } else if tile.precipitation < 2.0 {
                    1
                } else {
                    let level = (tile.precipitation/20.0).floor() as usize;
                    if level <= 2 {
                        2
                    } else if level <= 3 {
                        3
                    } else {
                        4
                    }
                };

                let temperature_band = ((20.0 - tile.temperature).max(0.0).floor() as usize).min(25);
                &biomes.matrix()[moisture_band][temperature_band]
            }

    
        };

        let mut tile = tiles_layer.try_feature_by_id(&tile.fid)?;
        
        tile.set_biome(biome)?;

        tiles_layer.update_feature(tile)?;

    }


    Ok(())

}
