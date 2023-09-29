use core::cmp::Reverse;
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
use crate::world_map::NationForSubnationColors;
use crate::errors::CommandError;
use crate::algorithms::naming::NamerSet;
use crate::world_map::WorldMapTransaction;
use crate::world_map::CultureWithType;
use crate::world_map::CultureWithNamer;
use crate::world_map::NamedEntity;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::CultureSchema;
use crate::world_map::EntityLookup;
use crate::world_map::SubnationForNormalize;
use crate::commands::OverwriteSubnationsArg;
use crate::commands::SubnationPercentArg;
use crate::algorithms::colors::RandomColorGenerator;
use super::colors::Luminosity;
use crate::world_map::SubnationForColors;


pub(crate) fn generate_subnations<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut NamerSet, subnation_percentage: &SubnationPercentArg, overwrite_layer: &OverwriteSubnationsArg, progress: &mut Progress) -> Result<(),CommandError> {

    let town_map = target.edit_towns_layer()?.read_features().into_entities_index::<_,TownForSubnations>(progress)?;
    let nations = target.edit_nations_layer()?.read_features().into_entities_vec::<_,NationForSubnations>(progress)?; 
    let mut towns_by_nation = HashMap::new();

    for tile in target.edit_tile_layer()?.read_features().into_entities::<TileForSubnations>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (_,tile) = tile?;
        if let (Some(nation_id),Some(town_id)) = (tile.nation_id,tile.town_id) {
            match towns_by_nation.get_mut(&nation_id) {
                None => _ = towns_by_nation.insert(nation_id, vec![(tile,town_id)]),
                Some(list) => list.push((tile,town_id))
            }
        }
    
    }

    let town_sort_normal = Normal::new(1.0f64,0.2f64).expect("Why would these constants fail when they never have before?");

    let mut subnations = target.create_subnations_layer(overwrite_layer)?;

    for nation in nations.into_iter().watch(progress,"Creating subnations.","Subnations created.") {
        let mut nation_towns = towns_by_nation.remove(&nation.fid).unwrap_or_default();
        if nation_towns.len() < 2 {
            continue; // at least two towns are required to get a province
        }

        let subnation_count = ((nation_towns.len() as f64 * subnation_percentage.subnation_percentage)/100.0).max(2.0).floor() as usize; // at least two must be created
        nation_towns.sort_by_cached_key(|a| (OrderedFloat::from(a.0.population as f64) * town_sort_normal.sample(rng).clamp(0.5,1.5),(a.1 == nation.capital_town_id)));

        for (center_tile,seat) in nation_towns.iter().take(subnation_count) {
            let center_tile_id = center_tile.fid;
            let culture = center_tile.culture.clone();
            let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;
            let name = if rng.gen_bool(0.5) {
                // name by town
                let town = town_map.try_get(seat)?;
                town.name.clone()
            } else {
                // new name by culture
                let namer = Culture::get_namer(culture_data, namers)?;
                namer.make_state_name(rng)                  
            };
            let color = nation.color;

            let type_ = culture_data.map(CultureWithType::type_).cloned().unwrap_or(CultureType::Generic);

            let seat_town_id = Some(*seat);

            _ = subnations.add_subnation(&NewSubnation {
                name,
                culture,
                center_tile_id,
                type_,
                seat_town_id,
                nation_id: nation.fid,
                color
            })?;
        }
    }


    Ok(())
}

