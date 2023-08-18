use std::collections::HashMap;

use rand::Rng;

use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::errors::CommandError;
use crate::algorithms::culture_sets::CultureSet;
use crate::algorithms::naming::NamerSet;
use crate::world_map::LakeDataForCultures;
use crate::world_map::BiomeDataForCultures;
use crate::world_map::TileCultureWork;
use crate::world_map::TileCultureWorkForPreferenceSorting;
use crate::utils::RandomIndex;
use crate::utils::Point;
use crate::utils::ToRoman;
use crate::world_map::Terrain;
use crate::world_map::TilesLayer;
use crate::world_map::CultureType;
use crate::world_map::NewCulture;
use crate::world_map::CultureForPlacement;

impl CultureType {

    fn generate_expansionism<Random: Rng>(&self, rng: &mut Random, size_variance: f64) -> f64 {
        let base = match self {
            Self::Lake => 0.8,
            Self::Naval => 1.5,
            Self::River => 0.9,
            Self::Nomadic => 1.5,
            Self::Hunting => 0.7,
            Self::Highland => 1.2,
            Self::Generic => 1.0
        };
        ((rng.gen_range(0.0..1.0) * size_variance / 2.0) + 1.0) * base
    }
    
}



pub(crate) fn generate_cultures<Random: Rng, Progress: ProgressObserver>(target: &mut WorldMapTransaction, rng: &mut Random, culture_set: CultureSet, namers: NamerSet, culture_count: usize, size_variance: f64, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm copied from AFMG

    let culture_count = if culture_count > culture_set.len() {
        progress.warning(|| "The provided culture set is not large enough to produce the requested number of cultures. The count will be limited.");
        culture_set.len()
    } else {
        culture_count
    };

    let biomes = target.edit_biomes_layer()?.build_index(progress)?;

    let lake_map = target.edit_lakes_layer()?.read_features().to_entities_index::<_,LakeDataForCultures>(progress)?;

    let mut tile_layer = target.edit_tile_layer()?;

    let (max_habitability, mut populated) = get_culturable_tiles(&mut tile_layer, &biomes, &lake_map, progress)?;

    let culture_count = if populated.len() < (culture_count * 25) {
        let culture_count = populated.len()/25;
        if culture_count == 0 {
            progress.warning(|| "There aren't enough habitable tiles to support urban societies. Only the 'wildlands' culture will be created.")
        } else {
            progress.warning(|| "There aren't enough habitiable tiles to support the requested number of cultures. The count will be limited.")
        }
        culture_count

    } else {
        culture_count
    };


    let culture_sources = culture_set.select(rng,culture_count);
    // TODO: Make sure to add the wildlands culture on and fill up the empties at the end if there are any.

    let mut placed_centers = Vec::new();
    let mut cultures = Vec::new();

    let (width,height) = tile_layer.get_layer_size()?;
    let spacing = (width + height) / 2.0 / culture_count as f64;
    let max_tile_choice = populated.len() / 2;
    const MAX_ATTEMPTS: usize = 100; // FUTURE: Configure?

    // I need to avoid duplicate names
    let mut culture_names = HashMap::new();

    for culture_source in culture_sources {

        // find the cultural center

        let preferences = culture_source.preferences();
        
        // sort so the most preferred tiles go to the top.
        populated.sort_by_cached_key(|a| preferences.get_value(a,max_habitability));
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
            if (i > MAX_ATTEMPTS) || !too_close(&placed_centers,&center.site,spacing) { 
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
        // TODO: This will be much simpler if it's in a function and early return
        let culture_type = get_culture_type(&center, rng)?;
        
        let expansionism = culture_type.generate_expansionism(rng,size_variance);

        let namer = culture_source.namer_name();

        namers.check(namer)?;

        let index = cultures.len();
        // TODO: This seems like a more efficient way to do this, instead of entry, since I only clone if the name is inserted
        // TODO: Change the other usages to use this if I can.
        match culture_names.get_mut(&name) {
            None => {
                culture_names.insert(name.clone(), vec![index]);
            },
            Some(indexes) => indexes.push(index),
        }

        cultures.push(NewCulture {
            name, 
            namer: namer.to_owned(),
            type_: culture_type,
            expansionism,
            center: center.fid as i64,
        });
        
    }

    progress.start_known_endpoint(|| ("Fixing culture names.",culture_names.len()));
    // now check the culture_names for duplicates and rename.
    for (i,(_,indexes)) in culture_names.into_iter().enumerate() {

        if indexes.len() > 1 {
            let mut suffix = 0;
            for index in indexes {
                suffix += 1;
                cultures[index].name += " ";
                cultures[index].name += &suffix.to_roman().unwrap_or_else(|| suffix.to_string());
            }

        }


        progress.update(|| i)
    }

    progress.finish(|| "Culture names fixed.");

    // NOTE: AFMG Had a Wildlands culture that was automatically placed wherever there were no cultures.
    // However, that culture did not behave like other cultures. The option is to do this, have a
    // special culture that doesn't have a culture center, and doesn't behave like a culture, or to 
    // just allow tiles to not have a culture. I prefer the latter.
    // FUTURE: Actually, what I really prefer is to not have any populated place that doesn't have a culture.
    // It's pretty arrogant to say that a "wildlands" culture is special. However, to do that I'll have to
    // randomize hundreds to thousands of of random cultures with their own languages, etc. Such cultures
    // would have a very low expansionism.

    progress.start_known_endpoint(|| ("Writing cultures.",cultures.len()));
    let mut cultures_layer = target.create_cultures_layer(overwrite_layer)?;

    for (i,culture) in cultures.iter().enumerate() {

        cultures_layer.add_culture(culture)?;

        progress.update(|| i);
    }

    progress.finish(|| "Cultures written.");





    Ok(())
}

fn get_culturable_tiles<'biome_life, Progress: ProgressObserver>(tile_layer: &mut TilesLayer, biomes: &'biome_life HashMap<String, BiomeDataForCultures>, lake_map: &HashMap<u64, LakeDataForCultures>, progress: &mut Progress) -> Result<(f64, Vec<TileCultureWorkForPreferenceSorting<'biome_life>>), CommandError> {

    let mut max_habitability: f64 = 0.0;
    
    let mut populated = Vec::new();
    
    progress.start_known_endpoint(|| ("Reading tiles.",tile_layer.feature_count()));
    
    for (i,tile) in tile_layer.read_features().into_entities::<TileCultureWork>().enumerate() {
        let (_,tile) = tile?;
        if tile.population > 0 {
            max_habitability = max_habitability.max(tile.habitability);
            populated.push(tile);
        }
        progress.update(|| i);
    }
    
    progress.finish(|| "Tiles read.");
    
    progress.start_known_endpoint(|| ("Processing tiles for preference sorting",populated.len()));

    let mut sortable_populated = Vec::new();

    for (i,tile) in populated.into_iter().enumerate() {
        sortable_populated.push(TileCultureWorkForPreferenceSorting::from(tile, &*tile_layer, &biomes, &lake_map)?);
        progress.update(|| i);
    }

    progress.finish(|| "Tiles processed.");

    Ok((max_habitability, sortable_populated))
}


