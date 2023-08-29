use std::collections::HashMap;

use gdal::vector::OGRwkbGeometryType;
use gdal::vector::Geometry;

use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::errors::CommandError;
use crate::world_map::NewTile;
use crate::world_map::TypedFeature;
use crate::world_map::TileWithNeighborsElevation;
use crate::world_map::TilesLayer;
use crate::utils::Point;
use crate::utils::bezierify_polygon;
use crate::utils::multipolygon_to_polygons;
use crate::gdal_fixes::GeometryFix;
use crate::world_map::TileForCultureDissolve;
use crate::utils::force_multipolygon;
use crate::world_map::CultureForDissolve;
use crate::world_map::TileWithGeometry;
use crate::world_map::TileWithShoreDistance;
use crate::world_map::TileWithNeighbors;
use crate::world_map::TileFeature;
use crate::world_map::Entity;
use crate::world_map::Schema;
use crate::world_map::CultureSchema;
use crate::world_map::TileSchema;
use crate::world_map::EntityIndex;
use crate::world_map::EntityLookup;

pub(crate) fn load_tile_layer<Generator: Iterator<Item=Result<NewTile,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target = target.create_tile_layer(overwrite_layer)?;

    for tile in generator.watch(progress,"Writing tiles.","Tiles written.") {
        target.add_tile(tile?)?;
    }

    Ok(())

}

pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    // TODO: Would it speed things up to make use of a QuadTree structure? Index all the vertices, then just look for tiles that share vertices. It might
    // be worth trying, as it would avoid the disjoint check.

    //use std::time::Instant;

    //let mut time_map = HashMap::new();

    macro_rules! mark_time {
        {$name: literal: $($block: tt)*} => {
            // NOTE: Uncomment these lines to do some benchmarking
            // let now = Instant::now();
            $($block)*
            //match time_map.entry($name) {
            //    std::collections::hash_map::Entry::Occupied(mut entry) => *entry.get_mut() += now.elapsed().as_secs_f64(),
            //    std::collections::hash_map::Entry::Vacant(entry) => {entry.insert(now.elapsed().as_secs_f64());},
            //}
            
        };
    }

    // TODO: Memory usage for this isn't much more than other algorithms. The problem is almost entirely time.
    // There's CPU usage as well, but the best way to reduce that is to reduce the time (I can reduce usage by sleeping for occasional milliseconds, but that
    // slows the whole process down.)
    let mut layer = target.edit_tile_layer()?;
    
    mark_time!{"reading tiles": 
        let mut features = Vec::new();
        for feature in layer.read_features().watch(progress,"Reading tiles.","Tiles read.") {
            features.push(feature.fid()?);
        }
    };

    // This cache of the disjoint result speeds this algorithm up by about one second (6.43 down to 5.45)
    // A significant but still disappointing improvement. (There were less duplicate checks than I expected)
    let mut disjoint_checked = HashMap::new();


    // # Loop through all features and find features that touch each feature
    // for f in feature_dict.values():
    for working_fid in features.iter().watch(progress,"Calculating neighbors.","Neighbors calculated.") {

        mark_time!{"feature geometry":
            let (site_x,site_y,envelope,working_geometry) = { // shelter mutable borrow

                let feature = layer.feature_by_id(&working_fid).unwrap();
                let working_geometry = feature.geometry()?;
                let envelope = working_geometry.envelope();
                (feature.site_x()?,feature.site_y()?,envelope,working_geometry.clone())

            };
        }

        mark_time!{"set_spatial_filter_rect":
            layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);
        }

        let mut neighbors = Vec::new();

        mark_time!{"intersecting features":
            for intersecting_feature in layer.read_features() {

                mark_time!{"intersecting features: get fid":
                    let intersecting_fid = intersecting_feature.fid()?;
                }

                if working_fid != &intersecting_fid {

                    mark_time!{"intersection features: get disjoint cache":
                        let cached = disjoint_checked.get(&(*working_fid,intersecting_fid));
                    }
                    
                    let is_disjoint = if let Some(is_disjoint) = cached {
                        // check the cache in the opposite way they were inserted, because they were inserted when working_id
                        // was intersecting_id
                        *is_disjoint
                    } else {
                        // NOTE: this is by far the slowest part of the process, apart from updating the feature. I can think of nothing to do to optimize this.
                        mark_time!{"intersecting features: disjoint check":
                            let is_disjoint = intersecting_feature.geometry()?.disjoint(&working_geometry);
                        }
                        disjoint_checked.insert((intersecting_fid,*working_fid),is_disjoint);
                        is_disjoint

                    };

                    if !is_disjoint {
                        mark_time!{"intersecting features: neighbor geometry":
                            let neighbor_site_x = intersecting_feature.site_x()?;
                            let neighbor_site_y = intersecting_feature.site_y()?;
                        }

                        mark_time!{"intersecting features: neighbor angle":
                            let neighbor_angle = {
                                let (site_x,site_y,neighbor_site_x,neighbor_site_y) = (site_x,site_y,neighbor_site_x,neighbor_site_y);
                                // needs to be clockwise, from the north, with a value from 0..360
                                // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                                let counter_clockwise_from_east = ((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round();
                                // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                                let clockwise_from_north = 450.0 - counter_clockwise_from_east; 
                                // And then, to get the values in the range from 0..360, mod it.
                                let clamped = clockwise_from_north % 360.0;
                                clamped
                            };
                        }
                
                        mark_time!{"intersecting features: push neighbor":
                            neighbors.push((intersecting_fid,neighbor_angle.floor() as i32)); 
                        }

                    }

                }

            }
        }

        mark_time!{"clear_spatial_filter":
            layer.clear_spatial_filter();
        }

        mark_time!{"update feature":
            if let Some(mut working_feature) = layer.feature_by_id(&working_fid) {
                working_feature.set_neighbors(&neighbors)?;

                layer.update_feature(working_feature)?;

            }
        }


    }

    //println!("{:#?}",time_map);
    
    Ok(())

}


