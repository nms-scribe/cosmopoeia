use crate::world_map::TileForSampling;
use crate::errors::CommandError;
use crate::raster::RasterMap;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::Grouping;

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

    let features = layer.read_features().to_entities_vec::<_,TileForSampling>(progress)?;

    for feature in features.iter().watch(progress,"Sampling elevations.","Elevations sampled.") {


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



    }

    Ok(())
}

pub(crate) enum OceanSamplingMethod {
    Below(f64), // any elevation below the specified value is ocean
    AllData // any elevation that is not nodata is ocean
}

pub(crate) fn sample_ocean_on_tiles<Progress: ProgressObserver>(target: &mut WorldMapTransaction, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

    let mut layer = target.edit_tile_layer()?;


    progress.start_unknown_endpoint(|| "Reading raster");

    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;
    let no_data_value = band.no_data_value();

    progress.finish(|| "Raster read.");

    let features = layer.read_features().to_entities_vec::<_,TileForSampling>(progress)?;

    let mut bad_ocean_tile_found = false;

    for feature in features.iter().watch(progress,"Sampling oceans.","Oceans sampled.") {


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
                    OceanSamplingMethod::AllData => !is_no_data
                }

            } else {

                false

            };

            if is_ocean && (feature.elevation()? > 0.0) {
                bad_ocean_tile_found = true;
            }

            // only apply if the data actually is ocean now, so one can use multiple ocean methods
            if is_ocean {
                feature.set_grouping(&Grouping::Ocean)?;

                layer.update_feature(feature)?;
    
            }

        }



    }

    if bad_ocean_tile_found {
        progress.warning(|| "At least one ocean tile was found with an elevation above 0.")
    }

    Ok(())

}
