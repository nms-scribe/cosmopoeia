use core::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;

use rand::Rng;
use priority_queue::PriorityQueue;
use ordered_float::OrderedFloat;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchablePriorityQueue;
use crate::world_map::WorldMapTransaction;
use crate::errors::CommandError;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::naming::NamerSet;
use crate::world_map::water_layers::LakeForCultureGen;
use crate::world_map::biome_layer::BiomeForCultureGen;
use crate::world_map::biome_layer::BiomeForCultureExpand;
use crate::world_map::tile_layer::TileForCultureGen;
use crate::world_map::tile_layer::TileForCulturePrefSorting;
use crate::world_map::tile_layer::TileForCultureExpand;
use crate::utils::random::RandomIndex;
use crate::utils::coordinates::Coordinates;
use crate::utils::ToRoman;
use crate::utils::world_shape::WorldShape;
use crate::world_map::fields::Grouping;
use crate::world_map::tile_layer::TileLayer;
use crate::world_map::fields::CultureType;
use crate::world_map::culture_layer::NewCulture;
use crate::world_map::culture_layer::CultureForPlacement;
use crate::typed_map::entities::EntityIndex;
use crate::world_map::water_layers::LakeSchema;
use crate::typed_map::entities::EntityLookup;
use crate::world_map::biome_layer::BiomeSchema;
use crate::algorithms::colors::RandomColorGenerator;
use crate::algorithms::colors::Luminosity;
use crate::commands::OverwriteCulturesArg;
use crate::commands::SizeVarianceArg;
use crate::commands::RiverThresholdArg;
use crate::commands::ExpansionFactorArg;
use crate::world_map::fields::NeighborAndDirection;
use crate::world_map::fields::Neighbor;

impl CultureType {

    fn generate_expansionism<Random: Rng>(&self, rng: &mut Random, size_variance: f64) -> f64 {
        let base = match self {
            Self::Lake => 0.8,
            Self::Naval | Self::Nomadic => 1.5,
            Self::River => 0.9,
            Self::Hunting => 0.7,
            Self::Highland => 1.2,
            Self::Generic => 1.0
        };
        ((rng.gen_range(0.0..1.0) * size_variance / 2.0) + 1.0) * base
    }
    
}