pub(crate) fn find_lowest_neighbors<Data: TileWithNeighborsElevation>(entity: &Data, tile_map: &EntityIndex<TileSchema,Data>) -> Result<(Vec<u64>, Option<f64>),CommandError> {
    let mut lowest = Vec::new();
    let mut lowest_elevation = None;

    // find the lowest neighbors
    for (neighbor_fid,_) in entity.neighbors() {
        let neighbor = tile_map.try_get(&neighbor_fid)?;
        let neighbor_elevation = neighbor.elevation();
        if let Some(lowest_elevation) = lowest_elevation.as_mut() {
            if neighbor_elevation < *lowest_elevation {
                *lowest_elevation = neighbor_elevation;
                lowest = vec![*neighbor_fid];
            } else if neighbor_elevation == *lowest_elevation {
                lowest.push(*neighbor_fid)
            }
        } else {
            lowest_elevation = Some(neighbor_elevation);
            lowest.push(*neighbor_fid)
        }

    }
    Ok((lowest,lowest_elevation.copied()))

}

pub(crate) fn find_tile_site_point(tile: Option<u64>, tiles: &TilesLayer<'_,'_>) -> Result<Option<Point>, CommandError> {
    Ok(if let Some(x) = tile {
        if let Some(x) = tiles.feature_by_id(&x) {
            Some(x.site()?)
        } else {
            None
        }
    } else {
        None
    })
}

