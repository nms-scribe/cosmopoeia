use std::collections::HashMap;
use std::collections::HashSet;

use rand::Rng;
use angular_units::Deg;
use ordered_float::OrderedFloat;

use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::errors::CommandError;
use crate::world_map::tile_layer::NewTileSite;
use crate::world_map::tile_layer::TileForCalcNeighbors;
use crate::typed_map::features::TypedFeature;
use crate::utils::coordinates::Coordinates;
use crate::world_map::tile_layer::TileForCultureDissolve;
use crate::world_map::culture_layer::CultureForDissolve;
use crate::world_map::tile_layer::TileWithGeometry;
use crate::world_map::tile_layer::TileWithShoreDistance;
use crate::world_map::tile_layer::TileWithNeighbors;
use crate::world_map::tile_layer::TileFeature;
use crate::typed_map::entities::Entity;
use crate::typed_map::schema::Schema;
use crate::world_map::culture_layer::CultureSchema;
use crate::world_map::tile_layer::TileSchema;
use crate::typed_map::entities::EntityIndex;
use crate::typed_map::entities::EntityLookup;
use crate::world_map::biome_layer::BiomeFeature;
use crate::world_map::culture_layer::CultureFeature;
use crate::world_map::biome_layer::BiomeSchema;
use crate::world_map::biome_layer::BiomeForDissolve;
use crate::world_map::tile_layer::TileForBiomeDissolve;
use crate::world_map::nation_layers::NationSchema;
use crate::world_map::tile_layer::TileForNationDissolve;
use crate::world_map::nation_layers::NationFeature;
use crate::world_map::nation_layers::SubnationSchema;
use crate::world_map::tile_layer::TileForSubnationDissolve;
use crate::world_map::nation_layers::SubnationFeature;
use crate::typed_map::features::TypedFeatureIterator;
use crate::typed_map::layers::MapLayer;
use crate::world_map::property_layer::ElevationLimits;
use crate::utils::extent::Extent;
use crate::algorithms::voronoi::VoronoiGenerator;
use crate::algorithms::triangles::DelaunayGenerator;
use crate::algorithms::random_points::PointGenerator;
use crate::utils::coordinates::ToGeometryCollection;
use crate::typed_map::features::NamedFeature;
use crate::commands::OverwriteTilesArg;
use crate::commands::OverwriteCoastlineArg;
use crate::commands::OverwriteOceanArg;
use crate::commands::BezierScaleArg;
use crate::geometry::MultiPolygon;
use crate::geometry::VariantArealGeometry;
use crate::world_map::fields::NeighborAndDirection;
use crate::world_map::fields::Neighbor;
use crate::utils::edge::Edge;
use crate::typed_map::fields::IdRef;
use crate::utils::world_shape::WorldShape;


pub(crate) fn generate_random_tiles<Random: Rng, Progress: ProgressObserver>(random: &mut Random, extent: Extent, shape: WorldShape, tile_count: usize, progress: &mut Progress) -> Result<VoronoiGenerator<DelaunayGenerator>, CommandError> {

    progress.announce("Generate random tiles");

    // yes, the random variable is a mutable reference, and PointGenerator doesn't take a reference as it's generic, 
    // but the reference implements the random number generator stuff so it works.
    // I assume if I was leaking the PointGenerator out of the function that I would get an error.
    let mut points = PointGenerator::new(random, extent.clone(), shape.clone(), tile_count);
    let mut triangles = DelaunayGenerator::new(points.to_geometry_collection(progress)?);
    
    triangles.start(progress)?;
    let mut voronois = VoronoiGenerator::new(triangles,extent,shape)?;
    
    voronois.start(progress)?;
    
    Ok(voronois)
}



