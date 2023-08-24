use std::collections::HashSet;
use std::collections::HashMap;

use rand::Rng;
use rand_distr::Normal;
use rand::distributions::Distribution;
use ordered_float::OrderedFloat;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
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