pub(crate) fn calculate_coastline<Progress: ProgressObserver>(target: &mut WorldMapTransaction, bezier_scale: f64, overwrite_coastline: bool, overwrite_ocean: bool, progress: &mut Progress) -> Result<(),CommandError> {

    // TODO: In theory, I could write an ocean_id or coastline_id onto tiles if I really wanted to. This might take the place of grouping_id. In fact,
    // I could determine almost every grouping type in this algorithm, except for lake and lake_island. If I could then generate those two in fill 
    // lakes, then I could get rid of the grouping task.

    // FUTURE: There is an issue with coastlines extending over the edge of the borders after curving. I will have to deal with these someday.
    // FUTURE: After curving, towns which are along the coastline will sometimes now be in the ocean. I may need to deal with that as well, someday.

    let mut tile_layer = target.edit_tile_layer()?;

    let mut iterator = tile_layer.read_features().filter_map(|f| {
        match f.grouping() {
            Ok(g) if !g.is_ocean() => Some(Ok(f)),
            Ok(_) => None,
            Err(err) => Some(Err(err)),
        }
    } ).watch(progress, "Gathering tiles.", "Tiles gathered.");

    let tile_union = if let Some(tile) = iterator.next() {
        let first_tile = tile?.geometry()?.clone(); // TODO: Make sure to change 'geometry' to just return an error, then test it with the other stuff after and replace try_geometry with geometry.

        // it's much faster to union two geometries rather than union them one at a time.
        let mut next_tiles = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
        while let Some(tile) = iterator.next() {
            next_tiles.add_geometry(tile?.geometry()?.clone())?;
        }
        progress.start_unknown_endpoint(|| "Uniting tiles.");
        // TODO: Check it, though, that it's doing the right thing here, and not just adding the first_tile to the multipolygon.
        let tile_union = first_tile.union(&next_tiles);
        progress.finish(|| "Tiles united.");
        tile_union
    } else {
        None
    };

    progress.start_unknown_endpoint(|| "Creating ocean polygon.");
    // Create base ocean tile before differences.
    let ocean = tile_layer.get_extent()?.create_boundary_geometry()?; 
    progress.finish(|| "Ocean polygon created.");

    let (land_polygons,ocean) = if let Some(tile_union) = tile_union {
        let mut ocean = ocean;
        let mut polygons = Vec::new();
        let union_polygons = multipolygon_to_polygons(tile_union);
        for polygon in union_polygons.into_iter().watch(progress,"Making coastlines curvy.","Coastlines are curvy.") {
            for new_polygon in bezierify_polygon(&polygon,bezier_scale)? {
                if let Some(difference) = ocean.difference(&new_polygon) {
                    ocean = difference; 
                } // TODO: Or what?
                polygons.push(new_polygon);
            }
        }
        (Some(polygons),ocean)
    } else {
        (None,ocean)
    };


    let ocean_polygons = multipolygon_to_polygons(ocean);

    let mut coastline_layer = target.create_coastline_layer(overwrite_coastline)?;
    if let Some(land_polygons) = land_polygons {
        for polygon in land_polygons.into_iter().watch(progress, "Writing land masses.", "Land masses written.") {
            coastline_layer.add_land_mass(polygon)?;
        }
    }

    let mut ocean_layer = target.create_ocean_layer(overwrite_ocean)?;
    for polygon in ocean_polygons.into_iter().watch(progress, "Writing oceans.", "Oceans written.") {
        ocean_layer.add_ocean(polygon)?;
    }

    Ok(())
}

pub(crate) trait Theme: Sized {

    type ThemeSchema: Schema;
    type TileForTheme: Entity<TileSchema> + TileWithGeometry + TileWithShoreDistance + TileWithNeighbors + Clone + for<'feature> TryFrom<TileFeature<'feature>,Error=CommandError>;
    type Feature<'feature>: TypedFeature<'feature,Self::ThemeSchema>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError>;

    fn get_theme_id(&self, tile: &Self::TileForTheme) -> Result<std::option::Option<&u64>, CommandError>;

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<crate::world_map::MapLayer<'layer,'feature, Self::ThemeSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature;

}

pub(crate) struct CultureTheme {
    culture_id_map: EntityLookup<CultureSchema, CultureForDissolve>
}

impl Theme for CultureTheme {

