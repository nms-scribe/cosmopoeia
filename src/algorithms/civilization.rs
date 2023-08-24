use std::collections::HashSet;
use std::collections::HashMap;
use std::cmp::Reverse;

use rand::Rng;
use rand_distr::Normal;
use rand::distributions::Distribution;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::WorldMapTransaction;
use crate::world_map::NamedCulture;
use crate::errors::CommandError;
use crate::algorithms::naming::Namer;
use crate::world_map::TileForTowns;
use crate::utils::point_finder::PointFinder;
use crate::world_map::NewTown;
use crate::world_map::TilesLayer;
use crate::utils::Extent;
use crate::world_map::TypedFeature;
use crate::world_map::CultureWithNamer;
use crate::world_map::CultureWithType;
use crate::world_map::TownForNations;
use crate::world_map::NewNation;
use crate::world_map::CultureType;
use crate::world_map::NationForPlacement;
use crate::world_map::BiomeForNationExpand;
use crate::world_map::TileForNationExpand;
use crate::world_map::TileFeature;
use crate::world_map::BiomeFeature;


struct ScoredTileForTowns {
    tile: TileForTowns,
    capital_score: OrderedFloat<f64>,
    town_score: OrderedFloat<f64>
}

pub(crate) fn generate_towns<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedCulture<'culture> + CultureWithNamer>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &HashMap<String,Culture>, namers: &mut HashMap<String,Namer>, default_namer: &str, capital_count: usize, town_count: Option<usize>, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    // a lot of this is ported from AFMG

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tiles = gather_tiles_for_towns(rng, &mut tiles_layer, progress)?;

    let extent = tiles_layer.get_extent()?;

    let (capitals, capitals_finder) = generate_capitals(&mut tiles, &extent, capital_count, progress);

    let towns = place_towns(rng, &mut tiles, &extent, town_count, tiles_layer.feature_count(), &capitals_finder, progress)?;

    // write the towns

    let mut towns_layer = target.create_towns_layer(overwrite_layer)?;

    let mut placed_towns = HashMap::new(); 
    for town in capitals.into_iter().chain(towns.into_iter()).watch(progress,"Writing towns.","Towns written.") {
        let (ScoredTileForTowns{tile,..},is_capital) = town;
        let culture = tile.culture;
        let namer = get_namer(culture.as_ref().and_then(|c| culture_lookup.get(c)), namers, default_namer)?;
        let name = namer.make_name(rng);
        let fid = towns_layer.add_town(NewTown {
            geometry: tile.site.create_geometry()?,
            name,
            culture,
            is_capital,
            tile_id: tile.fid as i64,
            grouping_id: tile.grouping_id
        })?;
        placed_towns.insert(tile.fid,fid); 
    }

    // even though we have the town locations indicated in the towns layer, there are going to be occasions
    // where I want to easily figure out if a tile has a town, so write that there as well.
    // FUTURE: If the user re-writes the towns with a different seed, there's going to be erroneous data in tiles.
    // Maybe delete it.
    let mut tiles_layer = target.edit_tile_layer()?;
    // I have to update all tiles, otherwise we might have erroneous data from a previous run.
    let tiles: Vec<u64> = tiles_layer.read_features().watch(progress,"Reading tiles.","Tiles read.").map(|f| f.fid()).collect::<Result<Vec<_>,_>>()?;
    for fid in tiles {
        let town = placed_towns.get(&fid);
        let mut tile = tiles_layer.try_feature_by_id(&fid)?;
        tile.set_town_id(town.map(|n| *n as i64))?;
        tiles_layer.update_feature(tile)?;
    }

    Ok(())
}

fn get_namer<'namers, Culture: CultureWithNamer>(culture: Option<&Culture>, namers: &'namers mut HashMap<String, Namer>, default_namer: &str) -> Result<&'namers mut Namer, CommandError> {
    let namer = if let Some(namer) = culture.map(|culture| culture.namer()) {
        namer
    } else {
        default_namer
    };
    let namer = namers.get_mut(namer).ok_or_else(|| CommandError::UnknownNamer(namer.to_owned()))?;
    Ok(namer)
}

