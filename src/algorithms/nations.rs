use core::cmp::Reverse;
use std::collections::HashSet;
use std::collections::HashMap;

use ordered_float::OrderedFloat;
use rand::Rng;
use priority_queue::PriorityQueue;
use prisma::Rgb;

use crate::world_map::TileForNationNormalize;
use crate::world_map::TownForNationNormalize;
use crate::world_map::BiomeSchema;
use crate::world_map::TileForNationExpand;
use crate::world_map::BiomeForNationExpand;
use crate::world_map::NationForPlacement;
use crate::world_map::NewNation;
use crate::world_map::CultureType;
use crate::world_map::TownForNations;
use crate::errors::CommandError;
use crate::algorithms::naming::NamerSet;
use crate::world_map::WorldMapTransaction;
use crate::world_map::CultureWithType;
use crate::world_map::CultureWithNamer;
use crate::world_map::NamedEntity;
use crate::world_map::CultureSchema;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::EntityLookup;
use crate::algorithms::colors::RandomColorGenerator;
use crate::commands::OverwriteNationsArg;
use crate::commands::SizeVarianceArg;
use crate::commands::RiverThresholdArg;
use crate::commands::ExpansionFactorArg;
use super::colors::Luminosity;

pub(crate) fn generate_nations<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut NamerSet, size_variance: &SizeVarianceArg, overwrite_layer: &OverwriteNationsArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut towns = target.edit_towns_layer()?;

    let mut nations = Vec::new();

    for town in towns.read_features().into_entities::<TownForNations>().watch(progress,"Reading towns.","Towns read.") {
        let (_,town) = town?;
        if town.is_capital {
            let culture = town.culture;
            let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;
            let namer = Culture::get_namer(culture_data, namers)?;
            let name = namer.make_state_name(rng);
            let type_ = culture_data.map(CultureWithType::type_).cloned().unwrap_or(CultureType::Generic);
            let center_tile_id = town.tile_id;
            let capital_town_id = town.fid;
            let expansionism = rng.gen_range(0.1f64..1.0f64).mul_add(size_variance.size_variance, 1.0);
            nations.push(NewNation {
                name,
                center_tile_id,
                culture,
                type_,
                expansionism,
                capital_town_id,
                color: Rgb::new(0,0,0)
            })

        }
    }

    let mut colors = RandomColorGenerator::new(None,Some(Luminosity::Light)).generate_colors(nations.len(), rng).into_iter();

    for nation in nations.iter_mut().watch(progress, "Assigning colors.", "Colors assigned") {
        nation.color = colors.next().expect("There should have been just as many colors as there were nations.");

    }

    let mut nations_layer = target.create_nations_layer(overwrite_layer)?;
    for nation in nations.into_iter().watch(progress,"Writing nations.","Nations written.") {
        _ = nations_layer.add_nation(&nation)?;
    }

    Ok(())
}

