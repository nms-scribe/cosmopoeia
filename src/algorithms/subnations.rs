use std::cmp::Reverse;
use std::collections::HashMap;

use priority_queue::PriorityQueue;
use ordered_float::OrderedFloat;
use rand_distr::Normal;
use rand_distr::Distribution;
use rand::Rng;

use crate::world_map::TileForSubnationNormalize;
use crate::world_map::NationForEmptySubnations;
use crate::world_map::TownForEmptySubnations;
use crate::world_map::TileForEmptySubnations;
use crate::world_map::SubnationForPlacement;
use crate::world_map::TileForSubnationExpand;
use crate::world_map::NewSubnation;
use crate::world_map::CultureType;
use crate::world_map::TileForSubnations;
use crate::world_map::NationForSubnations;
use crate::world_map::TownForSubnations;
use crate::errors::CommandError;
use crate::algorithms::naming::LoadedNamers;
use crate::world_map::WorldMapTransaction;
use crate::world_map::CultureWithType;
use crate::world_map::CultureWithNamer;
use crate::world_map::NamedEntity;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::CultureSchema;
use crate::world_map::EntityLookup;
use crate::world_map::EntityIndex;


pub(crate) fn generate_subnations<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut LoadedNamers, default_namer: &str, subnation_percentage: f64, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    let town_map = target.edit_towns_layer()?.read_features().to_entities_index::<_,TownForSubnations>(progress)?;
    let nations = target.edit_nations_layer()?.read_features().to_entities_vec::<_,NationForSubnations>(progress)?; 
    let mut towns_by_nation = HashMap::new();

    for tile in target.edit_tile_layer()?.read_features().into_entities::<TileForSubnations>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (_,tile) = tile?;
        if let (Some(nation_id),Some(town_id)) = (tile.nation_id,tile.town_id) {
            match towns_by_nation.get_mut(&nation_id) {
                None => {towns_by_nation.insert(nation_id, vec![(tile,town_id)]); },
                Some(list) => list.push((tile,town_id))
            }
        }
    
    }

    let town_sort_normal = Normal::new(1.0f64,0.2f64).unwrap();

    let mut subnations = target.create_subnations_layer(overwrite_layer)?;

    for nation in nations.into_iter().watch(progress,"Creating subnations.","Subnations created.") {
        let mut nation_towns = towns_by_nation.remove(&(nation.fid as i64)).unwrap_or_else(|| vec![]);
        if nation_towns.len() < 2 {
            continue; // at least two towns are required to get a province
        }

        let subnation_count = ((nation_towns.len() as f64 * subnation_percentage)/100.0).max(2.0).floor() as usize; // at least two must be created
        nation_towns.sort_by_cached_key(|a| (OrderedFloat::from(a.0.population as f64) * town_sort_normal.sample(rng).clamp(0.5,1.5),(a.1 == nation.capital)));
    
        for i in 0..subnation_count {
            let center = nation_towns[i].0.fid as i64;
            let seat = nation_towns[i].1;
            let culture = nation_towns[i].0.culture.clone();
            let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;
            let name = if rng.gen_bool(0.5) {
                // name by town
                let town = town_map.try_get(&(seat as u64))?;
                town.name.clone()
            } else {
                // new name by culture
                let namer = Culture::get_namer(culture_data, namers, default_namer)?;
                namer.make_state_name(rng)                  
            };
            let color = nation.color.clone();

            let type_ = culture_data.map(|c| c.type_()).cloned().unwrap_or_else(|| CultureType::Generic);

            let seat = Some(seat);

            subnations.add_subnation(NewSubnation {
                name,
                culture,
                center,
                type_,
                seat,
                nation_id: nation.fid as i64,
                color
            })?;
        }
    }


    Ok(())
}