fn place_towns<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, tiles: &mut Vec<ScoredTileForTowns>, extent: &Extent, town_count: Option<usize>, total_tiles_count: usize, capitals_finder: &PointFinder, progress: &mut Progress) -> Result<Vec<(ScoredTileForTowns, bool)>,CommandError> {
    let mut towns_finder;
    let mut town_cultures;
    let mut towns;

    let town_count = if let Some(town_count) = town_count {
        if town_count > tiles.len() {
            let reduced_town_count = tiles.len();
            if tiles.len() == 0 {
                progress.warning(|| "There aren't enough populated cells left to generate any towns.")
            } else {
                progress.warning(|| format!("There aren't enough populated cells to generate the requested number of towns. Only {} towns will be generated.",reduced_town_count))
            }
            reduced_town_count
        } else {
            town_count
        }
    } else {
        tiles.len() / 5 / ((total_tiles_count / 10000) as f64).powf(0.8).round() as usize
    };

    let mut spacing = (extent.width + extent.height) / 150.0 / ((town_count as f64).powf(0.7)/66.0);
    let town_spacing_normal = Normal::new(1.0f64,0.3f64).unwrap();
    // if this fails then it's a programming error, I'm pretty certain.

    macro_rules! reset_town_search {
        () => {
            towns_finder = PointFinder::fill_from(&capitals_finder,town_count)?;
            towns = vec![];
            town_cultures = HashSet::new();
            tiles.sort_by_key(|ScoredTileForTowns{town_score,..}| std::cmp::Reverse(*town_score));
        };
    }

    reset_town_search!();


    // we have to do this several times, adjusting the spacing as necessary
    loop {
        // can't use a for loop, because the range changes
        let i = 0;
        progress.start_known_endpoint(|| (format!("Placing towns at spacing {}",spacing),town_count));
        while (i < tiles.len()) && (towns.len() < town_count) {
            let entry = &tiles[i];
            let s = spacing * town_spacing_normal.sample(rng).clamp(0.2,2.0);
            if !towns_finder.points_in_target(&entry.tile.site, s) {
                let entry = tiles.remove(i);
                town_cultures.insert(entry.tile.culture.clone());
                towns.push((entry,false)); // true means it's a capital
                progress.update(|| towns.len());
            }

        }

        if towns.len() < town_count {
            // reset everything, add what we found back to the tiles, and sort it again
            tiles.extend(towns.into_iter().map(|(a,_)| a));
            reset_town_search!();
            spacing = spacing / 2.0;
            if spacing <= 1.0 {
                progress.finish(|| format!("Only {} towns could be placed.",towns.len()));
                break;
            } else {
                progress.finish(|| "Not enough towns could be placed, trying again with reduced spacing.");
            }
        } else {
            progress.finish(|| "Towns placed.");
            break;
        }
    }
    Ok(towns)
}

