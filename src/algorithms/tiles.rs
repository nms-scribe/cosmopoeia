use std::collections::HashMap;

use crate::world_map::TileEntitySiteGeo;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::errors::CommandError;
use crate::world_map::NewTileEntity;
use crate::world_map::TypedFeature;
use crate::world_map::TileEntityWithNeighborsElevation;
use crate::world_map::TilesLayer;
use crate::utils::Point;

pub(crate) fn load_tile_layer<Generator: Iterator<Item=Result<NewTileEntity,CommandError>>, Progress: ProgressObserver>(target: &mut WorldMapTransaction, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

    let mut target = target.create_tile_layer(overwrite_layer)?;

    // boundary points    

    progress.start(|| ("Writing tiles.",generator.size_hint().1));

    for (i,tile) in generator.enumerate() {
        target.add_tile(tile?)?;
        progress.update(|| i);
    }

    progress.finish(|| "Tiles written.");

    Ok(())

}

pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;

    let features = layer.read_features().to_entities_vec::<_,TileEntitySiteGeo>(progress)?;

    progress.start_known_endpoint(|| ("Calculating neighbors.",features.len()));

    // # Loop through all features and find features that touch each feature
    // for f in feature_dict.values():
    for (i,feature) in features.iter().enumerate() {

        let working_fid = feature.fid;
        let working_geometry = &feature.geometry;

        let envelope = working_geometry.envelope();
        layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);


        let mut neighbors = Vec::new();

        for intersecting_feature in layer.read_features() {

            if let Some(intersecting_fid) = intersecting_feature.fid() {
                if (working_fid != intersecting_fid) && (!intersecting_feature.geometry().unwrap().disjoint(&working_geometry)) {

                    let neighbor_site_x = intersecting_feature.site_x()?;
                    let neighbor_site_y = intersecting_feature.site_y()?;
                    let neighbor_angle = if let (site_x,site_y,Some(neighbor_site_x),Some(neighbor_site_y)) = (feature.site_x,feature.site_y,neighbor_site_x,neighbor_site_y) {
                        // needs to be clockwise, from the north, with a value from 0..360
                        // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                        let counter_clockwise_from_east = ((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round();
                        // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                        let clockwise_from_north = 450.0 - counter_clockwise_from_east; 
                        // And then, to get the values in the range from 0..360, mod it.
                        let clamped = clockwise_from_north % 360.0;
                        clamped
                    } else {
                        // in the off chance that we actually are missing data, this marks an appropriate angle.
                        -360.0 
                    };
            
                    neighbors.push((intersecting_fid,neighbor_angle.floor() as i32)) 
                }

            }

        }
    
        layer.clear_spatial_filter();

        if let Some(mut working_feature) = layer.feature_by_id(&working_fid) {
            working_feature.set_neighbors(&neighbors)?;

            layer.update_feature(working_feature)?;

        }


        progress.update(|| i);

    }

    progress.finish(|| "Neighbors calculated.");

    Ok(())

}


pub(crate) fn find_lowest_neighbors<Data: TileEntityWithNeighborsElevation>(entity: &Data, tile_map: &HashMap<u64,Data>) -> (Vec<u64>, Option<f64>) {
    let mut lowest = Vec::new();
    let mut lowest_elevation = None;

    // find the lowest neighbors
    for (neighbor_fid,_) in entity.neighbors() {
        if let Some(neighbor) = tile_map.get(&neighbor_fid) {
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

    }
    (lowest,lowest_elevation.copied())

}

pub(crate) fn find_tile_site_point(previous_tile: Option<u64>, tiles: &TilesLayer<'_>) -> Result<Option<Point>, CommandError> {
    Ok(if let Some(x) = previous_tile {
        if let Some(x) = tiles.feature_by_id(&x) {
            Some(x.site_point()?)
        } else {
            None
        }
    } else {
        None
    })
}

