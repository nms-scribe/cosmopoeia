use std::collections::HashSet;
use std::collections::HashMap;

use rand::Rng;
use rand_distr::Normal;
use rand::distributions::Distribution;
use ordered_float::OrderedFloat;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::WorldMapTransaction;
use crate::errors::CommandError;
use crate::algorithms::naming::Namer;
use crate::world_map::TileForTowns;
use crate::utils::point_finder::PointFinder;
use crate::world_map::CultureForTowns;
use crate::world_map::NewTown;


pub(crate) fn generate_towns<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, culture_lookup: &HashMap<String,CultureForTowns>, namers: &mut HashMap<String,Namer>, default_namer: &str, capital_count: usize, town_count: Option<usize>, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    // a lot of this is ported from AFMG

    let mut capital_count = capital_count;

    let mut tiles_layer = target.edit_tile_layer()?;

    let mut tiles = vec![];

    let town_score_normal = Normal::new(1.0f64,3.0f64).unwrap(); // if this fails then it's a programming error, I'm pretty certain.

    for tile in tiles_layer.read_features().into_entities::<TileForTowns>().watch(progress, "Reading tiles.", "Tiles read.") {
        let (_,tile) = tile?;
        if tile.habitability > 0.0 {
            let capital_score = tile.habitability * (0.5 + rng.gen_range(0.0..1.0) * 0.5);
            let town_score = tile.habitability * town_score_normal.sample(rng).clamp(0.0,20.0);
            if (capital_score > 0.0) || (town_score > 0.0) {
                tiles.push((tile,OrderedFloat::from(capital_score),OrderedFloat::from(town_score)))
            }
    
        }

    }


    if tiles.len() < (capital_count * 10) {
        capital_count = tiles.len() / 10;
        if capital_count == 0 {
            progress.warning(|| "There aren't enough populated cells to generate national capitals. Other towns will still be generated.")
        } else {
            progress.warning(|| format!("There aren't enough populated cells to generate the requested number of national capitals. Only {} capitals will be generated.",capital_count))
        }
    }

    let extent = tiles_layer.get_extent()?;
    let mut spacing = (extent.width + extent.height) / 2.0 / capital_count as f64;

    let mut capitals_finder;
    let mut capitals;
    let mut capital_cultures;

    macro_rules! reset_capital_search {
        () => {
            // this is a 2d index of points
            capitals_finder = PointFinder::new(&extent,capital_count);
            capitals = vec![];
            capital_cultures = HashSet::new();
            // sort the tiles so highest scores is at 0
            tiles.sort_by_key(|(_,capital_score,_)| std::cmp::Reverse(*capital_score));
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
            if !capitals_finder.points_in_target(&entry.0.site, spacing) {
                let entry = tiles.remove(i);
                capital_cultures.insert(entry.0.culture.clone());
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
        tiles.len() / 5 / ((tiles_layer.feature_count() / 10000) as f64).powf(0.8).round() as usize
    };

    let mut spacing = (extent.width + extent.height) / 150.0 / ((town_count as f64).powf(0.7)/66.0);
    let town_spacing_normal = Normal::new(1.0f64,0.3f64).unwrap(); // if this fails then it's a programming error, I'm pretty certain.
    let mut towns_finder;
    let mut town_cultures;
    let mut towns;
    tiles.sort_by_key(|(_,_,town_score)| std::cmp::Reverse(*town_score));

    macro_rules! reset_town_search {
        () => {
            towns_finder = PointFinder::fill_from(&capitals_finder,town_count)?;
            towns = vec![];
            town_cultures = HashSet::new();
            tiles.sort_by_key(|(_,_,town_score)| std::cmp::Reverse(*town_score));    
        };
    }

    reset_town_search!();


     // we have to do this several times, adjusting the spacing as necessary
    loop {
        // can't use a for loop, because the range changes
        let i = 0;
        progress.start_known_endpoint(|| (format!("Placing towns at spacing {}",spacing),capital_count));
        while (i < tiles.len()) && (towns.len() < town_count) {
            let entry = &tiles[i];
            let s = spacing * town_spacing_normal.sample(rng).clamp(0.2,2.0);
            if !towns_finder.points_in_target(&entry.0.site, s) {
                let entry = tiles.remove(i);
                town_cultures.insert(entry.0.culture.clone());
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

    // load the namers

    let mut towns_layer = target.create_towns_layer(overwrite_layer)?;

    //let mut placed_towns = vec![];
    for town in capitals.into_iter().chain(towns.into_iter()).watch(progress,"Writing towns.","Towns written.") {
        let ((tile,_,_),is_capital) = town;
        let culture = tile.culture;
        let namer = if let Some(namer) = culture.as_ref().and_then(|culture| culture_lookup.get(culture)).map(|culture| &culture.namer) {
            namer
        } else {
            default_namer
        };
        let namer = namers.get_mut(namer).ok_or_else(|| CommandError::UnknownNamer(namer.to_owned()))?; // we should have guaranteed that this was loaded.
        let name = namer.make_name(rng);
        let _fid = towns_layer.add_town(NewTown {
            geometry: tile.site.create_geometry()?,
            name,
            culture,
            is_capital,
            tile_id: tile.fid as i64,
            grouping_id: tile.grouping_id
        });
        //placed_towns.push((tile.fid,fid));
    }

    // TODO: Do we need to do this? It might make things a little easier, but we already have a link to the tile from towns.
    // TODO: Or do we want the link from tiles to town instead?
    /*/
    let tiles_layer = target.edit_tile_layer()?;
    for (tile_fid,town_fid) in placed_towns.into_iter().watch(progress,"Writing towns to tiles.","Towns written on tiles.") {
        let tile = tiles_layer.try_feature_by_id(&tile_fid)?;
        tile.set_town(Some(town_fid));
        tiles_layer.update_feature(tile);
    }
    */

    Ok(())
}