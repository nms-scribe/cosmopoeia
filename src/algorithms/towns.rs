use std::collections::HashSet;
use std::collections::HashMap;

use rand::Rng;
use ordered_float::OrderedFloat;
use rand_distr::Normal;
use rand_distr::Distribution;

use crate::world_map::town_layer::TownForPopulation;
use crate::world_map::water_layers::LakeForTownPopulation;
use crate::world_map::tile_layer::TileForTownPopulation;
use crate::utils::coordinates::Coordinates;
use crate::world_map::tile_layer::TileLayer;
use crate::utils::point_finder::PointFinder;
use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use crate::world_map::town_layer::NewTown;
use crate::errors::CommandError;
use crate::algorithms::naming::NamerSet;
use crate::world_map::WorldMapTransaction;
use crate::progress::WatchableIterator;
use crate::world_map::culture_layer::CultureWithNamer;
use crate::typed_map::entities::NamedEntity;
use crate::typed_map::features::TypedFeature;
use crate::progress::ProgressObserver;
use crate::world_map::tile_layer::TileForTowns;
use crate::world_map::culture_layer::CultureSchema;
use crate::typed_map::entities::EntityLookup;
use crate::commands::OverwriteTownsArg;
use crate::commands::RiverThresholdArg;
use crate::commands::TownCountsArg;
use crate::world_map::fields::Neighbor;
use crate::typed_map::fields::IdRef;

pub(crate) struct ScoredTileForTowns {
    tile: TileForTowns,
    capital_score: OrderedFloat<f64>,
    town_score: OrderedFloat<f64>
}


pub(crate) fn generate_towns<Random: Rng, Progress: ProgressObserver, Culture: NamedEntity<CultureSchema> + CultureWithNamer>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &EntityLookup<CultureSchema,Culture>, namers: &mut NamerSet, town_counts: &TownCountsArg, overwrite_layer: &OverwriteTownsArg, progress: &mut Progress) -> Result<(),CommandError> {

    // a lot of this is ported from AFMG

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tiles = gather_tiles_for_towns(rng, &mut tiles_layer, progress)?;

    let extent = tiles_layer.get_extent()?;

    let (capitals, capitals_finder) = generate_capitals(&mut tiles, &extent, &world_shape, town_counts.capital_count, progress);

    let towns = place_towns(rng, &mut tiles, &extent, capitals.len(), &town_counts.town_count, &capitals_finder, progress)?;

    // write the towns

    let mut towns_layer = target.create_towns_layer(overwrite_layer)?;

    let mut placed_towns = HashMap::new(); 
    for town in capitals.into_iter().chain(towns.into_iter()).watch(progress,"Writing towns.","Towns written.") {
        let (ScoredTileForTowns{tile,..},is_capital) = town;
        let culture = tile.culture();
        let namer = Culture::get_namer(culture.as_ref().map(|c| culture_lookup.try_get(c)).transpose()?, namers)?;
        let name = namer.make_name(rng);
        let fid = towns_layer.add_town(&NewTown {
            name,
            culture: culture.clone(),
            is_capital,
            tile_id: tile.fid().clone(),
            grouping_id: tile.grouping_id().clone(),
            population: 0,
            is_port: false
        },tile.site().create_geometry()?)?;
        _ = placed_towns.insert(tile.fid().clone(),fid); 
    }

    // even though we have the town locations indicated in the towns layer, there are going to be occasions
    // where I want to easily figure out if a tile has a town, so write that there as well.
    // FUTURE: If the user re-writes the towns with a different seed, there's going to be erroneous data in tiles.
    // Maybe delete it.
    let mut editing_tiles_layer = target.edit_tile_layer()?;
    // I have to update all tiles, otherwise we might have erroneous data from a previous run.
    let editing_tiles: Vec<IdRef> = editing_tiles_layer.read_features().watch(progress,"Reading tiles.","Tiles read.").map(|f| f.fid()).collect::<Result<Vec<_>,_>>()?;
    for fid in editing_tiles {
        let town = placed_towns.get(&fid);
        let mut tile = editing_tiles_layer.try_feature_by_id(&fid)?;
        tile.set_town_id(&town.cloned())?;
        editing_tiles_layer.update_feature(tile)?;
    }

    Ok(())
}

