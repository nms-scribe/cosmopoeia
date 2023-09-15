use crate::entity;
use crate::world_map::TypedFeature;
use crate::world_map::TileFeature;
use crate::world_map::Entity;
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

pub(crate) fn fill_biome_defaults<Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    let mut biomes = target.create_biomes_layer(overwrite_layer)?;

    let default_biomes = BiomeSchema::get_default_biomes();

    progress.start_known_endpoint(|| ("Writing biomes.",default_biomes.len()));

    for data in &default_biomes {
        biomes.add_biome(data)?;
    }

    progress.finish(|| "Biomes written.");

    Ok(())
}

pub(crate) fn apply_biomes<Progress: ProgressObserver>(target: &mut WorldMapTransaction, biomes: BiomeMatrix, progress: &mut Progress) -> Result<(), CommandError> {

    // we need a lake information map
    let mut lakes_layer = target.edit_lakes_layer()?;

    let lake_map = lakes_layer.read_features().to_entities_index::<_,LakeForBiomes>(progress)?;

    let mut tiles_layer = target.edit_tile_layer()?; 

    // based on AFMG algorithm

    entity!(BiomeSource: Tile {
        fid: u64,
        temperature: f64,
        elevation_scaled: i32,
        water_flow: f64,
        // TODO: Why am I initializing it like this? That should be the default initialization, no?
        lake_id: Option<u64> = |feature: &TileFeature| feature.lake_id(),
        grouping: Grouping
    });

    let tiles = tiles_layer.read_features().to_entities_vec::<_,BiomeSource>(progress)?;

    for tile in tiles.iter().watch(progress,"Applying biomes.","Biomes applied.") {

        let biome = if !tile.grouping.is_ocean() {
            if tile.temperature < -5.0 {
                biomes.glacier.clone()
            } else {
                let water_flow_scaled = tile.water_flow;
                // is it a wetland?
                if (tile.temperature > -2.0) && // no wetlands in colder environments... that seems odd and unlikely (Alaska is full of wetlands)
                   // FUTURE: AFMG assumed that if the land was below 25 it was near the coast. That seems inaccurate and I'm not sure what the point of
                   // that is: it requires *more* water to make the coast a wetland? Maybe the problem is basing it off of waterflow instead of precipitation.
                   (((water_flow_scaled > 40.0) && (tile.elevation_scaled < 25)) ||
                    ((water_flow_scaled > 24.0) && (tile.elevation_scaled > 24) && (tile.elevation_scaled < 60))) {
                    biomes.wetland.clone()
                } else if let Some(LakeType::Marsh) = tile.lake_id.and_then(|id| Some(lake_map.try_get(&(id as u64)).map(|l| &l.type_))).transpose()? {
                    biomes.wetland.clone()
                } else {
                    let moisture_band = ((water_flow_scaled/5.0).floor() as usize).min(4); // 0-4
                    // Math.min(Math.max(20 - temperature, 0), 25)
                    let temperature_band = ((20.0 - tile.temperature).max(0.0).floor() as usize).min(25);
                    biomes.matrix[moisture_band][temperature_band].clone()
                }

          
            }

        } else {
            biomes.ocean.to_owned()
        };

        if let Some(mut tile) = tiles_layer.feature_by_id(&tile.fid) {

            tile.set_biome(&biome)?;

            tiles_layer.update_feature(tile)?;

        }

    }

    Ok(())

}