pub(crate) fn generate_cultures<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, culture_set: &CultureSet, namers: &NamerSet, culture_count: usize, size_variance: &SizeVarianceArg, river_threshold: &RiverThresholdArg, overwrite_layer: &OverwriteCulturesArg, progress: &mut Progress) -> Result<(),CommandError> {

    const MAX_ATTEMPTS: usize = 100;

    // Algorithm copied from AFMG

    let culture_count = if culture_count > culture_set.len() {
        progress.warning(|| format!("The provided culture set is not large enough to produce the requested number of cultures. The count will be limited to {}.",culture_set.len()));
        culture_set.len()
    } else {
        culture_count
    };

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    let biomes = target.edit_biomes_layer()?.read_features().into_named_entities_index(progress)?;

    let lake_map = target.edit_lakes_layer()?.read_features().into_entities_index::<_,LakeForCultureGen>(progress)?;

    let mut tile_layer = target.edit_tile_layer()?;

    let (max_habitability, mut populated) = get_culturable_tiles(&mut tile_layer, &biomes, &lake_map, progress)?;

    let culture_count = if populated.len() < (culture_count * 25) {
        let fixed_culture_count = populated.len().div_euclid(25);
        if fixed_culture_count == 0 {
            progress.warning(|| "There aren't enough habitable tiles to support urban societies. Only the 'wildlands' culture will be created.")
        } else {
            progress.warning(|| format!("There aren't enough habitiable tiles to support the requested number of cultures. The count will be limited to {fixed_culture_count}."))
        }
        fixed_culture_count

    } else {
        culture_count
    };

    let culture_sources = culture_set.select(rng,culture_count);

    let mut placed_centers = Vec::new();
    let mut cultures = Vec::new();

    let (width,height) = tile_layer.get_layer_size()?;
    let spacing = (width + height) / 2.0 / culture_count as f64;
    let max_tile_choice = populated.len().div_euclid(2);

    // I need to avoid duplicate names
    let mut culture_names = HashMap::new();

    let mut colors = RandomColorGenerator::new(None,Some(Luminosity::Light)).generate_colors(culture_count, rng).into_iter();

    for culture_source in culture_sources {

        // find the cultural center

        let preferences = culture_source.preferences();
        
        // sort so the most preferred tiles go to the top.
        // FUTURE: It would be nice if there were a try_sort_by_cached_key, but I don't expect
        // them to implement. Some sort of alternative standard solution for this sort of pattern
        // would be nice, though. A panic and catch_unwind would allow me to short-circuit the sort
        // algorithm, at least.
        let mut error = None;
        populated.sort_by_cached_key(|a| 
            match preferences.get_value(a,max_habitability) {
                Ok(value) => value,
                Err(err) => {
                    error = Some(err);
                    OrderedFloat::from(0.0)
                },
            });
        if let Some(err) = error {
            return Err(err)
        }

        let mut spacing = spacing;
        let mut i = 0;
        let center = loop {
            // FUTURE: Right now, this chooses randomly and increases the spacing until we've randomly hit upon a good spot,
            // the spacing has decreased until the too_close is always going to fail, or we just give up and take one. 
            // There might be a better way:
            // - start with a biased index, as with current
            // - if that doesn't work, choose another biased index, but set the min of the parameter to the previous index
            // - keep trying that until the choice >= max_tile_choice
            // - at that point I can do one of the following:
            //   - try decreasing spacing and trying the whole thing again
            //   - increase by one index until one is found that is outside of the spacing, keeping track of the furthest available
            //     tile during the process and choose that at the end
            let index = populated.choose_biased_index(rng,0,max_tile_choice,5);
            let center = &populated[index];
            if (i > MAX_ATTEMPTS) || !too_close(&placed_centers,&center.site,spacing,&world_shape) { 
                // return the removed tile, to prevent any other culture from matching it.
                break populated.remove(index);
            }
            // reduce spacing in case that's what the problem is
            spacing *= 0.9;
            i += 1;
        };
        placed_centers.push(center.site.clone());

        let name = culture_source.name().to_owned();

        // define the culture type
        let culture_type = get_culture_type(&center, river_threshold.river_threshold, rng);
        
        let expansionism = culture_type.generate_expansionism(rng,size_variance.size_variance);

        let namer = culture_source.namer_name();

        namers.check_exists(namer)?;

        let index = cultures.len();
        match culture_names.get_mut(&name) {
            None => {
                _ = culture_names.insert(name.clone(), vec![index]);
            },
            Some(indexes) => indexes.push(index),
        }

        cultures.push(NewCulture {
            name, 
            namer: namer.to_owned(),
            type_: culture_type,
            expansionism,
            center_tile_id: center.fid,
            color: colors.next().expect("There should have been just as many colors generated as cultures.")
        });
        
    }

    // now check the culture_names for duplicates and rename.
    for (_,indexes) in culture_names.into_iter().watch(progress,"Fixing culture names.","Culture names fixed.") {

        if indexes.len() > 1 {
            let mut suffix = 0;
            for index in indexes {
                suffix += 1;
                cultures[index].name += " ";
                cultures[index].name += &suffix.to_roman().unwrap_or_else(|| suffix.to_string());
            }

        }

    }

    // NOTE: AFMG Had a Wildlands culture that was automatically placed wherever there were no cultures.
    // However, that culture did not behave like other cultures. The option is to do this, have a
    // special culture that doesn't have a culture center, and doesn't behave like a culture, or to 
    // just allow tiles to not have a culture. I prefer the latter.
    // FUTURE: Actually, what I really prefer is to not have any populated place that doesn't have a culture.
    // It's pretty arrogant to say that a "wildlands" culture is special. However, to do that I'll have to
    // randomize hundreds to thousands of of random cultures with their own languages, etc. Such cultures
    // would have a very low expansionism.

    let mut cultures_layer = target.create_cultures_layer(overwrite_layer)?;

    for culture in cultures.iter().watch(progress,"Writing cultures.","Cultures written.") {

        _ = cultures_layer.add_culture(culture)?;

    }





    Ok(())
}