pub(crate) fn place_towns<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, tiles: &mut Vec<ScoredTileForTowns>, extent: &Extent, placed_capital_count: usize, town_count: &Option<usize>, capitals_finder: &PointFinder, progress: &mut Progress) -> Result<Vec<(ScoredTileForTowns, bool)>,CommandError> {
    let mut towns_finder;
    let mut town_cultures;
    let mut towns;

    let town_count = if let Some(town_count) = town_count {
        if town_count > &tiles.len() {
            let reduced_town_count = tiles.len() - placed_capital_count;
            if tiles.is_empty() {
                progress.warning(|| "There aren't enough populated cells left to generate any towns.")
            } else {
                progress.warning(|| format!("There aren't enough populated cells to generate the requested number of towns. Only {reduced_town_count} towns will be generated."))
            }
            reduced_town_count
        } else {
            *town_count
        }
    } else {
        let map_size = extent.width() * extent.height();
        let generated_town_count = (map_size/100.0).floor() as usize;
        let generated_town_count = generated_town_count.min(tiles.len() - placed_capital_count);
        if generated_town_count == 0 {
            progress.warning(|| "There aren't enough populated cells to generate towns.")
        }
        generated_town_count
    };

    let mut spacing = (extent.width() + extent.height()) / 150.0 / ((town_count as f64).powf(0.7)/66.0);
    let town_spacing_normal = Normal::new(1.0f64,0.3f64).expect("Why would these constants fail if they never did before?");
    // if this fails then it's a programming error, I'm pretty certain.

    macro_rules! reset_town_search {
        () => {
            towns_finder = PointFinder::fill_from(&capitals_finder,town_count)?;
            towns = vec![];
            town_cultures = HashSet::new();
            tiles.sort_by_key(|ScoredTileForTowns{town_score,..}| core::cmp::Reverse(*town_score));
        };
    }

    reset_town_search!();


    // we have to do this several times, adjusting the spacing as necessary
    loop {
        // can't use a for loop, because the range changes
        let i = 0;
        progress.start_known_endpoint(|| (format!("Placing towns at spacing {spacing}"),town_count));
        while (i < tiles.len()) && (towns.len() < town_count) {
            let candidate = &tiles[i];
            let s = spacing * town_spacing_normal.sample(rng).clamp(0.2,2.0);
            if !towns_finder.points_in_target(candidate.tile.site(), s) {
                let entry = tiles.remove(i);
                _ = town_cultures.insert(entry.tile.culture().clone());
                towns.push((entry,false)); // true means it's a capital
                progress.update(|| towns.len());
            }

        }

        if towns.len() < town_count {
            // reset everything, add what we found back to the tiles, and sort it again
            tiles.extend(towns.into_iter().map(|(a,_)| a));
            reset_town_search!();
            spacing /= 2.0;
            if spacing <= 1.0 {
                progress.finish(|| format!("Only {} towns could be placed.",towns.len()));
                break;
            }
            
            progress.finish(|| "Not enough towns could be placed, trying again with reduced spacing.");
        } else {
            progress.finish(|| "Towns placed.");
            break;
        }
    }
    Ok(towns)
}