pub(crate) fn load_tile_layer<Generator: Iterator<Item=Result<NewTileSite,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: &OverwriteTilesArg, generator: Generator, limits: &ElevationLimits, world_shape: &WorldShape, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.create_tile_layer(overwrite_layer)?;

    // NOTE: The delaunay process seems to process the points in a random order. However, I need to always insert the tiles from the same
    // generated points in the same order. If I could somehow "map" the sites with their original points, I could apply an incrementing
    // id to the points while I'm generating and have it here to sort by. Until that time, I'm sorting by x,y. This sort is a little
    // bit heavy, so there might be a better way.
    let collected_tiles: Result<Vec<NewTileSite>,CommandError> = generator.watch(progress,"Collecting tiles", "Tiles collected.").collect();
    let mut collected_tiles = collected_tiles?;
    collected_tiles.sort_by_cached_key(|tile| tile.site.to_ordered_tuple());

    for tile in collected_tiles.into_iter().watch(progress,"Writing tiles.","Tiles written.") {
        tiles.add_tile(tile)?;
    }

    let mut props = target.create_properties_layer()?;

    _ = props.set_elevation_limits(limits)?;

    _ = props.set_world_shape(world_shape)?;

    Ok(())

}

pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    // NOTE: At one point I tried an algorithm which iterated through each polygon, set a spatial index for its bounds, then
    // found all non-disjoint polygons in that index to mark them as a neighbor. That was slow. This is hugely faster. The old way took about 
    // 5 seconds for 10,000 tiles, and this one was almost instantaneous for that number. I blame my algorithm for curvifying
    // polygons for coming up with this idea.

    // NOTE 2: This could easily be part of the Tile generating code, wherein I already have the point indexes. However, I do
    // not have fid's yet for building maps, so that parts a little tricky.

    enum Side {
        East,
        West
    }

    let world_shape = target.edit_properties_layer()?.get_world_shape()?;

    let mut layer = target.edit_tile_layer()?;

    let layer_extent = layer.get_extent()?;

    let mut point_tile_index = HashMap::new();

    let mut east_west_list = if layer_extent.wraps_latitudinally() {
        Some(Vec::new())
    } else {
        None
    };
    
    let reaches_north_pole = layer_extent.reaches_north_pole();
    let reaches_south_pole = layer_extent.reaches_south_pole();

    // Find all tiles which share the same vertex. 
    // And if we're wrapping, keep a list of those that line up along east and west lines
    let mut tile_map = layer.read_features().into_entities_index_for_each::<_,TileForCalcNeighbors,_>(|fid,tile| {
        let ring = tile.geometry.get_ring(0)?;
        let usable_points_len = ring.len() - 1;
        // rings duplicate points at either end, so I need to skip the last.
        for point in ring.into_iter().take(usable_points_len) { 
            if let Some(list) = east_west_list.as_mut() {
                if (point.0 - layer_extent.east()).abs() < f64::EPSILON {
                    list.push((fid.clone(),point.1,Side::East))
                } else if (point.0 - layer_extent.west).abs() < f64::EPSILON {
                    list.push((fid.clone(),point.1,Side::West))
                }
            }
            let point: Coordinates = point.try_into()?;
            match point_tile_index.get_mut(&point) {
                None => {
                    _ = point_tile_index.insert(point, HashSet::from([fid.clone()]));
                },
                Some(set) => {
                    _ = set.insert(fid.clone());
                }
            }
        }

        Ok(())

    }, progress)?;

    // map all of the tiles that share each vertex as their own neighbors.
    for (_,tiles) in point_tile_index.into_iter().watch(progress, "Matching vertices.", "Vertices matched.") {

        for tile in &tiles {
            
            // I can't calculate the angle yet, because I'm still deduplicating any intersections. I'll do that in the next loop.
            let neighbors = tiles.iter().filter(|neighbor| *neighbor != tile).cloned();

            tile_map.try_get_mut(tile)?.neighbor_set.extend(neighbors)

        }

    }

    let wraps_latitudinally = if let Some(mut east_west_list) = east_west_list {

        // I have a list of tile ids, latitude of their vertices, and their side
        // sort that list by latitude.
        east_west_list.sort_by_cached_key(|t| OrderedFloat(t.1));

        // now, with that list sorted by latitude, when I go through it, if I see an id the first time, that turns the tile "on", and
        // when I see that the second time, that turns the tile "off". So, we keep track of "active" tiles on each side.
        // to track them, I'm keeping a map of tile and their neighbors
        let mut active_east_tiles: HashMap<IdRef, HashSet<IdRef>> = HashMap::new();
        let mut active_west_tiles: HashMap<IdRef, HashSet<IdRef>> = HashMap::new();

        // iterate through the list
        for (id,_,side) in east_west_list.into_iter().watch(progress, "Matching antimeridian neighbors.", "Antimeridian neighbors matched.") {
            let (hither_tiles,yonder_tiles) = match side {
                Side::East => (&mut active_east_tiles,&mut active_west_tiles),
                Side::West => (&mut active_west_tiles,&mut active_east_tiles),
            };

            match hither_tiles.remove(&id) {
                None => {
                    // this tile is being turned "on". So add it as a neighbor to any currently active tiles *on the other side*,
                    // plus gather those active tiles as a set to be inserted for this tile.
                    let mut yonder_neighbors = HashSet::new();
                    for (yonder_id,yonder_set) in yonder_tiles {
                        _ = yonder_neighbors.insert(yonder_id.clone());
                        _ = yonder_set.insert(id.clone());
                    }
                    // add that set to hither_tiles.
                    _ = hither_tiles.insert(id, yonder_neighbors);
                },
                Some(neighbors) => {
                    // key existed in the map, so turn it "off", no more neighbors will be assigned to it,
                    // so add those neighbors to the cross_neight_set
                    tile_map.try_get_mut(&id)?.cross_neighbor_set.extend(neighbors.into_iter());
                },
            }

/*             if hither_tiles.contains_key(&id) {
                // then it's time to turn it "off", no more neighbors should be assigned to it
                let neighbors = hither_tiles.remove(&id).expect("Why wouldn't this exist if I put it in when the key was inserted?");
                // and add those neighbors to the cross neighbors map
                tile_map.try_get_mut(&id)?.cross_neighbor_set.extend(neighbors.into_iter());
            } else {
                // this tile is being turned "on". So add it as a neighbor to any currently active tiles *on the other side*,
                // plus gather those active tiles as a set to be inserted for this tile.
                let mut yonder_neighbors = HashSet::new();
                for (yonder_id,yonder_set) in yonder_tiles.iter_mut() {
                    _ = yonder_neighbors.insert(*yonder_id);
                    _ = yonder_set.insert(id);
                }
                // add that set to hither_tiles.
                _ = hither_tiles.insert(id, yonder_neighbors);
            }
 */        }

        if (!active_east_tiles.is_empty()) || (!active_west_tiles.is_empty()) {
            println!("east: {active_east_tiles:?}");
            println!("west: {active_west_tiles:?}");
            panic!("Why would there be any tiles left active? A tile should always has exactly two nodes along a side.")
        }

        true


    } else {
        false
    };


    for (fid,tile) in tile_map.iter().watch(progress, "Writing neighbors.", "Neighbors written.") {

        let mut neighbors = Vec::new();
        for neighbor_id in &tile.neighbor_set {
            let neighbor_angle = calculate_neighbor_angle(tile, neighbor_id, &tile_map, &world_shape, false)?;

            neighbors.push(NeighborAndDirection(Neighbor::Tile(neighbor_id.clone()),neighbor_angle))

        }

        // handle the cross neighbors, if they were calculated, but this should only happen if they were on the edge
        if let Some(edge) = &tile.edge {
            for neighbor_id in &tile.cross_neighbor_set {
                let neighbor_angle = calculate_neighbor_angle(tile, neighbor_id, &tile_map, &world_shape, true)?;
    
                neighbors.push(NeighborAndDirection(Neighbor::CrossMap(neighbor_id.clone(),edge.clone()),neighbor_angle))
    
            }
    
        }

        // recalculate edge for the purposes of creating OffMap tiles
        // wrapping edges (east and west) should not have OffMap tiles because they already have CrossMap tiles.
        // polar edges (north and south) should not have OffMap tiles in order to keep features from extending to the poles, which can make things look weird.
        #[allow(clippy::match_same_arms)] // I have them separated for better understanding of what's going on
        let edge: Option<Edge> = match (wraps_latitudinally,reaches_north_pole,reaches_south_pole,&tile.edge) {
            (_, _, _, None) => None, // there was no edge in the first place

            // wraps_latitudinally, reaches_north_pole and reaches_south_pole
            (true, true, true, Some(_)) => None, // all items being true means there are no OffMap tiles

            // wraps_latitudinally and reaches_north_pole, so only have OffMap tiles for the south
            (true, true, false, Some(Edge::North | Edge::East | Edge::West | Edge::Northwest | Edge::Northeast)) => None,
            (true, true, false, Some(Edge::South | Edge::Southeast | Edge::Southwest)) => Some(Edge::South),

            // wraps_latitudinally and reaches_south_pole, so only OffMap tiles for the north
            (true, false, true, Some(Edge::South | Edge::East | Edge::West | Edge::Southeast | Edge::Southwest)) => None,
            (true, false, true, Some(Edge::North | Edge::Northwest | Edge::Northeast)) => Some(Edge::North),

            // wraps_latitudinally and that's it, so OffMap tiles for north and south only
            (true, false, false, Some(Edge::East | Edge::West)) => None,
            (true, false, false, Some(Edge::North | Edge::Northwest | Edge::Northeast)) => Some(Edge::North),
            (true, false, false, Some(Edge::South | Edge::Southeast | Edge::Southwest)) => Some(Edge::South),

            // reaches_north_pole and reaches_south_pole, so OffMap tiles for east and west only
            (false, true, true, Some(Edge::North | Edge::South)) => None,
            (false, true, true, Some(Edge::Northeast | Edge::Southeast | Edge::East)) => Some(Edge::East),
            (false, true, true, Some(Edge::Northwest | Edge::Southwest | Edge::West)) => Some(Edge::West),

            // reaches_north_pole, so OffMap tiles for east, west, south and south corners
            (false, true, false, Some(Edge::North)) => None,
            (false, true, false, Some(Edge::Northeast | Edge::East)) => Some(Edge::East),
            (false, true, false, Some(Edge::Northwest | Edge::West)) => Some(Edge::West),
            (false, true, false, Some(edge @ (Edge::South | Edge::Southeast | Edge::Southwest))) => Some(edge.clone()),

            // reaches_south_pole, so OffMap tiles for east, west and north corners
            (false, false, true, Some(Edge::South)) => None,
            (false, false, true, Some(Edge::Southeast | Edge::East)) => Some(Edge::East),
            (false, false, true, Some(Edge::Southwest | Edge::West)) => Some(Edge::West),
            (false, false, true, Some(edge @ (Edge::North | Edge::Northwest | Edge::Northeast))) => Some(edge.clone()),

            // no wrapping or poles at all, so edges are all as originally calculated
            (false, false, false, Some(edge)) => Some(edge.clone()),
        };


        // push the "edge" neighbors
        if let Some(edge) = edge {

            neighbors.push(NeighborAndDirection(Neighbor::OffMap(edge.clone()), edge.direction()))
            
        }

        // sort the neighbors by tile_id, to help ensure random reproducibility
        neighbors.sort_by_cached_key(|n| n.0.clone());

        let mut feature = layer.try_feature_by_id(fid)?;
        feature.set_neighbors(&neighbors)?;
        layer.update_feature(feature)?;

    }
    
    Ok(())

}