fn get_culturable_tiles<'biome_life, Progress: ProgressObserver>(tile_layer: &mut TileLayer, biomes: &'biome_life EntityLookup<BiomeSchema, BiomeForCultureGen>, lake_map: &EntityIndex<LakeSchema, LakeForCultureGen>, progress: &mut Progress) -> Result<(f64, Vec<TileForCulturePrefSorting<'biome_life>>), CommandError> {

    let mut max_habitability: f64 = 0.0;
    
    let mut populated = Vec::new();
    
    for tile in tile_layer.read_features().into_entities::<TileForCultureGen>().watch(progress,"Reading tiles.","Tiles read.") {
        let (_,tile) = tile?;
        if tile.population > 0 {
            max_habitability = max_habitability.max(tile.habitability);
            populated.push(tile);
        }
    }
    
    
    let mut sortable_populated = Vec::new();

    for tile in populated.into_iter().watch(progress,"Processing tiles for preference sorting.","Tiles processed.") {
        sortable_populated.push(TileForCulturePrefSorting::from(tile, &*tile_layer, biomes, lake_map)?);
    }

    Ok((max_habitability, sortable_populated))
}


fn get_culture_type<Random: Rng>(center: &TileForCulturePrefSorting, river_threshold: f64, rng: &mut Random) -> CultureType {
    if center.elevation_scaled < 70 && center.biome.supports_nomadic {
        return CultureType::Nomadic 
    } else if center.elevation_scaled > 50 {
        return CultureType::Highland
    }
    
    if let Some(water_count) = center.water_count {
        if let Some(neighboring_lake_size) = center.neighboring_lake_size {
            if neighboring_lake_size > 5 {
                return CultureType::Lake
            }
        }

        if (center.neighboring_lake_size.is_none() && rng.gen_bool(0.1)) || // on the ocean cost (on water cost and not on a lake)
           ((water_count == 1) && rng.gen_bool(0.6)) || // on exactly one water (makes a good harbor)
           (matches!(center.grouping,Grouping::Islet) && rng.gen_bool(0.4)) { // on a small island
            return CultureType::Naval
        }
    }
    
    if center.water_flow > river_threshold { 
        CultureType::River
    } else if center.shore_distance > 2 && center.biome.supports_hunting {
        CultureType::Hunting
    } else {
        CultureType::Generic
    }
}

fn too_close(point_vec: &Vec<Coordinates>, new_point: &Coordinates, spacing: f64, world_shape: &WorldShape) -> bool {
    // NOTE: While I could use some sort of quadtree/point-distance index, I don't feel like I'm going to deal with enough cultures
    // at any one point to worry about that.
    for point in point_vec {
        // FUTURE: This won't return true if the point's are across the antimeridian from each other, but I'm not sure how often this will
        // be a problem.
        if point.distance(new_point,world_shape) < spacing {
            return true;
        }
    }
    false
}