    type ThemeSchema = CultureSchema;
    type TileForTheme = TileForCultureDissolve;
    type Feature<'feature> = crate::world_map::CultureFeature<'feature>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError> {
        let culture_id_map = target.edit_cultures_layer()?.read_features().to_named_entities_index::<_,CultureForDissolve>(progress)?;
        Ok(Self {
            culture_id_map
        })
    }

    fn get_theme_id(&self, tile: &TileForCultureDissolve) -> Result<std::option::Option<&u64>, CommandError> {
        if let Some(culture) = &tile.culture {
            // TODO: Is TryGet implemented for this?
            Ok::<_,CommandError>(Some(&self.culture_id_map.try_get(culture)?.fid))
        } else {
            Ok(None)
        }
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<crate::world_map::MapLayer<'layer,'feature, CultureSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_cultures_layer()        
    }


    
}

pub(crate) fn dissolve_tiles_by_theme<'target,Progress: ProgressObserver, ThemeType: Theme>(target: &'target mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> 
{
    // TODO: I'm testing this first with cultures, then I have to abstract it out so I can pass a field in later. Possibly,
    // it's just a matter of passing some sort of entity type that implements a trait?

    // TODO: In order to get this to work, I think the simplest answer is to get rid of the requirement of TryFrom for entities, so
    // I don't need the original feature anymore. TryFrom will still be implemented, however, and the various functions which require them
    // will now have a TryFrom initial requirement instead.

    let mut new_polygon_map: HashMap<u64, _> = HashMap::new();

    let theme = ThemeType::new(target,progress)?;


    let mut tile_map = EntityIndex::new();
    let mut tiles = Vec::new();

    {
        let mut tile_layer = target.edit_tile_layer()?;
        for feature in tile_layer.read_features().watch(progress, "Indexing tiles.", "Tiles indexed.") {
            let id = feature.fid()?;
            let entity = ThemeType::TileForTheme::try_from(feature)?;
            tile_map.insert(id, entity.clone());
            tiles.push(entity);
        }
    
    }

    for tile in tiles.into_iter().watch(progress, "Gathering tiles.", "Tiles gathered.") {
        let mapping: Option<(u64,_)> = if let Some(id) = theme.get_theme_id(&tile)? {
            Some((*id,tile.geometry().clone()))
        } else if tile.shore_distance() == &-1 {
            let mut usable_neighbors = HashMap::new();
            for (neighbor_id,_) in tile.neighbors() {
                let neighbor = tile_map.try_get(&neighbor_id)?;
                if let Some(id) = theme.get_theme_id(neighbor)? {
                    match usable_neighbors.get_mut(id) {
                        None => {
                            usable_neighbors.insert(id, 1);
                        },
                        Some(entry) => {
                            *entry += 1
                        }
                    }
                }
            }

            if usable_neighbors.len() > 0 {
                let chosen_value = usable_neighbors.iter().max_by_key(|n| n.1).unwrap().0;
                Some((**chosen_value,tile.geometry().clone()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((key,geometry)) = mapping {
            match new_polygon_map.get_mut(&key) {
                None => {
                    new_polygon_map.insert(key, vec![geometry]);
                },
                Some(entry) => entry.push(geometry),
            }
        }
    }

    let polygon_layer = ThemeType::edit_theme_layer(target)?;

    for (key,geometries) in new_polygon_map.into_iter().watch(progress, "Dissolving tiles.", "Tiles dissolved.") {
        let mut geometries = geometries.into_iter();
        if let Some(first) = geometries.next() {
            let mut remaining = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
            for geometry in geometries {
                remaining.add_geometry(geometry)?;
            }
            let united = first.union(&remaining).unwrap(); // TODO: Or what?
            let united = force_multipolygon(united)?;
            let mut feature: ThemeType::Feature<'_> = polygon_layer.try_feature_by_id(&key)?;
            feature.set_geometry(united)?;
            polygon_layer.update_feature(feature)?;
        }
    }
 
    Ok(())
}