fn get_culture_type<Random: Rng>(center: &TileCultureWorkForPreferenceSorting, rng: &mut Random) -> Result<CultureType, CommandError> {
    if center.elevation_scaled < 70 && center.biome.supports_nomadic {
        return Ok(CultureType::Nomadic) // TODO: These should be an enum eventually.
    } else if center.elevation_scaled > 50 {
        return Ok(CultureType::Highland)
    }

    if let Some(water_count) = center.water_count {
        if let Some(neighboring_lake_size) = center.neighboring_lake_size {
            if neighboring_lake_size > 5 {
                return Ok(CultureType::Lake)
            }
        }

        if (center.neighboring_lake_size.is_none() && rng.gen_bool(0.1)) || // on the ocean cost (on water cost and not on a lake)
           ((water_count == 1) && rng.gen_bool(0.6)) || // on exactly one water (makes a good harbor)
           (matches!(center.terrain,Terrain::Islet) && rng.gen_bool(0.4)) { // on a small island
            return Ok(CultureType::Naval)
        }
    }
    
    if center.water_flow > 100.0 { // TODO: Is this the right value?
        return Ok(CultureType::River)
    } else if center.shore_distance > 2 && center.biome.supports_hunting {
        return Ok(CultureType::Hunting)
    } else {
        return Ok(CultureType::Generic)
    }
}

fn too_close(point_vec: &Vec<Point>, new_point: &Point, spacing: f64) -> bool {
    // NOTE: While I could use some sort of quadtree/point-distance index, I don't feel like I'm going to deal with enough cultures
    // at any one point to worry about that.
    for point in point_vec {
        if point.distance(new_point) < spacing {
            return true;
        }
    }
    return false;
}


pub(crate) fn place_cultures<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let cultures = target.edit_cultures_layer()?.read_features().to_entities_vec::<_,CultureForPlacement>(progress)?;

    let tiles = target.edit_tile_layer()?;

    for culture in cultures {

        let mut tile = tiles.try_feature_by_id(&(culture.center as u64))?;

        tile.set_culture(Some(&culture.name))?;

        tiles.update_feature(tile)?;
    }

    Ok(())
}