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
use crate::algorithms::naming::LoadedNamers;
use crate::world_map::TileForTowns;
use crate::utils::point_finder::PointFinder;
use crate::world_map::NewTown;
use crate::world_map::TileForTownPopulation;
use crate::world_map::LakeForTownPopulation;
use crate::world_map::TownForPopulation;
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
use crate::world_map::BiomeFeature;
use crate::world_map::TileForNationNormalize;
use crate::world_map::TownForNationNormalize;
use crate::utils::Point;
use crate::utils::TryGetMap;


struct ScoredTileForTowns {
    tile: TileForTowns,
    capital_score: OrderedFloat<f64>,
    town_score: OrderedFloat<f64>
}

pub(crate) fn generate_towns<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedCulture<'culture> + CultureWithNamer, CultureMap: TryGetMap<String,Culture>>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &CultureMap, namers: &mut LoadedNamers, default_namer: &str, capital_count: usize, town_count: Option<usize>, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    // TODO: Certain culture "types" shouldn't generate towns, or should generate fewer towns. Nomads, for example. 

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
        let namer = Culture::get_namer(culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?, namers, default_namer)?;
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

pub(crate) fn populate_towns<'culture, Progress: ProgressObserver>(target: &mut WorldMapTransaction, river_threshold: f64, progress: &mut Progress) -> Result<(),CommandError> {

    struct TownDetails {
        population: i32,
        is_port: bool,
        new_location: Option<Point>
    }

    let mut tile_layer = target.edit_tile_layer()?;

    let tile_map = tile_layer.read_features().to_entities_index::<_,TileForTownPopulation>(progress)?;

    let mut lake_layer = target.edit_lakes_layer()?;

    let lake_map = lake_layer.read_features().to_entities_index::<_,LakeForTownPopulation>(progress)?;

    let mut coastal_towns = HashMap::new();

    let mut town_details = HashMap::new();

    let mut towns_layer = target.edit_towns_layer()?;

    for town in towns_layer.read_features().into_entities::<TownForPopulation>().watch(progress,"Populating towns.","Towns populated.") {
        let (_,town) = town?;
        let tile = tile_map.try_get(&(town.tile_id as u64))?;

        // figure out if it's a port
        let port_location = if let Some(closest_water) = tile.closest_water {
            let harbor = tile_map.try_get(&(closest_water as u64))?;

            // add it to the map of towns by feature for removing port status later.
            match coastal_towns.get_mut(&harbor.grouping_id) {
                None => { coastal_towns.insert(harbor.grouping_id, vec![town.fid]); },
                Some(entry) => entry.push(town.fid),
            }

            // no ports if the water is frozen
            if harbor.temperature > 0.0 {
                let on_large_water = if let Some(lake_id) = harbor.lake_id {
                    // don't make it a port if the lake is only 1 tile big
                    let lake = lake_map.try_get(&(lake_id as u64))?;
                    lake.size > 1
                } else {
                    harbor.grouping.is_ocean()
                };

                // it's a port if it's on the large water and either it's a capital or has a good harbor (only one water tile next to it)
                if on_large_water && (town.is_capital || matches!(tile.water_count,Some(1))) {
                    Some(tile.find_middle_point_between(harbor)?)
                } else {
                    None
                }
            } else {
                None
            }

        } else {
            None
        };

        // figure out it's population -- habitability is already divided by 5, so this makes it 10% of true suitability for people.
        // FUTURE: The population should be increased by the road traffic, but that could be done in the road generating stuff
        // TODO: I'm not sure why AFMG added that 8 in there. Check town populations when I'm done and possibly get rid of it.
        let population = (((tile.habitability / 2.0) / 8.0) * 1000.0).max(100.0); 

        let population = if town.is_capital {
            population * 1.3
        } else {
            population
        };

        let population = if port_location.is_some() {
            population * 1.3
        } else {
            population
        };

        let population = population.floor() as i32;

        let (is_port,new_location) = if port_location.is_none() && tile.water_flow > river_threshold {
            let shift = (tile.water_flow / 150.0).min(1.0);
            let x = if (tile.site.x.into_inner() % 2.0) < 1.0 { tile.site.x + shift } else { tile.site.x - shift };
            let y = if (tile.site.y.into_inner() % 2.0) < 1.0 { tile.site.y + shift } else { tile.site.y - shift };
            (false,Some(Point::new(x,y)))
        } else {
            (port_location.is_some(),port_location)
        };


        town_details.insert(town.fid,TownDetails {
            new_location,
            population,
            is_port
        });
    }

    // remove port status if there's only one on the feature, but still get the benefits
    for list in coastal_towns.values().watch(progress,"Validating ports.","Ports validated.") {
        if list.len() == 1 {
            town_details.get_mut(&list[0]).unwrap().is_port = false
        }
    }

    for (fid,town) in town_details.into_iter().watch(progress,"Writing town details.","Town details written.") {
        let mut town_feature = towns_layer.try_feature_by_id(&fid)?;
        if let Some(new_location) = town.new_location {
            town_feature.move_to(new_location)?;
        }
        town_feature.set_population(town.population)?;
        town_feature.set_is_port(town.is_port)?;
        towns_layer.update_feature(town_feature)?;
    }


    Ok(())
}