pub(crate) fn generate_capitals<Progress: ProgressObserver>(tiles: &mut Vec<ScoredTileForTowns>, extent: &Extent, world_shape: &WorldShape, capital_count: Option<usize>, progress: &mut Progress) -> (Vec<(ScoredTileForTowns, bool)>, PointFinder) {
    let mut capitals_finder;
    let mut capitals;
    let mut capital_cultures;


    let capital_count = if let Some(capital_count) = capital_count {
        if tiles.len() < capital_count {
            let fixed_capital_count = tiles.len();
            if fixed_capital_count == 0 {
                progress.warning(|| "There aren't enough populated cells to generate national capitals.")
            } else {
                progress.warning(|| format!("There aren't enough populated cells to generate the requested number of national capitals. Only {fixed_capital_count} capitals will be generated."))
            }
            fixed_capital_count
        } else {
            capital_count
        }
    } else {
        let map_size = extent.width() * extent.height();
        let generated_capital_count = (map_size/1000.0).floor() as usize;
        let generated_capital_count = generated_capital_count.min(tiles.len());
        if generated_capital_count == 0 {
            progress.warning(|| "There aren't enough populated cells to generate national capitals.")
        }
        generated_capital_count
    };

    let mut spacing = (extent.width() + extent.height()) / 2.0 / capital_count as f64;

    macro_rules! reset_capital_search {
        () => {
            // this is a 2d index of points
            capitals_finder = PointFinder::new(&extent,world_shape.clone(),capital_count);
            capitals = vec![];
            capital_cultures = HashSet::new();
            // sort the tiles so highest scores is at 0
            tiles.sort_by_key(|ScoredTileForTowns{capital_score,..}| core::cmp::Reverse(*capital_score));
        };
    }

    reset_capital_search!();

    // we have to do this several times, adjusting the spacing as necessary
    loop {
        // can't use a for loop, because the range changes
        let i = 0;
        progress.start_known_endpoint(|| (format!("Placing capitals at spacing {spacing}"),capital_count));
        while (i < tiles.len()) && (capitals.len() < capital_count) {
            let candidate = &tiles[i];
            if !capitals_finder.points_in_target(candidate.tile.site(), spacing) {
                let entry = tiles.remove(i);
                _ = capital_cultures.insert(entry.tile.culture().clone());
                capitals.push((entry,true)); // true means it's a capital
                progress.update(|| capitals.len());
            }

        }

        if capitals.len() < capital_count {
            progress.finish(|| "Not enough capitals could be placed, trying again with reduced spacing.");
            // reset everything, add what we found back to the tiles, and sort it again
            tiles.extend(capitals.into_iter().map(|(a,_)| a));
            reset_capital_search!();
            spacing /= 1.2;
        } else {
            progress.finish(|| "Capitals placed.");
            break;
        }
    }
    (capitals, capitals_finder)
}