fn generate_capitals<Progress: ProgressObserver>(tiles: &mut Vec<ScoredTileForTowns>, extent: &Extent, capital_count: usize, progress: &mut Progress) -> (Vec<(ScoredTileForTowns, bool)>, PointFinder) {
    let mut capitals_finder;
    let mut capitals;
    let mut capital_cultures;

    let capital_count = if tiles.len() < (capital_count * 10) {
        let capital_count = tiles.len() / 10;
        if capital_count == 0 {
            progress.warning(|| "There aren't enough populated cells to generate national capitals. Other towns will still be generated.")
        } else {
            progress.warning(|| format!("There aren't enough populated cells to generate the requested number of national capitals. Only {} capitals will be generated.",capital_count))
        }
        capital_count
    } else {
        capital_count
    };

    let mut spacing = (extent.width + extent.height) / 2.0 / capital_count as f64;

    macro_rules! reset_capital_search {
        () => {
            // this is a 2d index of points
            capitals_finder = PointFinder::new(&extent,capital_count);
            capitals = vec![];
            capital_cultures = HashSet::new();
            // sort the tiles so highest scores is at 0
            tiles.sort_by_key(|ScoredTileForTowns{capital_score,..}| std::cmp::Reverse(*capital_score));
        };
    }

    reset_capital_search!();

    // we have to do this several times, adjusting the spacing as necessary
    loop {
        // can't use a for loop, because the range changes
        let i = 0;
        progress.start_known_endpoint(|| (format!("Placing capitals at spacing {}",spacing),capital_count));
        while (i < tiles.len()) && (capitals.len() < capital_count) {
            let entry = &tiles[i];
            if !capitals_finder.points_in_target(&entry.tile.site, spacing) {
                let entry = tiles.remove(i);
                capital_cultures.insert(entry.tile.culture.clone());
                capitals.push((entry,true)); // true means it's a capital
                progress.update(|| capitals.len());
            }

        }

        if capitals.len() < capital_count {
            progress.finish(|| "Not enough capitals could be placed, trying again with reduced spacing.");
            // reset everything, add what we found back to the tiles, and sort it again
            tiles.extend(capitals.into_iter().map(|(a,_)| a));
            reset_capital_search!();
            spacing = spacing / 1.2;
        } else {
            progress.finish(|| "Capitals placed.");
            break;
        }
    }
    (capitals, capitals_finder)
}

fn gather_tiles_for_towns<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, tiles_layer: &mut TilesLayer, progress: &mut Progress) -> Result<Vec<ScoredTileForTowns>, CommandError> {

    let town_score_normal = Normal::new(1.0f64,3.0f64).unwrap(); // if this fails then it's a programming error, I'm pretty certain.

    let mut tiles = vec![];

    for tile in tiles_layer.read_features().into_entities::<TileForTowns>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (_,tile) = tile?;
        if tile.habitability > 0.0 {
            let capital_score = tile.habitability * (0.5 + rng.gen_range(0.0..1.0) * 0.5);
            let town_score = tile.habitability * town_score_normal.sample(rng).clamp(0.0,20.0);
            if (capital_score > 0.0) || (town_score > 0.0) {
                let capital_score = OrderedFloat::from(capital_score);
                let town_score = OrderedFloat::from(town_score);
                tiles.push(ScoredTileForTowns {
                    tile,
                    capital_score,
                    town_score,
                })
            }

        }

    }
    Ok(tiles)
}


pub(crate) fn generate_nations<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedCulture<'culture> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &HashMap<String,Culture>, namers: &mut HashMap<String,Namer>, default_namer: &str, size_variance: f64, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    let mut towns = target.edit_towns_layer()?;

    let mut nations = Vec::new();

    for town in towns.read_features().into_entities::<TownForNations>().watch(progress,"Reading towns.","Towns read.") {
        let (_,town) = town?;
        if town.is_capital {
            let culture = town.culture;
            let culture_data = culture.as_ref().and_then(|c| culture_lookup.get(c));
            let namer = get_namer(culture_data, namers, default_namer)?;
            let name = namer.make_state_name(rng);
            let type_ = culture_data.map(|c| c.type_()).cloned().unwrap_or_else(|| CultureType::Generic);
            let center = town.tile_id;
            let capital = town.fid as i64;
            let expansionism = rng.gen_range(0.1..1.0) * size_variance + 1.0;
            nations.push(NewNation {
                name,
                center,
                culture,
                type_,
                expansionism,
                capital
            })
    
        }
    }

    let mut nations_layer = target.create_nations_layer(overwrite_layer)?;
    for nation in nations.into_iter().watch(progress,"Writing nations.","Nations written.") {
        nations_layer.add_nation(nation)?;
    }

    Ok(())
}