fn calculate_neighbor_angle(tile: &TileForCalcNeighbors, neighbor_id: &IdRef, tile_map: &EntityIndex<TileSchema, TileForCalcNeighbors>, world_shape: &WorldShape, across_anti_meridian: bool) -> Result<Deg<f64>, CommandError> {
    let neighbor = tile_map.try_get(neighbor_id)?;
    let neighbor_angle = {
        let (site_x,site_y) = tile.site.to_tuple();
        let (neighbor_site_x,neighbor_site_y) = neighbor.site.to_tuple();

        let neighbor_site_x = if across_anti_meridian {
            Coordinates::longitude_across_antimeridian(neighbor_site_x,&site_x)
        } else {
            neighbor_site_x
        };

        world_shape.calculate_bearing(site_x,site_y,neighbor_site_x,neighbor_site_y)

    };
    Ok(neighbor_angle)
}


// GetElevation takes an option. If the value is None, an appropriate elevation to handle data that is off the map should be returned
pub(crate) fn find_lowest_tile<Data: Entity<TileSchema>, GetElevation: Fn(Option<(&Data,bool)>) -> f64, GetNeighbors: Fn(&Data) -> &Vec<NeighborAndDirection>>(entity: &Data, tile_map: &EntityIndex<TileSchema,Data>, elevation: GetElevation, neighbors: GetNeighbors) -> Result<(Vec<Neighbor>, Option<f64>),CommandError> {
    let mut lowest = Vec::new();
    let mut lowest_elevation = None;

    // find the lowest neighbors
    for NeighborAndDirection(neighbor_fid,_) in neighbors(entity) {
        let neighbor = match neighbor_fid {
            Neighbor::Tile(neighbor_fid) => Some((tile_map.try_get(neighbor_fid)?,false)),
            Neighbor::CrossMap(neighbor_fid,_) => Some((tile_map.try_get(neighbor_fid)?,true)),
            Neighbor::OffMap(_) => None,
        };
        let neighbor_elevation = elevation(neighbor);
        if let Some(lowest_elevation) = lowest_elevation.as_mut() {
            if neighbor_elevation < *lowest_elevation {
                *lowest_elevation = neighbor_elevation;
                lowest = vec![neighbor_fid.clone()];
            } else if (neighbor_elevation - *lowest_elevation).abs() < f64::EPSILON {
                lowest.push(neighbor_fid.clone())
            }
        } else {
            lowest_elevation = Some(neighbor_elevation);
            lowest.push(neighbor_fid.clone())
        }

    }
    Ok((lowest,lowest_elevation))

}

