use crate::entity;
use crate::world_map::TypedFeature;
use crate::world_map::BiomeMatrix;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::BiomeSchema;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::world_map::LakeType;
use crate::world_map::LakeForBiomes;
use crate::world_map::Grouping;
use crate::world_map::TileSchema;
use crate::commands::OverwriteBiomesArg;

pub(crate) fn fill_biome_defaults<Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: &OverwriteBiomesArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut biomes = target.create_biomes_layer(overwrite_layer)?;

    let default_biomes = BiomeSchema::get_default_biomes();

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

    // based on AFMG algorithm

    entity!(BiomeSource: Tile {
        fid: u64,
        temperature: f64,
        water_flow: f64,
        precipitation: f64,
        lake_id: Option<u64>,
        grouping: Grouping
    });

    let tiles = tiles_layer.read_features().into_entities_vec::<_,BiomeSource>(progress)?;

    for tile in tiles.iter().watch(progress,"Applying biomes.","Biomes applied.") {

        let biome = if tile.grouping.is_ocean() {
            biomes.ocean.clone()
        } else if tile.temperature < -5.0 {
            biomes.glacier.clone()
        } else {
            // is it a wetland?
            if (tile.water_flow > 400.0) || 
               matches!(tile.lake_id.map(|id| lake_map.try_get(&(id)).map(|l| &l.type_)).transpose()?, Some(LakeType::Marsh)) {
                biomes.wetland.clone()
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
                biomes.matrix[moisture_band][temperature_band].clone()
            }

    
        };

        if let Some(mut tile) = tiles_layer.feature_by_id(tile.fid) {

            tile.set_biome(&biome)?;

            tiles_layer.update_feature(tile)?;

        }

    }


    Ok(())

}
