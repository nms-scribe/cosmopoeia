use std::collections::HashMap;
use std::collections::HashSet;

use gdal::vector::OGRwkbGeometryType;
use gdal::vector::Geometry;

use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::errors::CommandError;
use crate::world_map::NewTile;
use crate::world_map::TileForCalcNeighbors;
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
use crate::world_map::BiomeFeature;
use crate::world_map::CultureFeature;
use crate::world_map::BiomeSchema;
use crate::world_map::BiomeForDissolve;
use crate::world_map::TileForBiomeDissolve;
use crate::world_map::NationSchema;
use crate::world_map::TileForNationDissolve;
use crate::world_map::NationFeature;
use crate::world_map::SubnationSchema;
use crate::world_map::TileForSubnationDissolve;
use crate::world_map::SubnationFeature;
use crate::world_map::TypedFeatureIterator;
use crate::world_map::MapLayer;
use crate::world_map::ElevationLimits;

pub(crate) fn load_tile_layer<Generator: Iterator<Item=Result<NewTile,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, limits: ElevationLimits, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.create_tile_layer(overwrite_layer)?;

    // TODO: The delaunay process seems to process the points in a random order. However, I need to always insert the tiles from the same
    // generated points in the same order. If I could somehow "map" the sites with their original points, I could apply an incrementing
    // id to the points while I'm generating and have it here to sort by. Until that time, I'm sorting by x,y. This sort is a little
    // bit heavy, so there might be a better way.
    let collected_tiles: Result<Vec<NewTile>,CommandError> = generator.watch(progress,"Collecting tiles", "Tiles collected.").collect();
    let mut collected_tiles = collected_tiles?;
    collected_tiles.sort_by_cached_key(|tile| (ordered_float::OrderedFloat::from(tile.site_x),ordered_float::OrderedFloat::from(tile.site_y)));

    for tile in collected_tiles.into_iter().watch(progress,"Writing tiles.","Tiles written.") {
        tiles.add_tile(tile)?;
    }

    let mut props = target.create_properties_layer()?;

    props.set_elevation_limits(limits)?;

    Ok(())

}

pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    // TODO: I should also be able to do some additional things:
    // 1. Figure out if a tile is on the edge of the map, which may be important later.
    // 2. Find neighbors on the other side of the world, if a configuration property tells us to wrap east and west.
    // 3. Make all polar tiles neighbors, if a configuration property tells us that we should do that.

    // NOTE: At one point I tried an algorithm which iterated through each polygon, set a spatial index for its bounds, then
    // found all non-disjoint polygons in that index to mark them as a neighbor. This is hugely faster. The old way took about 
    // 5 seconds for 10,000 tiles, and this one was almost instantaneous for that number. I blame my algorithm for curvifying
    // polygons for coming up with this idea.

    let mut point_tile_index = HashMap::new();

    let mut layer = target.edit_tile_layer()?;

    // Find all tiles which share the same vertex.
    let mut tile_map = layer.read_features().to_entities_index_for_each::<_,TileForCalcNeighbors,_>(|fid,tile| {
        let ring = tile.geometry.get_geometry(0);
        for i in 0..ring.point_count() as i32 {
            let (x,y,_) = ring.get_point(i);
            let point = Point::from_f64(x, y)?;
            match point_tile_index.get_mut(&point) {
                None => {
                    point_tile_index.insert(point, HashSet::from([*fid]));
                },
                Some(set) => {
                    set.insert(*fid);
                }
            }
        }

        Ok(())

    }, progress)?;

    // map all of the tiles that share each vertex as their own neighbors.
    for (_,tiles) in point_tile_index.into_iter().watch(progress, "Matching vertices.", "Vertices matched.") {

        for tile in &tiles {
            
            // I can't calculate the angle yet, because I'm still deduplicating any intersections. I'll do that in the next loop.
            let neighbors = tiles.iter().filter(|neighbor| *neighbor != tile).map(|neighbor| *neighbor);

            tile_map.try_get_mut(tile)?.neighbor_set.extend(neighbors)

        }

    }

    for (fid,tile) in tile_map.iter().watch(progress, "Writing neighbors.", "Neighbors written.") {

        let mut neighbors = Vec::new();
        for neighbor_id in &tile.neighbor_set {
            let neighbor = tile_map.try_get(&neighbor_id)?;

            let neighbor_angle = {
                let (site_x,site_y) = tile.site.to_tuple();
                let (neighbor_site_x,neighbor_site_y) = neighbor.site.to_tuple();

                // needs to be clockwise, from the north, with a value from 0..360
                // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                let counter_clockwise_from_east = ((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round();
                // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                let clockwise_from_north = 450.0 - counter_clockwise_from_east; 
                // And then, to get the values in the range from 0..360, mod it.
                let clamped = clockwise_from_north % 360.0;
                clamped.floor() as i32
            };

            neighbors.push((*neighbor_id,neighbor_angle))

        }

        // sort the neighbors by tile_id, to help ensure random reproducibility
        neighbors.sort_by_key(|n| n.0);

        let mut feature = layer.try_feature_by_id(fid)?;
        feature.set_neighbors(&neighbors)?;
        layer.update_feature(feature)?;

    }
    
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

    fn get_theme_id(&self, tile: &Self::TileForTheme) -> Result<std::option::Option<u64>, CommandError>;

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, Self::ThemeSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature;

    fn read_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer,'feature, Self::ThemeSchema, Self::Feature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature;


}