pub(crate) fn calculate_coastline<Progress: ProgressObserver>(target: &mut WorldMapTransaction, bezier_scale: &BezierScaleArg, overwrite_coastline: &OverwriteCoastlineArg, overwrite_ocean: &OverwriteOceanArg, progress: &mut Progress) -> Result<(),CommandError> {

    // FUTURE: There is an issue with coastlines extending over the edge of the borders after curving. I will have to deal with these someday.
    // FUTURE: After curving, towns which are along the coastline will sometimes now be in the ocean. I may need to deal with that as well, someday.

    let mut tile_layer = target.edit_tile_layer()?;
    let extent_polygon = tile_layer.get_extent()?.create_polygon()?;

    let mut iterator = tile_layer.read_features().filter_map(|f| {
        match f.grouping() {
            Ok(g) if !g.is_ocean() => Some(Ok(f)),
            Ok(_) => None,
            Err(err) => Some(Err(err)),
        }
    } ).watch(progress, "Gathering tiles.", "Tiles gathered.");

    let tile_union = if let Some(tile) = iterator.next() {
        let first_tile: MultiPolygon = tile?.geometry()?.try_into()?; 

        // it's much faster to union two geometries rather than union them one at a time.
        let next_tiles = MultiPolygon::from_polygon_results(iterator.map(|f| f?.geometry()))?;

        progress.start_unknown_endpoint(|| "Uniting tiles.");

        let tile_union = first_tile.union(&next_tiles)?;
        progress.finish(|| "Tiles united.");
        Some(tile_union)
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
        for polygon in tile_union.into_iter().watch(progress,"Making coastlines curvy.","Coastlines are curvy.") {
            for new_polygon in polygon?.bezierify(bezier_scale.bezier_scale)? {
                let new_polygon = new_polygon?;
                ocean = ocean.difference(&VariantArealGeometry::Polygon(new_polygon.clone()))?;

                // snip it into the edge of the extent_polygon
                let new_polygon = new_polygon.intersection(&extent_polygon)?;
        
                // the last one returns a variant (multi?)polygon, so extend it as an iterator of polygons instead.
                polygons.extend(new_polygon);
            }
        }
        (Some(polygons),ocean)
    } else {
        (None,ocean)
    };

    // snip the ocean polygon as well.
    let ocean = ocean.intersection(&extent_polygon.into())?;

    let mut coastline_layer = target.create_coastline_layer(overwrite_coastline)?;
    if let Some(land_polygons) = land_polygons {
        for polygon in land_polygons.into_iter().watch(progress, "Writing land masses.", "Land masses written.") {
            _ = coastline_layer.add_land_mass(polygon?)?;
        }
    }

    let mut ocean_layer = target.create_ocean_layer(overwrite_ocean)?;
    for polygon in ocean.into_iter().watch(progress, "Writing oceans.", "Oceans written.") {
        _ = ocean_layer.add_ocean(polygon?)?;
    }

    Ok(())
}