pub(crate) fn expand_subnations<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, subnation_percentage: f64, progress: &mut Progress) -> Result<(),CommandError> {

    let max = subnation_max_cost(rng, subnation_percentage);

    let mut tile_layer = target.edit_tile_layer()?;

    let mut tile_map = tile_layer.read_features().to_entities_index::<_,TileForSubnationExpand>(progress)?;

    let mut costs = HashMap::new();

    let mut queue = PriorityQueue::new();

    for subnation in target.edit_subnations_layer()?.read_features().into_entities::<SubnationForPlacement>().watch(progress,"Reading subnations.","Subnations read.") {
        let (_,subnation) = subnation?; // TODO: I have to do this so often, is there a shortcut?
        let center = subnation.center as u64;
        tile_map.try_get_mut(&center)?.subnation_id = Some(subnation.fid as i64);
        costs.insert(center, OrderedFloat::from(1.0));
        queue.push((center,subnation), Reverse(OrderedFloat::from(0.0)));
    }

    let mut queue = queue.watch_queue(progress, "Expanding subnations.", "Subnations expanded.");

    while let Some(((tile_id,subnation),priority)) = queue.pop() {

        let mut place_subnations = Vec::new();

        let tile = tile_map.try_get(&(tile_id as u64))?;
        for (neighbor_id,_) in &tile.neighbors {
            let neighbor = tile_map.try_get(&neighbor_id)?;

            let total_cost = match subnation_expansion_cost(neighbor, &subnation, priority) {
                Some(value) => value,
                None => continue,
            };

            if total_cost.0 <= max {

                // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                // then I can place or replace the culture with this one. This will remove cultures that were previously
                // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                // may still be spreading, don't worry).
                let replace_subnation = if let Some(neighbor_cost) = costs.get(&neighbor_id) {
                    if &total_cost.0 < neighbor_cost {
                        true
                    } else {
                        false
                    }
                } else {
                    true
                };

                if replace_subnation {
                    if !neighbor.grouping.is_ocean() { // this is also true for nations.
                        place_subnations.push((*neighbor_id,subnation.fid.clone()));
                    }
                    costs.insert(*neighbor_id, total_cost);
                    queue.push((*neighbor_id,subnation.clone()), Reverse(total_cost));
                } // else we can't expand into this tile, and this line of spreading ends here.
            }


        }
    
        for (tile_id,subnation_id) in place_subnations {
            let tile = tile_map.try_get_mut(&tile_id)?;
            tile.subnation_id = Some(subnation_id as i64);
        }



    }

    let tile_layer = target.edit_tile_layer()?;

    for (fid,tile) in tile_map.into_iter().watch(progress,"Writing subnations.","Subnations written.") {
        let mut feature = tile_layer.try_feature_by_id(&fid)?;
        feature.set_subnation_id(tile.subnation_id)?;
        tile_layer.update_feature(feature)?;
    }



    Ok(())
}

pub(crate) fn subnation_max_cost<Random: Rng>(rng: &mut Random, subnation_percentage: f64) -> f64 {
    if subnation_percentage == 100.0 {
        1000.0
    } else {
        Normal::new(20.0f64,5.0f64).unwrap().sample(rng).clamp(5.0,100.0) * subnation_percentage.powf(0.5)
    }
}

pub(crate) fn subnation_expansion_cost(neighbor: &TileForSubnationExpand, subnation: &SubnationForPlacement, priority: Reverse<OrderedFloat<f64>>) -> Option< OrderedFloat<f64>> {
    if neighbor.shore_distance < -3 {
        return None; // don't pass through deep ocean
    }
    if neighbor.nation_id != Some(subnation.nation_id) {
        return None; // don't leave nation
    }
    let elevation_cost = if neighbor.elevation_scaled >= 70 {
        100
    } else if neighbor.elevation_scaled >= 50 {
        30
    } else if neighbor.grouping.is_water() {
        100
    } else {
        10
    } as f64;
    let total_cost = priority.0 + elevation_cost;
    Some(total_cost)
}