pub(crate) fn expand_cultures<Progress: ProgressObserver>(target: &mut WorldMapTransaction, river_threshold: &RiverThresholdArg, limit_factor: &ExpansionFactorArg, progress: &mut Progress) -> Result<(),CommandError> {

    let cultures = target.edit_cultures_layer()?.read_features().into_entities_vec::<_,CultureForPlacement>(progress)?;

    let biome_map = target.edit_biomes_layer()?.read_features().into_named_entities_index::<_,BiomeForCultureExpand>(progress)?;

    let mut tiles = target.edit_tile_layer()?;

    // we're working with a tile map, and completely overwriting whatever is there.
    let mut tile_map = tiles.read_features().into_entities_index::<_,TileForCultureExpand>(progress)?;

    // priority queue keeps tasks sorted by priority
    // Since I need to go for the least priorities first, I need the double queue to get pop_min
    let mut queue = PriorityQueue::new();

    // empty hashmap of tile ids
    let mut costs = HashMap::new();

    let tile_size = tiles.estimate_average_tile_area()?;

    // This is how far the cultures will be able to spread.
    // This is a arbitrary number, it basically limits the size of the culture to about 10,000 "square degrees". Although once
    // I get sherical directions and areas, I'll want to revisit this.
    let max_expansion_cost = OrderedFloat::from(10000.0/tile_size * limit_factor.expansion_factor);

    let mut culture_centers = HashSet::new();
    
    for culture in cultures {

        _ = culture_centers.insert(culture.center_tile_id.clone());

        // place the culture center
        let tile = tile_map.try_get_mut(&culture.center_tile_id)?;
        tile.culture = Some(culture.name.clone());

        // add the tile to the queue for work.
        _ = queue.push((culture.center_tile_id.clone(),culture,tile.biome.clone()), Reverse(OrderedFloat::from(0.0)));

    }

    let mut queue = queue.watch_queue(progress, "Expanding cultures.", "Cultures expanded.");

    while let Some(((tile_id, culture, culture_biome), priority)) = queue.pop() {

        let mut place_cultures = Vec::new();

        
        let tile = tile_map.try_get(&tile_id)?;

        for NeighborAndDirection(neighbor_id,_) in &tile.neighbors {
            
            match neighbor_id {
                Neighbor::Tile(neighbor_id) | Neighbor::CrossMap(neighbor_id,_) => {

                    if culture_centers.contains(neighbor_id) {
                        // don't overwrite a culture center
                        continue;
                    }

                    let neighbor = tile_map.try_get(neighbor_id)?;

                    let neighbor_biome = biome_map.try_get(&neighbor.biome)?;

                    let biome_cost = get_biome_cost(&culture_biome,neighbor_biome,&culture.type_);

                    // NOTE: AFMG Had a line that looked very much like this one. I don't know if that was what was intended or not, but
                    // from my view, this will always return 0.
                    // let biome_change_cost = if neighbor_biome == biome_map.get(&neighbor.biome) { 0 } else { 20 };

                    let height_cost = get_height_cost(neighbor, &culture.type_);

                    let river_cost = get_river_cost(neighbor, river_threshold.river_threshold, &culture.type_);

                    let type_cost = get_shore_cost(neighbor, &culture.type_);

                    let cell_cost = OrderedFloat::from(biome_cost /* + biome_change_cost */ + height_cost + river_cost + type_cost) / culture.expansionism;

                    let total_cost = priority.0 + cell_cost;

                    if total_cost <= max_expansion_cost {

                        // if no previous cost has been assigned for this tile, or if the total_cost is less than the previously assigned cost,
                        // then I can place or replace the culture with this one. This will remove cultures that were previously
                        // placed, and in theory could even wipe a culture off the map. (Although the previous culture placement
                        // may still be spreading, don't worry).
                        let replace_culture = if let Some(neighbor_cost) = costs.get(neighbor_id) {
                            &total_cost < neighbor_cost
                        } else {
                            true
                        };

                        if replace_culture {
                            // even if there's no population in the tile, if we can spread to it, mark it as the cultures.
                            // Otherwise, we get some "empty" areas around lakes.
                            place_cultures.push((neighbor_id.clone(),culture.name.clone()));
                            _ = costs.insert(neighbor_id.clone(), total_cost);

                            queue.push((neighbor_id.clone(), culture.clone(), culture_biome.clone()), Reverse(total_cost));

                        } // else we can't expand into this tile, and this line of spreading ends here.
                    } else {
                        // else we can't make it into this tile, so give up.

                        // FUTURE: If you ever need to debug cultures that seem to stop too early...
                        //if ["Roman I","Roman II","Roman IV"].contains(&culture.name.as_str()) {
                        //    println!("Culture {}",culture.name);
                        //    println!("   priority {}",priority);
                        //    println!("   culture biome {}",culture_biome);
                        //    println!("   neighbor biome {}",neighbor_biome.name);
                        //    println!("   biome_cost {}",biome_cost);
                        //    println!("   height_cost {}",height_cost);
                        //    println!("   river_cost {}",river_cost);
                        //    println!("   type_cost {}",type_cost);
                        //    println!("   total_cost {}",total_cost);
                        //}
        
        
                    }

                }
                Neighbor::OffMap(_) => {
                    // the neighbor is off the map. While it's possible the culture might extend, I can't map it. I'll just assume that
                    // it's a free-for-all out there beyond.
                }
            }


        }

        for (place_tile_id,place_culture) in place_cultures {
            tile_map.try_get_mut(&place_tile_id)?.culture = Some(place_culture);
        }


    }

    for (fid,tile) in tile_map.iter().watch(progress,"Writing cultures.","Cultures written.") {

        let mut feature = tiles.try_feature_by_id(fid)?;

        feature.set_culture(&tile.culture)?;

        tiles.update_feature(feature)?;

    }


    Ok(())
}