pub(crate) trait Theme: Sized {

    type ThemeSchema: Schema<Geometry = MultiPolygon>;
    type TileForTheme: Entity<TileSchema> + TileWithGeometry + TileWithShoreDistance + TileWithNeighbors + Clone + for<'feature> TryFrom<TileFeature<'feature>,Error=CommandError>;
    type Feature<'feature>: NamedFeature<'feature,Self::ThemeSchema>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError>;

    fn get_theme_id(&self, tile: &Self::TileForTheme) -> Result<Option<IdRef>, CommandError>;

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, Self::ThemeSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature;

    fn read_theme_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer,'feature, Self::ThemeSchema, Self::Feature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature;


}

pub(crate) struct CultureTheme {
    culture_id_map: EntityLookup<CultureSchema, CultureForDissolve>
}

impl Theme for CultureTheme {

    type ThemeSchema = CultureSchema;
    type TileForTheme = TileForCultureDissolve;
    type Feature<'feature> = CultureFeature<'feature>;

    fn new<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<Self,CommandError> {
        let culture_id_map = target.edit_cultures_layer()?.read_features().into_named_entities_index::<_,CultureForDissolve>(progress)?;
        Ok(Self {
            culture_id_map
        })
    }

    fn get_theme_id(&self, tile: &TileForCultureDissolve) -> Result<Option<IdRef>, CommandError> {
        if let Some(culture) = &tile.culture {
            Ok::<_,CommandError>(Some(self.culture_id_map.try_get(culture)?.fid.clone()))
        } else {
            Ok(None)
        }
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, CultureSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_cultures_layer()        
    }