pub(crate) fn fill_empty_subnations<'culture, Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut LoadedNamers, default_namer: &str, subnation_percentage: f64, progress: &mut Progress) -> Result<(),CommandError> {

    let max = subnation_max_cost(rng, subnation_percentage);

    let mut tile_layer = target.edit_tile_layer()?;

    let mut tiles_by_nation = HashMap::new();

    let mut tile_map = EntityIndex::new();

    for tile in tile_layer.read_features().into_entities::<TileForEmptySubnations>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (fid,tile) = tile?;
        if let (Some(nation_id),None) = (tile.nation_id,tile.subnation_id) {
            let nation_id = nation_id as u64;
            // use a priority queue to make it easier to remove by value as well.
            match tiles_by_nation.get_mut(&nation_id) {
                None => { 
                    let mut queue = PriorityQueue::new();
                    queue.push(fid, tile.population);
                    tiles_by_nation.insert(nation_id, queue); 
                },
                Some(queue) => {
                    queue.push(fid, tile.population);
                },
            }
        }
        tile_map.insert(fid, tile);
    }

    let town_map = target.edit_towns_layer()?.read_features().to_entities_index::<_,TownForEmptySubnations>(progress)?;

    let nations = target.edit_nations_layer()?.read_features().to_entities_vec::<_,NationForEmptySubnations>(progress)?;

    let mut tile_subnation_changes = HashMap::new();
    let mut new_subnations = Vec::new();
    let mut next_subnation_id = 0..;

    for nation in nations.into_iter().watch(progress,"Creating and placing subnations.","Subnations created and placed.") {
        if let Some(mut nation_tiles) = tiles_by_nation.remove(&nation.fid) {
            while let Some((tile_id,_)) = nation_tiles.pop() {
                let tile = tile_map.try_get(&tile_id)?;
                // we have what we need to start a new subnation, this should be the highest population tile
                let mut seat = None;
                let center = tile_id as i64;

                let culture = tile.culture.as_ref().or_else(|| nation.culture.as_ref()).cloned();
                let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;

                let type_ = culture_data.map(|c| c.type_()).cloned().unwrap_or_else(|| CultureType::Generic);

                let nation_id = nation.fid as i64;
                let color = nation.color.clone();


                let subnation = SubnationForPlacement {
                    fid: next_subnation_id.next().unwrap(), // It's an infinite range, it should always unwrap
                    center,
                    nation_id,
                };



                tile_subnation_changes.insert(tile_id, subnation.fid);
                let mut costs = HashMap::new();
                costs.insert(tile_id, OrderedFloat::from(1.0));
                let mut queue = PriorityQueue::new();
                queue.push(tile_id,Reverse(OrderedFloat::from(0.0)));
                while let Some((tile_id,priority)) = queue.pop() {
                    let tile = tile_map.try_get(&tile_id)?;
                    // check if we've got a seat, or a better one.
                    match (tile.town_id,seat) {
                        (Some(town_id),None) => seat = Some((town_id,tile.population)),
                        (Some(new_town_id),Some((_,old_population))) if tile.population > old_population => {
                            seat = Some((new_town_id,tile.population))
                        },
                        (Some(_),Some(_)) | (None,_) => {}
                    }

                    for (neighbor_id,_) in &tile.neighbors {
                        let neighbor = tile_map.try_get(&neighbor_id)?;
                        if neighbor.subnation_id.is_some() {
                            continue;
                        }

                        // the cost is different than regular subnation expansion. Basically, there is no cost to finish filling
                        // up everything, except a small cost to keep things small.
                        if neighbor.shore_distance < -3 {
                            continue; // don't pass through deep ocean
                        }
                        if neighbor.nation_id != Some(subnation.nation_id) {
                            continue; // don't leave nation
                        }

                        let total_cost = priority.0 + 10.0;                        
                            
                        if total_cost.0 <= max {
        
                            // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                            // then I can place or replace the culture with this one. This will remove cultures that were previously
                            // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                            // may still be spreading, don't worry).
                            let replace_subnation = if let Some(neighbor_cost) = costs.get(&neighbor_id) {
                                if &total_cost < neighbor_cost {
                                    true
                                } else {
                                    false
                                }
                            } else {
                                true
                            };
        
                            if replace_subnation {
                                if !neighbor.grouping.is_ocean() { 
                                    tile_subnation_changes.insert(*neighbor_id, subnation.fid);
                                }
                                nation_tiles.remove(neighbor_id);
                                costs.insert(*neighbor_id, total_cost);
                                queue.push(*neighbor_id, Reverse(total_cost));
                            } // else we can't expand into this tile, and this line of spreading ends here.
                        }
        
        
                    }


                }

                let seat = seat.map(|(id,_)| id);

                let name = if let (Some(seat),true) = (seat,rng.gen_bool(0.5)) {
                    // name by town
                    let town = town_map.try_get(&(seat as u64))?;
                    town.name.clone()
                } else {
                    // new name by culture
                    let namer = Culture::get_namer(culture_data, namers, default_namer)?;
                    namer.make_state_name(rng)                  
                };

                new_subnations.push((subnation.fid,NewSubnation {
                    name,
                    culture,
                    center,
                    type_,
                    seat,
                    nation_id,
                    color
                }))


            }

        }

    }


    // create the new subnations and get their real id for assigning to tiles.
    let mut assigned_ids = HashMap::new();

    let mut subnations_layer = target.edit_subnations_layer()?;

    for (temp_id,subnation) in new_subnations.into_iter().watch(progress, "Writing new subnations.", "New subnations written.") {
        let real_id = subnations_layer.add_subnation(subnation)?;
        assigned_ids.insert(temp_id, real_id as i64);
    }

    let tiles_layer = target.edit_tile_layer()?;

    for (tile_id,temp_subnation_id) in tile_subnation_changes.into_iter().watch(progress,"Writing new subnations to tiles.","New subnations written to tiles.") {

        let mut tile = tiles_layer.try_feature_by_id(&tile_id)?;

        let real_id = assigned_ids.get(&temp_subnation_id).unwrap();

        tile.set_subnation_id(Some(*real_id))?;

        tiles_layer.update_feature(tile)?;

    }


    Ok(())
}