pub(crate) fn expand_nations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, river_threshold: f64, limit_factor: f64, progress: &mut Progress) -> Result<(),CommandError> {

    // TODO: These nations are really huge. This may be related to cultures. I wonder if the problem is that I'm using a world-scale, while AFMG is using region-scale, even if they support world-scale. Maybe I've got to base the expansion on the size of the world somehow.

    // TODO: A lot of these lookups use 'edit', except I'm not editing them, should I be able to open up a layer without editing in the WorldMapTransaction?
    let nations = target.edit_nations_layer()?.read_features().to_entities_vec::<_,NationForPlacement>(progress)?;

    let biome_map = target.edit_biomes_layer()?.build_named_index::<_,BiomeForNationExpand>(progress)?;

    let mut tiles = target.edit_tile_layer()?;

    // we're working with a tile map, and completely overwriting whatever is there.
    let mut tile_map = tiles.read_features().to_entities_index::<_,TileForNationExpand>(progress)?;

    // priority queue keeps tasks sorted by priority
    // Since I need to go for the least priorities first, I need the double queue to get pop_min
    let mut queue = PriorityQueue::new();

    // empty hashmap of tile ids
    let mut costs = HashMap::new();

    let mut capitals = HashSet::new();

    let max_expansion_cost = OrderedFloat::from((tiles.feature_count() / 2) as f64 * limit_factor);
    
    for nation in nations {

        // place the nation center
        let tile = tile_map.get_mut(&(nation.center as u64)).ok_or_else(|| CommandError::MissingFeature(TileFeature::LAYER_NAME, nation.center as u64))?;
        tile.nation_id = Some(nation.fid as i64);

        costs.insert(nation.center as u64, OrderedFloat::from(1.0));

        capitals.insert(nation.center as u64);

        // add the tile to the queue for work.
        queue.push((nation.center as u64,nation,tile.biome.clone()), Reverse(OrderedFloat::from(0.0)));

    }

    // TODO: I use this algorithm a lot. Maybe I need to put this in some sort of function? But there are so many differences.

    let mut queue = queue.watch_queue(progress, "Expanding cultures.", "Cultures expanded.");

    while let Some(((tile_id, nation, nation_biome), priority)) = queue.pop() {

        let mut place_nations = Vec::new();

        
        // TODO: I should find a way to avoid repeating this error check.
        let tile = tile_map.get(&tile_id).ok_or_else(|| CommandError::MissingFeature(TileFeature::LAYER_NAME, nation.center as u64))?;

        for (neighbor_id,_) in &tile.neighbors {
            
            if capitals.contains(neighbor_id) {
                continue; // don't overwrite capital cells
            }

            let neighbor = tile_map.get(&neighbor_id).ok_or_else(|| CommandError::MissingFeature(TileFeature::LAYER_NAME, nation.center as u64))?;

            let culture_cost = if tile.culture == neighbor.culture {-9.0} else { 100.0 };

            let population_cost = if neighbor.grouping.is_water() { 
                0.0
            } else if neighbor.habitability > 0.0 {
                (20.0 - neighbor.habitability).max(0.0)
            } else {
                5000.0
            };

            let neighbor_biome = biome_map.get(&neighbor.biome).ok_or_else(|| CommandError::UnknownBiome(neighbor.biome.clone()))?;

            let biome_cost = get_biome_cost(&nation_biome,neighbor_biome,&nation.type_);

            let height_cost = get_height_cost(neighbor, &nation.type_);

            let river_cost = get_river_cost(neighbor, river_threshold, &nation.type_);

            let shore_cost = get_shore_cost(neighbor, &nation.type_);

            let cell_cost = OrderedFloat::from((culture_cost + population_cost + biome_cost + height_cost + river_cost + shore_cost).max(0.0)) / nation.expansionism;

            let total_cost = priority.0 + OrderedFloat::from(10.0) + cell_cost;

            if total_cost <= max_expansion_cost {

                // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                // then I can place or replace the culture with this one. This will remove cultures that were previously
                // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                // may still be spreading, don't worry).
                let replace_culture = if let Some(neighbor_cost) = costs.get(&neighbor_id) {
                    if &total_cost < neighbor_cost {
                        true
                    } else {
                        false
                    }
                } else {
                    true
                };

                if replace_culture {
                    if !neighbor.grouping.is_ocean() {
                        place_nations.push((*neighbor_id,nation.fid.clone()));
                        // even if we don't place the culture, because people can't live here, it will still spread.
                    }
                    costs.insert(*neighbor_id, total_cost);

                    queue.push((*neighbor_id, nation.clone(), nation_biome.clone()), Reverse(total_cost));

                } // else we can't expand into this tile, and this line of spreading ends here.
            } else {
                // else we can't make it into this tile, so give up.    
    
            }


        }

        for (tile_id,nation_id) in place_nations {
            let tile = tile_map.get_mut(&tile_id).ok_or_else(|| CommandError::MissingFeature(TileFeature::LAYER_NAME, tile_id))?;
            tile.nation_id = Some(nation_id as i64);
        }


    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing nations.","Nations written.") {

        let mut feature = tiles.try_feature_by_id(&fid)?;

        feature.set_nation_id(tile.nation_id)?;

        tiles.update_feature(feature)?;

    }


    Ok(())
}