pub(crate) fn expand_nations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, river_threshold: &RiverThresholdArg, limit_factor: &ExpansionFactorArg, progress: &mut Progress) -> Result<(),CommandError> {


    let nations = target.edit_nations_layer()?.read_features().into_entities_vec::<_,NationForPlacement>(progress)?;

    let biome_map = target.edit_biomes_layer()?.read_features().into_named_entities_index::<_,BiomeForNationExpand>(progress)?;

    let mut tiles = target.edit_tile_layer()?;

    // we're working with a tile map, and completely overwriting whatever is there.
    let mut tile_map = tiles.read_features().into_entities_index::<_,TileForNationExpand>(progress)?;

    // priority queue keeps tasks sorted by priority
    // Since I need to go for the least priorities first, I need the double queue to get pop_min
    let mut queue = PriorityQueue::new();

    // empty hashmap of tile ids
    let mut costs = HashMap::new();

    let mut capitals = HashSet::new();

    let max_expansion_cost = OrderedFloat::from((tiles.feature_count() as f64 / 2.0) * limit_factor.expansion_factor);

    for nation in nations {

        // place the nation center
        let tile = tile_map.try_get_mut(&nation.center_tile_id)?;
        tile.nation_id = Some(nation.fid);

        _ = costs.insert(nation.center_tile_id, OrderedFloat::from(1.0));

        _ = capitals.insert(nation.center_tile_id);

        // add the tile to the queue for work.
        _ = queue.push((nation.center_tile_id,nation,tile.biome.clone()), Reverse(OrderedFloat::from(0.0)));

    }

    let mut queue = queue.watch_queue(progress, "Expanding cultures.", "Cultures expanded.");

    while let Some(((tile_id, nation, nation_biome), priority)) = queue.pop() {

        let mut place_nations = Vec::new();

    
        let tile = tile_map.try_get(&tile_id)?;

        for (neighbor_id,_) in &tile.neighbors {
        
            if capitals.contains(neighbor_id) {
                continue; // don't overwrite capital cells
            }

            let neighbor = tile_map.try_get(neighbor_id)?;

            let culture_cost = if tile.culture == neighbor.culture {-9.0} else { 100.0 };

            let population_cost = if neighbor.grouping.is_water() { 
                0.0
            } else if neighbor.habitability > 0.0 {
                (20.0 - neighbor.habitability).max(0.0)
            } else {
                5000.0
            };

            let neighbor_biome = biome_map.try_get(&neighbor.biome)?;

            let biome_cost = get_biome_cost(&nation_biome,neighbor_biome,&nation.type_);

            let height_cost = get_height_cost(neighbor, &nation.type_);

            let river_cost = get_river_cost(neighbor, river_threshold.river_threshold, &nation.type_);

            let shore_cost = get_shore_cost(neighbor, &nation.type_);

            let cell_cost = OrderedFloat::from((culture_cost + population_cost + biome_cost + height_cost + river_cost + shore_cost).max(0.0)) / nation.expansionism;

            let total_cost = priority.0 + OrderedFloat::from(10.0) + cell_cost;

            if total_cost <= max_expansion_cost {

                // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                // then I can place or replace the culture with this one. This will remove cultures that were previously
                // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                // may still be spreading, don't worry).
                let replace_nation = if let Some(neighbor_cost) = costs.get(neighbor_id) {
                    &total_cost < neighbor_cost
                } else {
                    true
                };

                if replace_nation {
                    if !neighbor.grouping.is_ocean() { 
                        place_nations.push((*neighbor_id,nation.fid));
                        // even if we don't place the culture, because people can't live here, it will still spread.
                    }
                    _ = costs.insert(*neighbor_id, total_cost);

                    queue.push((*neighbor_id, nation.clone(), nation_biome.clone()), Reverse(total_cost));

                } // else we can't expand into this tile, and this line of spreading ends here.
            } else {
                // else we can't make it into this tile, so give up.    

            }


        }

        for (place_tile_id,nation_id) in place_nations {
            let place_tile = tile_map.try_get_mut(&place_tile_id)?;
            place_tile.nation_id = Some(nation_id);
        }


    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing nations.","Nations written.") {

        let mut feature = tiles.try_feature_by_id(*fid)?;

        feature.set_nation_id(tile.nation_id)?;

        tiles.update_feature(feature)?;

    }


    Ok(())
}

pub(crate) const fn get_shore_cost(neighbor: &TileForNationExpand, culture_type: &CultureType) -> f64 {
    match culture_type {
        CultureType::Lake => match neighbor.shore_distance {
            2 | 1 => 0.0, 
            ..=-2 | 0 | 2.. => 100.0, // penalty for the mainland 
            -1 => 0.0,
        },
        CultureType::Naval => match neighbor.shore_distance {
            1 => 0.0,
            2 => 30.0, // penalty for mainland 
            ..=-2 | 0 | 2.. => 100.0,  // penalty for mainland 
            -1 => 0.0,
        },
        CultureType::Nomadic => match neighbor.shore_distance {
            1 => 60.0, // larger penalty for reaching the coast
            2 => 30.0, // penalty for approaching the coast
            -1 | ..=-2 | 0 | 2.. => 0.0, 
        },
        CultureType::Generic  => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            -1 | ..=-2 | 0 | 2.. => 0.0, 
        },
        CultureType::River => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            -1 | ..=-2 | 0 | 2.. => 0.0, 
        },
        CultureType::Hunting => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            -1 | ..=-2 | 0 | 2.. => 0.0,
        },
        CultureType::Highland => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            -1 | ..=-2 | 0 | 2.. => 0.0, 
        },
    }

}

pub(crate) fn get_river_cost(neighbor: &TileForNationExpand, river_threshold: f64, culture_type: &CultureType) -> f64 {
    match culture_type {
        CultureType::River => if neighbor.water_flow > river_threshold {
            0.0
        } else {
            // they want to stay near rivers
            100.0
        },
        CultureType::Generic |
        CultureType::Lake |
        CultureType::Naval |
        CultureType::Nomadic |
        CultureType::Hunting |
        CultureType::Highland => if neighbor.water_flow <= river_threshold {
            0.0 // no penalty for non-rivers
        } else {
            // penalty based on flowage
            (neighbor.water_flow / 10.0).clamp(20.0, 100.0)
        },
    }
}