// TODO: is 'normalize' the right word?
pub(crate) fn normalize_subnations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tile_map = EntityIndex::new();
    let mut tile_list = Vec::new();

    for tile in tiles_layer.read_features().into_entities::<TileForSubnationNormalize>().watch(progress,"Reading tiles.","Tiles read.") {
        let (fid,tile) = tile?;
        tile_list.push(fid);
        tile_map.insert(fid,tile);
    }

    for tile_id in tile_list.into_iter().watch(progress,"Normalizing subnations.","Subnations normalized.") {
        let tile = tile_map.try_get(&tile_id)?;

        if tile.town_id.is_some() {
            continue; // don't overwrite towns
        }

        let mut adversaries = HashMap::new();
        let mut adversary_count = 0;
        let mut buddy_count = 0;
        for (neighbor_id,_) in &tile.neighbors {

            let neighbor = tile_map.try_get(&neighbor_id)?;

            if neighbor.nation_id == tile.nation_id {
                if neighbor.subnation_id != tile.subnation_id {
                    if let Some(count) = adversaries.get(&neighbor.subnation_id) {
                        adversaries.insert(neighbor.subnation_id, count + 1)
                    } else {
                        adversaries.insert(neighbor.subnation_id, 1)
                    };
                    adversary_count += 1;
                } else {
                    buddy_count += 1;
                }
            }

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
                tile.set_subnation_id(worst_adversary)?;
                tiles_layer.update_feature(tile)?    
            }

        }

    }


    Ok(()) 
}