pub(crate) fn generate_nations<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedCulture<'culture> + CultureWithNamer + CultureWithType, CultureMap: TryGetMap<String,Culture>>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &CultureMap, namers: &mut LoadedNamers, default_namer: &str, size_variance: f64, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    let mut towns = target.edit_towns_layer()?;

    let mut nations = Vec::new();

    for town in towns.read_features().into_entities::<TownForNations>().watch(progress,"Reading towns.","Towns read.") {
        let (_,town) = town?;
        if town.is_capital {
            let culture = town.culture;
            let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;
            let namer = Culture::get_namer(culture_data, namers, default_namer)?;
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

    let biome_map = target.edit_biomes_layer()?.build_lookup::<_,BiomeForNationExpand>(progress)?;

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
        let tile = tile_map.try_get_mut(&(nation.center as u64))?;
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
        let tile = tile_map.try_get(&tile_id)?;

        for (neighbor_id,_) in &tile.neighbors {
            
            if capitals.contains(neighbor_id) {
                continue; // don't overwrite capital cells
            }

            let neighbor = tile_map.try_get(&neighbor_id)?;

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
            let tile = tile_map.try_get_mut(&tile_id)?;
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

// TODO: is 'normalize' the right word?
pub(crate) fn normalize_nations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let town_index = target.edit_towns_layer()?.read_features().to_entities_index::<_,TownForNationNormalize>(progress)?;

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tile_map = HashMap::new();
    let mut tile_list = Vec::new();

    for tile in tiles_layer.read_features().into_entities::<TileForNationNormalize>().watch(progress,"Reading tiles.","Tiles read.") {
        let (fid,tile) = tile?;
        tile_list.push(fid);
        tile_map.insert(fid,tile);
    }

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

            let neighbor = tile_map.try_get(&neighbor_id)?;

            if let Some(town_id) = neighbor.town_id {
                let town = town_index.try_get(&(town_id as u64))?;
                if town.is_capital {
                    dont_overwrite = true; // don't overwrite near capital
                    break;
                }
            }

            if !neighbor.grouping.is_water() {
                if neighbor.nation_id != tile.nation_id {
                    if let Some(count) = adversaries.get(&neighbor.nation_id) {
                        adversaries.insert(neighbor.nation_id, count + 1)
                    } else {
                        adversaries.insert(neighbor.nation_id, 1)
                    };
                    adversary_count += 1;
                } else {
                    buddy_count += 1;
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

        if let Some((worst_adversary,count)) = adversaries.into_iter().max_by_key(|(_,count)| *count).and_then(|(adversary,count)| Some((adversary,count))) {
            if count > buddy_count {
                let mut tile = tiles_layer.try_feature_by_id(&tile_id)?;
                tile.set_nation_id(worst_adversary)?;
                tiles_layer.update_feature(tile)?    
            }
    
        }

    }

    
    Ok(()) 
}