    fn read_theme_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, CultureSchema, CultureFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
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
        let biome_id_map = target.edit_biomes_layer()?.read_features().into_named_entities_index::<_,BiomeForDissolve>(progress)?;
        Ok(Self {
            biome_id_map
        })
    }

    fn get_theme_id(&self, tile: &TileForBiomeDissolve) -> Result<Option<IdRef>, CommandError> {
        let biome = &tile.biome; 
        Ok::<_,CommandError>(Some(self.biome_id_map.try_get(biome)?.fid.clone()))
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, BiomeSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_biomes_layer()        
    }

    fn read_theme_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, BiomeSchema, BiomeFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }


    
}


pub(crate) struct NationTheme;

impl Theme for NationTheme {

    type ThemeSchema = NationSchema;
    type TileForTheme = TileForNationDissolve;
    type Feature<'feature> = NationFeature<'feature>;

    fn new<Progress: ProgressObserver>(_: &mut WorldMapTransaction, _: &mut Progress) -> Result<Self,CommandError> {
        Ok(Self)
    }

    fn get_theme_id(&self, tile: &TileForNationDissolve) -> Result<Option<IdRef>, CommandError> {
        Ok(tile.nation_id.clone())
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, NationSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_nations_layer()        
    }

    fn read_theme_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, NationSchema, NationFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }


    
}



pub(crate) struct SubnationTheme;

impl Theme for SubnationTheme {

    type ThemeSchema = SubnationSchema;
    type TileForTheme = TileForSubnationDissolve;
    type Feature<'feature> = SubnationFeature<'feature>;

    fn new<Progress: ProgressObserver>(_: &mut WorldMapTransaction, _: &mut Progress) -> Result<Self,CommandError> {
        Ok(Self)
    }

    fn get_theme_id(&self, tile: &TileForSubnationDissolve) -> Result<Option<IdRef>, CommandError> {
        Ok(tile.subnation_id.clone())
    }