const fn get_shore_cost(neighbor: &TileForCultureExpand, culture_type: &CultureType) -> f64 {
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

fn get_river_cost(neighbor: &TileForCultureExpand, river_threshold: f64, culture_type: &CultureType) -> f64 {
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

fn get_height_cost(neighbor: &TileForCultureExpand, culture_type: &CultureType) -> f64 {
    match culture_type {
        CultureType::Lake => if neighbor.lake_id.is_some() {
            // low lake crossing penalty for lake cultures
            10.0
        } else if neighbor.grouping.is_water() {
            // general sea/lake crossing penalty
            neighbor.area * 6.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            30.0
        } else {
            0.0
        },
        CultureType::Naval => if neighbor.grouping.is_water() {
            // low water crossing penalty
            neighbor.area * 2.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            30.0
        } else {
            0.0
        },
        CultureType::Nomadic => if neighbor.grouping.is_water() {
            neighbor.area * 50.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            30.0
        } else {
            0.0
        },
        CultureType::Highland => if neighbor.grouping.is_water() {
            // general sea/lake corssing penalty
            neighbor.area * 6.0
        } else if neighbor.elevation_scaled < 44 {
            // big penalty for highlanders in lowlands
            3000.0
        } else if neighbor.elevation_scaled < 62 {
            // smaller but still big penalty for hills
            200.0
        } else {
            // no penalty in highlands
            0.0
        },
        CultureType::Generic |
        CultureType::River |
        CultureType::Hunting => if neighbor.grouping.is_water() {
            // general sea/lake corssing penalty
            neighbor.area * 6.0
        } else if neighbor.elevation_scaled >= 67 {
            // mountain crossing penalty
            200.0 
        } else if neighbor.elevation_scaled > 44 {
            // hill crossing penalt
            30.0
        } else {
            0.0
        }
    }
}

fn get_biome_cost(culture_biome: &String, neighbor_biome: &BiomeForCultureExpand, culture_type: &CultureType) -> f64 {
    // FUTURE: I need a way to make this more configurable...
    const FOREST_BIOMES: [&str; 5] = [BiomeSchema::TROPICAL_SEASONAL_FOREST, BiomeSchema::TEMPERATE_DECIDUOUS_FOREST, BiomeSchema::TROPICAL_RAINFOREST, BiomeSchema::TEMPERATE_RAINFOREST, BiomeSchema::TAIGA];

    
    if culture_biome == &neighbor_biome.name {
        // tiny penalty for native biome
        10.0
    } else {
        (match culture_type {
            CultureType::Hunting => neighbor_biome.movement_cost * 5, // non-native biome penalty for hunters
            CultureType::Nomadic => if FOREST_BIOMES.contains(&neighbor_biome.name.as_str()) {
                // penalty for forests
                neighbor_biome.movement_cost * 10
            } else {
                neighbor_biome.movement_cost * 2
            },
            CultureType::Generic |
            CultureType::Lake |
            CultureType::Naval |
            CultureType::River |
            CultureType::Highland => neighbor_biome.movement_cost * 2,
        }) as f64
    
    }

}