fn get_shore_cost(neighbor: &TileForNationExpand, culture_type: &CultureType) -> f64 {
    match culture_type {
        CultureType::Lake => match neighbor.shore_distance {
            1 => 0.0,
            2 => 0.0, 
            ..=-2 | 0 | 2.. => 100.0, // penalty for the mainland // TODO: But also the outer water
            -1 => 0.0,
        },
        CultureType::Naval => match neighbor.shore_distance {
            1 => 0.0,
            2 => 30.0, // penalty for mainland 
            ..=-2 | 0 | 2.. => 100.0,  // penalty for mainland // TODO: But also the outer water
            -1 => 0.0,
        },
        CultureType::Nomadic => match neighbor.shore_distance {
            1 => 60.0, // larger penalty for reaching the coast
            2 => 30.0, // penalty for approaching the coast
            ..=-2 | 0 | 2.. => 0.0, 
            -1 => 0.0, // TODO: No problem going out on the ocean?
        },
        CultureType::Generic  => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            2 => 0.0, 
            ..=-2 | 0 | 2.. => 0.0, 
            -1 => 0.0, // TODO: No problem going out on the ocean?
        },
        CultureType::River => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            2 => 0.0, 
            ..=-2 | 0 | 2.. => 0.0, 
            -1 => 0.0, // TODO: No problem going out on the ocean?
        },
        CultureType::Hunting => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            2 => 0.0, 
            ..=-2 | 0 | 2.. => 0.0, 
            -1 => 0.0, // TODO: No problem going out on the ocean?
        },
        CultureType::Highland => match neighbor.shore_distance {
            1 => 20.0, // penalty for reaching the coast
            2 => 0.0, 
            ..=-2 | 0 | 2.. => 0.0, 
            -1 => 0.0, // TODO: No problem going out on the ocean?
        },
    }

}

fn get_river_cost(neighbor: &TileForNationExpand, river_threshold: f64, culture_type: &CultureType) -> f64 {
    match culture_type {
        // TODO: Can I go wi
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

fn get_height_cost(neighbor: &TileForNationExpand, culture_type: &CultureType) -> f64 {
    // TODO: This is similar to the way cultures work, but not exactly.
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
        CultureType::Nomadic => if neighbor.grouping.is_water() {
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

fn get_biome_cost(culture_biome: &String, neighbor_biome: &BiomeForNationExpand, culture_type: &CultureType) -> f64 {
    // TODO: This is very similar to the one for cultures, but not exactly.

    // FUTURE: I need a way to make this more configurable...
    const FOREST_BIOMES: [&str; 5] = [BiomeFeature::TROPICAL_SEASONAL_FOREST, BiomeFeature::TEMPERATE_DECIDUOUS_FOREST, BiomeFeature::TROPICAL_RAINFOREST, BiomeFeature::TEMPERATE_RAINFOREST, BiomeFeature::TAIGA];

    
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