    fn edit_theme_layer<'layer,'feature>(target: &'layer mut WorldMapTransaction) -> Result<MapLayer<'layer,'feature, SubnationSchema, Self::Feature<'feature>>, CommandError> where 'layer: 'feature {
        target.edit_subnations_layer()        
    }

    fn read_theme_features<'layer,'feature>(layer: &'layer mut MapLayer<'layer, 'feature, SubnationSchema, SubnationFeature<'feature>>) -> TypedFeatureIterator<'feature,Self::ThemeSchema,Self::Feature<'feature>> where 'layer: 'feature {
        layer.read_features()
    }


    
}


pub(crate) fn dissolve_tiles_by_theme<Progress: ProgressObserver, ThemeType: Theme>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> 
{

    let mut new_polygon_map: HashMap<IdRef, _> = HashMap::new();

    let theme = ThemeType::new(target,progress)?;

    let mut tiles = Vec::new();

    let tile_map = target.edit_tile_layer()?.read_features().into_entities_index_for_each::<_,ThemeType::TileForTheme,_>(|_,tile| {
        tiles.push(tile.clone());
        Ok(())
    },progress)?;

    for tile in tiles.into_iter().watch(progress, "Gathering tiles.", "Tiles gathered.") {
        let mapping: Option<(IdRef,MultiPolygon)> = if let Some(id) = theme.get_theme_id(&tile)? {
            Some((id,tile.geometry().clone().try_into()?))
        } else if tile.shore_distance() == &-1 {
            let mut usable_neighbors = HashMap::new();
            for NeighborAndDirection(neighbor_id,_) in tile.neighbors() {
                match neighbor_id {
                    Neighbor::Tile(neighbor_id) => {
                        let neighbor = tile_map.try_get(neighbor_id)?;
                        if let Some(id) = theme.get_theme_id(neighbor)? {
                            match usable_neighbors.get_mut(&id) {
                                None => {
                                    _ = usable_neighbors.insert(id, 1);
                                },
                                Some(entry) => {
                                    *entry += 1
                                }
                            }
                        }
                    }
                    Neighbor::CrossMap(_,_) | Neighbor::OffMap(_) => (), // don't dissolve across the latitude. They will still be mapped, but they should be in separate polygons. And if it's off the map, ignore it.
                } 
            }

            if usable_neighbors.is_empty() {
                None
            } else {
                let chosen_value = usable_neighbors.iter().max_by_key(|n| n.1).expect("Why would there be no max if we know the list isn't empty?").0;
                Some((chosen_value.clone(),tile.geometry().clone().try_into()?))
            }
        } else {
            None
        };

        if let Some((key,geometry)) = mapping {
            match new_polygon_map.get_mut(&key) {
                None => _ = new_polygon_map.insert(key, vec![geometry]),
                Some(entry) => entry.push(geometry),
            }
        }
    }

    let mut polygon_layer = ThemeType::edit_theme_layer(target)?;

    let mut empty_features = Vec::new();

    let mut changed_features = Vec::new();

    for feature in ThemeType::read_theme_features(&mut polygon_layer).watch(progress, "Dissolving tiles.", "Tiles dissolved.") {
        let fid = feature.fid()?;
        let geometry = if let Some(geometries) = new_polygon_map.remove(&fid) {
            // it should never be empty if it's in the map, but since I'm already allowing for empty geographies, might as well check.
            if geometries.is_empty() {
                empty_features.push((fid.clone(),feature.get_name()?));
                MultiPolygon::from_polygons([])?
            } else {
                let mut geometries = geometries.into_iter();
                let first = geometries.next().expect("Why would next fail if the len > 0?"); 
                let remaining = MultiPolygon::from_combined(geometries)?;

                first.union(&remaining)?.try_into()?
            }
        } else {
            empty_features.push((fid.clone(),feature.get_name()?));
            // An empty multipolygon appears to be the answer, at the very least I no longer get the null geometry pointer
            // errors in the later ones.
            MultiPolygon::from_polygons([])?
        };

        changed_features.push((fid,geometry));

    }

    // reinitialize to avoid a mutable/immutable borrowing conflict
    let edit_polygon_layer = ThemeType::edit_theme_layer(target)?;

    for (fid,geometry) in changed_features.into_iter().watch(progress, "Writing geometries.", "Geometries written.") {

        let mut feature: ThemeType::Feature<'_> = edit_polygon_layer.try_feature_by_id(&fid)?;
        feature.set_geometry(geometry)?;
        edit_polygon_layer.update_feature(feature)?;    

    }

    for (key,name) in empty_features {
        progress.warning(|| format!("Feature {} ({}) in {} is not used in any tiles.",name,key,ThemeType::ThemeSchema::LAYER_NAME))
    }
 
    Ok(())
}