pub(crate) struct CultureTheme {
    culture_id_map: EntityLookup<CultureSchema, CultureForDissolve>
}

impl Theme for CultureTheme {

    type ThemeSchema = CultureSchema;
    type TileForTheme = TileForCultureDissolve;
    type Feature<'feature> = CultureFeature<'feature>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError> {
        let culture_id_map = target.edit_cultures_layer()?.read_features().to_named_entities_index::<_,CultureForDissolve>(progress)?;
        Ok(Self {
            culture_id_map
        })
    }

    fn get_theme_id(&self, tile: &TileForCultureDissolve) -> Result<std::option::Option<u64>, CommandError> {
        if let Some(culture) = &tile.culture {
            Ok::<_,CommandError>(Some(self.culture_id_map.try_get(culture)?.fid))
        } else {
            Ok(None)
        }
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, CultureSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_cultures_layer()        
    }

    fn read_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, CultureSchema, CultureFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }
    
}


pub(crate) struct BiomeTheme {
    biome_id_map: EntityLookup<BiomeSchema, BiomeForDissolve>
}

impl Theme for BiomeTheme {

    type ThemeSchema = BiomeSchema;
    type TileForTheme = TileForBiomeDissolve;
    type Feature<'feature> = BiomeFeature<'feature>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError> {
        let biome_id_map = target.edit_biomes_layer()?.read_features().to_named_entities_index::<_,BiomeForDissolve>(progress)?;
        Ok(Self {
            biome_id_map
        })
    }

    fn get_theme_id(&self, tile: &TileForBiomeDissolve) -> Result<std::option::Option<u64>, CommandError> {
        let biome = &tile.biome; 
        Ok::<_,CommandError>(Some(self.biome_id_map.try_get(biome)?.fid))
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, BiomeSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_biomes_layer()        
    }

    fn read_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, BiomeSchema, BiomeFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }


    
}


pub(crate) struct NationTheme();

impl Theme for NationTheme {

    type ThemeSchema = NationSchema;
    type TileForTheme = TileForNationDissolve;
    type Feature<'feature> = NationFeature<'feature>;

    fn new<Progress: ProgressObserver>(_: &mut WorldMapTransaction, _: &mut Progress) -> Result<Self,CommandError> {
        Ok(Self())
    }

    fn get_theme_id(&self, tile: &TileForNationDissolve) -> Result<std::option::Option<u64>, CommandError> {
        Ok(tile.nation_id.map(|n| n as u64))
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, NationSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_nations_layer()        
    }

    fn read_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, NationSchema, NationFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }


    
}



pub(crate) struct SubnationTheme();

impl Theme for SubnationTheme {

    type ThemeSchema = SubnationSchema;
    type TileForTheme = TileForSubnationDissolve;
    type Feature<'feature> = SubnationFeature<'feature>;

    fn new<Progress: ProgressObserver>(_: &mut WorldMapTransaction, _: &mut Progress) -> Result<Self,CommandError> {
        Ok(Self())
    }

    fn get_theme_id(&self, tile: &TileForSubnationDissolve) -> Result<std::option::Option<u64>, CommandError> {
        Ok(tile.subnation_id.map(|n| n as u64))
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, SubnationSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_subnations_layer()        
    }

    fn read_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, SubnationSchema, SubnationFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
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

    let mut tiles = Vec::new();

    let tile_map = target.edit_tile_layer()?.read_features().to_entities_index_for_each::<_,ThemeType::TileForTheme,_>(|_,tile| {
        tiles.push(tile.clone());
        Ok(())
    },progress)?;

    for tile in tiles.into_iter().watch(progress, "Gathering tiles.", "Tiles gathered.") {
        let mapping: Option<(u64,_)> = if let Some(id) = theme.get_theme_id(&tile)? {
            Some((id,tile.geometry().clone()))
        } else if tile.shore_distance() == &-1 {
            let mut usable_neighbors = HashMap::new();
            for (neighbor_id,_) in tile.neighbors() {
                let neighbor = tile_map.try_get(&neighbor_id)?;
                if let Some(id) = theme.get_theme_id(neighbor)? {
                    match usable_neighbors.get_mut(&id) {
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
                Some((*chosen_value,tile.geometry().clone()))
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