pub(crate) fn gather_tiles_for_towns<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, tiles_layer: &mut TileLayer, progress: &mut Progress) -> Result<Vec<ScoredTileForTowns>, CommandError> {

    let town_score_normal = Normal::new(1.0f64,3.0f64).expect("Why would these constants fail if they never did before?");

    let mut tiles = vec![];

    for tile in tiles_layer.read_features().into_entities::<TileForTowns>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (_,tile) = tile?;
        if tile.habitability() > &0.0 {
            let capital_score = tile.habitability() * rng.gen_range(0.0f64..1.0f64).mul_add(0.5, 0.5);
            let town_score = tile.habitability() * town_score_normal.sample(rng).clamp(0.0,20.0);
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

pub(crate) fn populate_towns<Progress: ProgressObserver>(target: &mut WorldMapTransaction, river_threshold: &RiverThresholdArg, progress: &mut Progress) -> Result<(),CommandError> {

    struct TownDetails {
        population: i32,
        is_port: bool,
        new_location: Option<Coordinates>
    }

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    let mut tile_layer = target.edit_tile_layer()?;
    let extent = tile_layer.get_extent()?;

    let tile_map = tile_layer.read_features().into_entities_index::<_,TileForTownPopulation>(progress)?;

    let mut lake_layer = target.edit_lakes_layer()?;

    let lake_map = lake_layer.read_features().into_entities_index::<_,LakeForTownPopulation>(progress)?;

    let mut coastal_towns = HashMap::new();

    let mut town_details = HashMap::new();

    let mut towns_layer = target.edit_towns_layer()?;

    for town in towns_layer.read_features().into_entities::<TownForPopulation>().watch(progress,"Populating towns.","Towns populated.") {
        let (_,town) = town?;
        let tile = tile_map.try_get(town.tile_id())?;

        // figure out if it's a port
        let port_location = if let Some(closest_water) = &tile.harbor_tile_id() {
            match closest_water {
                neighbor @ (Neighbor::Tile(closest_water) | Neighbor::CrossMap(closest_water, _)) => {
                    let harbor = tile_map.try_get(closest_water)?;

                    // add it to the map of towns by feature for removing port status later.
                    match coastal_towns.get_mut(harbor.grouping_id()) {
                        None => _ = coastal_towns.insert(harbor.grouping_id().clone(), vec![town.fid().clone()]),
                        Some(entry) => entry.push(town.fid().clone()),
                    }
        
                    // no ports if the water is frozen
                    if harbor.temperature() > &0.0 {
                        let on_large_water = if let Some(lake_id) = &harbor.lake_id() {
                            // don't make it a port if the lake is only 1 tile big
                            let lake = lake_map.try_get(lake_id)?;
                            lake.size() > &1
                        } else {
                            harbor.grouping().is_ocean()
                        };
        
                        // it's a port if it's on the large water and either it's a capital or has a good harbor (only one water tile next to it)
                        if on_large_water && (*town.is_capital() || matches!(tile.water_count(),Some(1))) {
                            match neighbor {
                                Neighbor::Tile(_) => Some(tile.find_middle_point_between(harbor,&world_shape)?),
                                Neighbor::CrossMap(_, edge) => Some(tile.find_middle_point_on_edge(edge,&extent,&world_shape)?),
                                Neighbor::OffMap(_) => unreachable!("`neighbor` was only matched with Tile and CrossMap."),
                            }
                            
                        } else {
                            None
                        }
                    } else {
                        None
                    }                    
                },
                Neighbor::OffMap(_) => unreachable!("Why would there be an offmap harbor?"),
            }


        } else {
            None
        };

        // figure out it's population -- habitability is already divided by 5, so this makes it 10% of true suitability for people.
        // FUTURE: The population should be increased by the road traffic, but that could be done in the road generating stuff
        let population = ((tile.habitability() / 2.0) * 1000.0).max(100.0); 

        let population = if *town.is_capital() {
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

        let (is_port,new_location) = if port_location.is_none() && tile.water_flow() > &river_threshold.river_threshold {
            let shift = (tile.water_flow() / 150.0).min(1.0);
            let (tile_x,tile_y) = tile.site().to_tuple();
            let x = if (tile_x % 2.0) < 1.0 { tile_x + shift } else { tile_x - shift };
            let y = if (tile_y % 2.0) < 1.0 { tile_y + shift } else { tile_y - shift };
            (false,Some(Coordinates::try_from((x,y))?))
        } else {
            (port_location.is_some(),port_location)
        };


        _ = town_details.insert(town.fid().clone(),TownDetails { 
            population, 
            is_port, 
            new_location 
        });
    }

    // remove port status if there's only one on the feature, but still get the benefits
    for list in coastal_towns.values().watch(progress,"Validating ports.","Ports validated.") {
        if list.len() == 1 {
            town_details.get_mut(&list[0]).expect("Why would this get fail if the list was built from the same thing generating the keys?").is_port = false
        }
    }

    for (fid,town) in town_details.into_iter().watch(progress,"Writing town details.","Town details written.") {
        let mut town_feature = towns_layer.try_feature_by_id(&fid)?;
        if let Some(new_location) = town.new_location {
            town_feature.move_to(&new_location)?;
        }
        town_feature.set_population(&town.population)?;
        town_feature.set_is_port(&town.is_port)?;
        towns_layer.update_feature(town_feature)?;
    }


    Ok(())
}