pub(crate) fn expand_subnations<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, subnation_percentage: &SubnationPercentArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tile_layer = target.edit_tile_layer()?;

    let max = subnation_max_cost(rng, tile_layer.estimate_average_tile_area()?, subnation_percentage.subnation_percentage);

    let mut tile_map = tile_layer.read_features().into_entities_index::<_,TileForSubnationExpand>(progress)?;

    let mut costs = HashMap::new();

    let mut queue = PriorityQueue::new();

    for subnation in target.edit_subnations_layer()?.read_features().into_entities::<SubnationForPlacement>().watch(progress,"Reading subnations.","Subnations read.") {
        let (_,subnation) = subnation?;
        let center = subnation.center_tile_id;
        tile_map.try_get_mut(&center)?.subnation_id = Some(subnation.fid);
        _ = costs.insert(center, OrderedFloat::from(1.0));
        _ = queue.push((center,subnation), Reverse(OrderedFloat::from(0.0)));
    }

    let mut queue = queue.watch_queue(progress, "Expanding subnations.", "Subnations expanded.");

    while let Some(((tile_id,subnation),priority)) = queue.pop() {

        let mut place_subnations = Vec::new();

        let tile = tile_map.try_get(&(tile_id))?;
        for (neighbor_id,_) in &tile.neighbors {
            let neighbor = tile_map.try_get(neighbor_id)?;

            let Some(total_cost) = subnation_expansion_cost(neighbor, &subnation, priority) else { continue };

            if total_cost.0 <= max {

                // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                // then I can place or replace the culture with this one. This will remove cultures that were previously
                // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                // may still be spreading, don't worry).
                let replace_subnation = if let Some(neighbor_cost) = costs.get(neighbor_id) {
                    &total_cost.0 < neighbor_cost
                } else {
                    true
                };

                if replace_subnation {
                    if !neighbor.grouping.is_ocean() { // this is also true for nations.
                        place_subnations.push((*neighbor_id,subnation.fid));
                    }
                    _ = costs.insert(*neighbor_id, total_cost);
                    queue.push((*neighbor_id,subnation.clone()), Reverse(total_cost));
                } // else we can't expand into this tile, and this line of spreading ends here.
            }


        }
    
        for (place_tile_id,subnation_id) in place_subnations {
            let place_tile = tile_map.try_get_mut(&place_tile_id)?;
            place_tile.subnation_id = Some(subnation_id);
        }



    }

    let tile_layer_update = target.edit_tile_layer()?;

    for (fid,tile) in tile_map.into_iter().watch(progress,"Writing subnations.","Subnations written.") {
        let mut feature = tile_layer_update.try_feature_by_id(fid)?;
        feature.set_subnation_id(tile.subnation_id)?;
        tile_layer_update.update_feature(feature)?;
    }



    Ok(())
}

