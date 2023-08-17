use crate::world_map::TileEntitySite;
use crate::errors::CommandError;
use crate::raster::RasterMap;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::world_map::Terrain;

pub(crate) fn sample_elevations_on_tiles<Progress: ProgressObserver>(target: &mut WorldMapTransaction, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;


    progress.start_unknown_endpoint(|| "Reading raster");

    let (min_elevation,max_elevation) = raster.compute_min_max(1,true)?;
    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;

    let positive_elevation_scale = 80.0/max_elevation;
    let negative_elevation_scale = 20.0/min_elevation.abs();

//    * find the max_elevation from the raster, if possible
//    * find the absolute value of the min_elevation from the raster, if possible
//    * if elevation >= 0
//      * elevation_scaled = (elevation*80)/max_elevation
//    * else
//      * elevation_scaled = 20 - (elevation.abs()*20)/min_elevation.abs()


    progress.finish(|| "Raster read.");

    let features = layer.read_features().to_entities_vec::<_,TileEntitySite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling elevations.",features.len()));

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(elevation) = band.get_value(x, y) {

            if let Some(mut feature) = layer.feature_by_id(&feature.fid) {

                let elevation_scaled = if elevation >= &0.0 {
                    20 + (elevation * positive_elevation_scale).floor() as i32
                } else {
                    20 - (elevation.abs() * negative_elevation_scale).floor() as i32
                };

                feature.set_elevation(*elevation)?;
                feature.set_elevation_scaled(elevation_scaled)?;

                layer.update_feature(feature)?;

            }

        }



        progress.update(|| i);




    }

    progress.finish(|| "Elevation sampled.");

    Ok(())
}

pub(crate) enum OceanSamplingMethod {
    Below(f64), // any elevation below the specified value is ocean
    AllData, // any elevation that is not nodata is ocean
    NoData, // any elevation that is nodata is ocean
    NoDataAndBelow(f64), // any elevation that is no data or below the specified value is ocean.
    // TODO: Another option: a list of points to act as seeds, along with an elevation, use a flood-fill to mark oceans that are connected to these and under that elevation.
}

pub(crate) fn sample_ocean_on_tiles<Progress: ProgressObserver>(target: &mut WorldMapTransaction, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;


    progress.start_unknown_endpoint(|| "Reading raster");

    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;
    let no_data_value = band.no_data_value();

    progress.finish(|| "Raster read.");

    let features = layer.read_features().to_entities_vec::<_,TileEntitySite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling oceans.",features.len()));

    let mut bad_ocean_tile_found = false;

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(mut feature) = layer.feature_by_id(&feature.fid) {

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                let is_no_data = match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => elevation.is_nan(),
                    Some(no_data_value) => elevation == no_data_value,
                    None => false,
                };

                match method {
                    OceanSamplingMethod::Below(_) if is_no_data => false,
                    OceanSamplingMethod::Below(below) => elevation < &below,
                    OceanSamplingMethod::AllData => !is_no_data,
                    OceanSamplingMethod::NoData => is_no_data,
                    OceanSamplingMethod::NoDataAndBelow(below) => is_no_data || (elevation < &below),
                }

            } else {

                match method {
                    OceanSamplingMethod::Below(_) => false,
                    OceanSamplingMethod::AllData => false,
                    OceanSamplingMethod::NoData => true,
                    OceanSamplingMethod::NoDataAndBelow(_) => true,
                }

            };

            if is_ocean && (feature.elevation()? > 0.0) {
                bad_ocean_tile_found = true;
            }

            let terrain = if is_ocean {
                Terrain::Ocean
            } else {
                // process of setting lake, island, continent, etc. will have to be redone.
                Terrain::Land
            };

            feature.set_terrain(&terrain)?;

            layer.update_feature(feature)?;

        }


        progress.update(|| i);




    }

    progress.finish(|| "Oceans sampled.");

    if bad_ocean_tile_found {
        println!("At least one ocean tile was found with an elevation above 0.")

    }

    Ok(())

}