pub(crate) const fn get_height_cost(neighbor: &TileForNationExpand, culture_type: &CultureType) -> f64 {
    // This is similar to the way cultures work, but not exactly.
    match culture_type {
        CultureType::Lake => if neighbor.lake_id.is_some() {
            // low lake crossing penalty for lake cultures
            10.0
        } else if neighbor.grouping.is_water() {
            // general sea/lake crossing penalty
            1000.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            2200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            300.0
        } else {
            0.0
        },
        CultureType::Naval => if neighbor.grouping.is_water() {
            // low water crossing penalty
            300.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            2200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            300.0
        } else {
            0.0
        },
        CultureType::Highland => if neighbor.grouping.is_water() {
            // general sea/lake corssing penalty
            1000.0
        } else if neighbor.elevation_scaled < 62 {
            // smaller but still big penalty for hills
            1100.0
        } else {
            // no penalty in highlands
            0.0
        },
        CultureType::Nomadic |
        CultureType::Generic |
        CultureType::River |
        CultureType::Hunting => if neighbor.grouping.is_water() {
            // general sea/lake corssing penalty
            1000.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            2200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            300.0
        } else {
            0.0
        }
    }
}

pub(crate) fn get_biome_cost(culture_biome: &String, neighbor_biome: &BiomeForNationExpand, culture_type: &CultureType) -> f64 {
    // This is very similar to the one for cultures, but not exactly.

    // FUTURE: I need a way to make this more configurable...
    const FOREST_BIOMES: [&str; 5] = [BiomeSchema::TROPICAL_SEASONAL_FOREST, BiomeSchema::TEMPERATE_DECIDUOUS_FOREST, BiomeSchema::TROPICAL_RAINFOREST, BiomeSchema::TEMPERATE_RAINFOREST, BiomeSchema::TAIGA];


    if culture_biome == &neighbor_biome.name {
        // tiny penalty for native biome
        10.0
    } else {
        (match culture_type {
            CultureType::Hunting => neighbor_biome.movement_cost * 2, // non-native biome penalty for hunters
            CultureType::Nomadic => if FOREST_BIOMES.contains(&neighbor_biome.name.as_str()) {
                // penalty for forests
                neighbor_biome.movement_cost * 3
            } else {
                neighbor_biome.movement_cost
            },
            CultureType::Generic |
            CultureType::Lake |
            CultureType::Naval |
            CultureType::River |
            CultureType::Highland => neighbor_biome.movement_cost,
        }) as f64

    }

}

pub(crate) fn normalize_nations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let town_index = target.edit_towns_layer()?.read_features().into_entities_index::<_,TownForNationNormalize>(progress)?;

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tile_list = Vec::new();
    let tile_map = tiles_layer.read_features().into_entities_index_for_each::<_,TileForNationNormalize,_>(|fid,_| {
        tile_list.push(*fid);
        Ok(())
    }, progress)?;

    for tile_id in tile_list.into_iter().watch(progress,"Normalizing nations.","Nations normalized.") {
        let tile = tile_map.try_get(&tile_id)?;

        if tile.grouping.is_water() || tile.town_id.is_some() {
            continue; // don't overwrite
        }

        let mut dont_overwrite = false;
        let mut adversaries = HashMap::new();
        let mut adversary_count = 0;
        let mut buddy_count = 0;
        for (neighbor_id,_) in &tile.neighbors {

            let neighbor = tile_map.try_get(neighbor_id)?;

            if let Some(town_id) = neighbor.town_id {
                let town = town_index.try_get(&(town_id))?;
                if town.is_capital {
                    dont_overwrite = true; // don't overwrite near capital
                    break;
                }
            }

            if !neighbor.grouping.is_water() {
                if neighbor.nation_id == tile.nation_id {
                    buddy_count += 1;
                } else {
                    if let Some(count) = adversaries.get(&neighbor.nation_id) {
                        _ = adversaries.insert(neighbor.nation_id, count + 1);
                    } else {
                        _ = adversaries.insert(neighbor.nation_id, 1);
                    };
                    adversary_count += 1;
                }
            }

        }

        if dont_overwrite {
            continue;
        }

        if adversary_count < 2 {
            continue;
        }

        if buddy_count > 2 {
            continue;
        }

        if adversaries.len() < buddy_count {
            continue;
        }

        if let Some((worst_adversary,count)) = adversaries.into_iter().max_by_key(|(_,count)| *count).map(|(adversary,count)| (adversary,count)) {
            if count > buddy_count {
                let mut change_tile = tiles_layer.try_feature_by_id(tile_id)?;
                change_tile.set_nation_id(worst_adversary)?;
                tiles_layer.update_feature(change_tile)?    
            }

        }

    }


    Ok(()) 
}