pub(crate) fn subnation_max_cost<Random: Rng>(rng: &mut Random, estimated_tile_area: f64, subnation_percentage: f64) -> f64 {

    // This is how far the nations will be able to spread.
    // This is a arbitrary number, it basically limits the size of the nation to about 5,000 "square degrees" (half the size of a culture). Although once
    // I get sherical directions and areas, I'll want to revisit this.
    let max_expansion_cost = (500.0/estimated_tile_area).max(5.0);

    if (subnation_percentage - 100.0).abs() < f64::EPSILON {
        max_expansion_cost
    } else {
        Normal::new(max_expansion_cost/5.0,5.0f64).expect("Why would these constants fail if they naver have before?").sample(rng).clamp(5.0,max_expansion_cost) * subnation_percentage.sqrt()
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
    let total_cost = priority.0 + elevation_cost * neighbor.area;
    Some(total_cost)
}

pub(crate) fn fill_empty_subnations<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer + CultureWithType>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut NamerSet, subnation_percentage: &SubnationPercentArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tile_layer = target.edit_tile_layer()?;

    let max = subnation_max_cost(rng, tile_layer.estimate_average_tile_area()?, subnation_percentage.subnation_percentage);

    let mut tiles_by_nation = HashMap::new();

    let tile_map = tile_layer.read_features().into_entities_index_for_each::<_,TileForEmptySubnations,_>(|fid,tile| {
        if let (Some(nation_id),None) = (tile.nation_id,tile.subnation_id) {
            // use a priority queue to make it easier to remove by value as well.
            match tiles_by_nation.get_mut(&nation_id) {
                None => { 
                    let mut queue = PriorityQueue::new();
                    _ = queue.push(*fid, tile.population);
                    _ = tiles_by_nation.insert(nation_id, queue); 
                },
                Some(queue) => {
                    _ = queue.push(*fid, tile.population);
                },
            }
        }
        Ok(())
    }, progress)?;

    let town_map = target.edit_towns_layer()?.read_features().into_entities_index::<_,TownForEmptySubnations>(progress)?;

    let nations = target.edit_nations_layer()?.read_features().into_entities_vec::<_,NationForEmptySubnations>(progress)?;

    let mut tile_subnation_changes = HashMap::new();
    let mut new_subnations = Vec::new();
    let mut next_subnation_id = 0..;

    for nation in nations.into_iter().watch(progress,"Creating and placing subnations.","Subnations created and placed.") {

        if let Some(mut nation_tiles) = tiles_by_nation.remove(&nation.fid) {
            while let Some((tile_id,_)) = nation_tiles.pop() {
                let tile = tile_map.try_get(&tile_id)?;
                // we have what we need to start a new subnation, this should be the highest population tile
                let mut seat = None;
                let center_tile_id = tile_id;

                #[allow(clippy::unnecessary_lazy_evaluations)] // I disagree, it's calling a function
                let culture = tile.culture.as_ref().or_else(|| nation.culture.as_ref()).cloned();
                let culture_data = culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?;

                let type_ = culture_data.map(CultureWithType::type_).cloned().unwrap_or(CultureType::Generic);

                let nation_id = nation.fid;
                let color = nation.color;


                let subnation = SubnationForPlacement {
                    fid: next_subnation_id.next().expect("Why would an unlimited range stop returning values?"),
                    center_tile_id,
                    nation_id,
                };

                _ = tile_subnation_changes.insert(tile_id, subnation.fid);
                
                let mut costs = HashMap::new();
                _ = costs.insert(tile_id, OrderedFloat::from(1.0));
                
                let mut queue = PriorityQueue::new();
                _ = queue.push(tile_id,Reverse(OrderedFloat::from(0.0)));

                while let Some((expand_tile_id,priority)) = queue.pop() {
                    let expand_tile = tile_map.try_get(&expand_tile_id)?;
                    // check if we've got a seat, or a better one.
                    match (expand_tile.town_id,seat) {
                        (Some(town_id),None) => seat = Some((town_id,expand_tile.population)),
                        (Some(new_town_id),Some((_,old_population))) if expand_tile.population > old_population => {
                            seat = Some((new_town_id,expand_tile.population))
                        },
                        (Some(_),Some(_)) | (None,_) => {}
                    }

                    for (neighbor_id,_) in &expand_tile.neighbors {

                        if nation_tiles.get(neighbor_id).is_none() {
                            // we've already placed this in another subnation, or it wasn't available.
                            continue;
                        }


                        let neighbor = tile_map.try_get(neighbor_id)?;
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

                        let total_cost = priority.0 + 10.0 * neighbor.area;                        
                            
                        if total_cost.0 <= max {
        
                            // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                            // then I can place or replace the culture with this one. This will remove cultures that were previously
                            // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                            // may still be spreading, don't worry).
                            let replace_subnation = if let Some(neighbor_cost) = costs.get(neighbor_id) {
                                &total_cost < neighbor_cost
                            } else {
                                true
                            };
        
                            if replace_subnation {
                                if !neighbor.grouping.is_ocean() { 
                                    _ = tile_subnation_changes.insert(*neighbor_id, subnation.fid);
                                }
                                _ = nation_tiles.remove(neighbor_id);
                                _ = costs.insert(*neighbor_id, total_cost);
                                _ = queue.push(*neighbor_id, Reverse(total_cost));
                            } // else we can't expand into this tile, and this line of spreading ends here.
                        }
        
        
                    }


                }

                let seat_town_id = seat.map(|(id,_)| id);

                let name = if let (Some(seat_town_id),true) = (seat_town_id,rng.gen_bool(0.5)) {
                    // name by town
                    let town = town_map.try_get(&seat_town_id)?;
                    town.name.clone()
                } else {
                    // new name by culture
                    let namer = Culture::get_namer(culture_data, namers)?;
                    namer.make_state_name(rng)                  
                };

                new_subnations.push((subnation.fid,NewSubnation {
                    name,
                    culture,
                    center_tile_id,
                    type_,
                    seat_town_id,
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
        let real_id = subnations_layer.add_subnation(&subnation)?;
        _ = assigned_ids.insert(temp_id, real_id);
    }

    let tiles_layer = target.edit_tile_layer()?;

    for (tile_id,temp_subnation_id) in tile_subnation_changes.into_iter().watch(progress,"Writing new subnations to tiles.","New subnations written to tiles.") {

        let mut tile = tiles_layer.try_feature_by_id(tile_id)?;

        let real_id = assigned_ids.get(&temp_subnation_id).expect("How would we use an id that we didn't add to the map?");

        tile.set_subnation_id(Some(*real_id))?;

        tiles_layer.update_feature(tile)?;

    }


    Ok(())
}

pub(crate) fn normalize_subnations<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let subnations_map = target.edit_subnations_layer()?.read_features().into_entities_index::<_,SubnationForNormalize>(progress)?;

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tile_list = Vec::new();
    let tile_map = tiles_layer.read_features().into_entities_index_for_each::<_,TileForSubnationNormalize,_>(|fid,_| {
        tile_list.push(*fid);
        Ok(())
    },progress)?;

    for tile_id in tile_list.into_iter().watch(progress,"Normalizing subnations.","Subnations normalized.") {
        let tile = tile_map.try_get(&tile_id)?;

        if tile.town_id.is_some() {
            continue; // don't overwrite towns
        }

        if let Some(subnation_id) = tile.subnation_id {
            // if the subnation doesn't have a seat, don't erase it's center tile.
            // (if it did have a seat, then it has towns, and the above check would hold it.)
            // This prevents very small subnations which were created with "Fill Empty" from being
            // deleted.
            let subnation = subnations_map.try_get(&(subnation_id))?;
            if subnation.seat_town_id.is_none() && tile_id == subnation.center_tile_id {
                continue;
            }
        }

        let mut adversaries = HashMap::new();
        let mut adversary_count = 0;
        let mut buddy_count = 0;
        for (neighbor_id,_) in &tile.neighbors {

            let neighbor = tile_map.try_get(neighbor_id)?;

            if neighbor.nation_id == tile.nation_id {
                if neighbor.subnation_id == tile.subnation_id {
                    buddy_count += 1;
                } else {
                    if let Some(count) = adversaries.get(&neighbor.subnation_id) {
                        _ = adversaries.insert(neighbor.subnation_id, count + 1)
                    } else {
                        _ = adversaries.insert(neighbor.subnation_id, 1)
                    };
                    adversary_count += 1;
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

        if let Some((worst_adversary,count)) = adversaries.into_iter().max_by_key(|(_,count)| *count).map(|(adversary,count)| (adversary,count)) {
            if count > buddy_count {
                let mut change_tile = tiles_layer.try_feature_by_id(tile_id)?;
                change_tile.set_subnation_id(worst_adversary)?;
                tiles_layer.update_feature(change_tile)?    
            }

        }

    }


    Ok(()) 
}

pub(crate) fn assign_subnation_colors<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, progress: &mut Progress) -> Result<(),CommandError> {

    let mut nation_color_index = target.edit_nations_layer()?.read_features().into_entities_index::<_,NationForSubnationColors>(progress)?;

    let mut subnations_layer = target.edit_subnations_layer()?;
    
    let subnations = subnations_layer.read_features().into_entities_vec::<_,SubnationForColors>(progress)?;

    for subnation in subnations.iter().watch(progress, "Counting subnations.", "Subnations counted.") {
        let nation = subnation.nation_id;
        nation_color_index.try_get_mut(&nation)?.subnation_count += 1;
    }

    // This will become an input to the generator so we can generate colors within the same ranges as the nations.
    let hue_range_split = RandomColorGenerator::split_hue_range_for_color_set(&None, nation_color_index.len());

    let mut nation_color_generator_index = HashMap::new();

    for (fid,entity) in nation_color_index.into_iter().watch(progress, "Generating colors.", "Colors generated.") {
        //let generator = RandomColorGenerator::from_rgb(&entity.color,Some(Luminosity::Light));
        let generator = RandomColorGenerator::from_rgb_in_split_hue_range(entity.color,&hue_range_split,Some(Luminosity::Light));
        _ = nation_color_generator_index.insert(fid, generator.generate_colors(entity.subnation_count, rng).into_iter());
    }

    for subnation in subnations.into_iter().watch(progress, "Assigning colors.", "Colors assigned.") {
        let generator = nation_color_generator_index.get_mut(&subnation.nation_id).expect("This was just added to the map, so it should still be there.");
        let mut feature = subnations_layer.try_feature_by_id(subnation.fid)?;
        feature.set_color(generator.next().expect("There should have been enough colors generated for everybody."))?;
        subnations_layer.update_feature(feature)?;

    }

    Ok(())
}