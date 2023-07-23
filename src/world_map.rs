use std::path::Path;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::utils::LayerGeometryIterator;
use crate::utils::Point;
use crate::raster::RasterMap;

pub(crate) const POINTS_LAYER_NAME: &str = "points";
pub(crate) const TRIANGLES_LAYER_NAME: &str = "triangles";
pub(crate) const TILES_LAYER_NAME: &str = "tiles";

// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.


pub(crate) struct WorldLayer<'lifetime> {
    layer: Layer<'lifetime>
}

impl<'lifetime> WorldLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset, name: &str) -> Result<Self,CommandError> {
        let layer = dataset.layer_by_name(name)?;
        Ok(Self {
            layer
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, name: &str, geometry_type: OGRwkbGeometryType::Type, field_defs: Option<&[(&str, OGRFieldType::Type)]>, overwrite: bool) -> Result<Self,CommandError> {
        let layer = dataset.create_layer(LayerOptions {
            name,
            ty: geometry_type,
            options: if overwrite {
                Some(&["OVERWRITE=YES"])
            } else {
                None
            },
            ..Default::default()
        })?;
        if let Some(field_defs) = field_defs {
            layer.create_defn_fields(field_defs)?;
        }
        // NOTE: I'm specifying the field value as real for now. Eventually I might want to allow it to choose a type based on the raster type, but there
        // really isn't much choice, just three numeric types (int, int64, and real)

        Ok(Self {
            layer
        })
    }

    fn add(&mut self, geometry: Geometry, field_names: &[&str], field_values: &[FieldValue]) -> Result<(),CommandError> {

        self.layer.create_feature_fields(geometry,field_names,field_values)?;
        Ok(())
    }


    fn read_geometries(&mut self) -> LayerGeometryIterator {
        LayerGeometryIterator::new(&mut self.layer)

    }



}

pub(crate) struct PointsLayer<'lifetime> {
    points: WorldLayer<'lifetime>
}

impl<'lifetime> PointsLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let points = WorldLayer::open_from_dataset(dataset, POINTS_LAYER_NAME)?;
        Ok(Self {
            points
        })
    }

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let points = WorldLayer::create_from_dataset(dataset, POINTS_LAYER_NAME, OGRwkbGeometryType::wkbPoint, None, overwrite)?;

        Ok(Self {
            points
        })
    }

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.points.add(point,&[],&[])

    }

/* TODO: I'm going to need something like this eventually for sampling the tiles, so don't delete this yet.
    pub(crate) fn sample_point_from_raster(&mut self, x: f64, y: f64, transformer: &RasterCoordTransformer, buffer: &RasterBandBuffer<f64>) -> Result<(),CommandError> {
        let (lon,lat) = transformer.pixels_to_coords(x, y);
        let mut point = Geometry::empty(wkbPoint)?;
        point.add_point_2d((lon,lat));
        self.add_point(point, buffer.get_value(x, y))
    }
 */

    pub(crate) fn read_points(&mut self) -> LayerGeometryIterator {
        self.points.read_geometries()

    }


}



pub(crate) struct TrianglesLayer<'lifetime> {
    tiles: WorldLayer<'lifetime>
}

impl<'lifetime> TrianglesLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, TRIANGLES_LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, TRIANGLES_LAYER_NAME, OGRwkbGeometryType::wkbPolygon, None, overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.tiles.add(geo,&[],&[])

    }


    pub(crate) fn read_triangles(&mut self) -> LayerGeometryIterator {
        self.tiles.read_geometries()

    }



}

pub(crate) struct VoronoiTile {
    geometry: Geometry,
    site: Point
}

impl VoronoiTile {

    pub(crate) fn new(geometry: Geometry, site: Point) -> Self {
        Self {
            geometry,
            site
        }
    }
}

pub(crate) enum OceanSamplingMethod {
    Below(f64), // any elevation below the specified value is ocean
    AllData, // any elevation that is not nodata is ocean
    NoData, // any elevation that is nodata is ocean
    NoDataAndBelow(f64), // any elevation that is no data or below the specified value is ocean.
}

pub(crate) struct TilesLayer<'lifetime> {
    tiles: WorldLayer<'lifetime>
}

impl<'lifetime> TilesLayer<'lifetime> {

    const FIELD_SITE_X: &str = "site_x";
    const FIELD_SITE_Y: &str = "site_y";
    const FIELD_NEIGHBOR_TILES: &str = "neighbor_tiles";
    const FIELD_ELEVATION: &str = "elevation";
    const FIELD_OCEAN: &str = "is_ocean";

    const FIELD_DEFS: [(&str,OGRFieldType::Type); 5] = [
        (Self::FIELD_SITE_X,OGRFieldType::OFTReal),
        (Self::FIELD_SITE_Y,OGRFieldType::OFTReal),
        (Self::FIELD_NEIGHBOR_TILES,OGRFieldType::OFTString),
        (Self::FIELD_ELEVATION,OGRFieldType::OFTReal),
        (Self::FIELD_OCEAN,OGRFieldType::OFTInteger)
    ];

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, TILES_LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, TILES_LAYER_NAME, OGRwkbGeometryType::wkbPolygon, Some(&Self::FIELD_DEFS), overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_tile(&mut self, tile: VoronoiTile) -> Result<(),CommandError> {

        let (x,y) = tile.site.to_tuple();
        self.tiles.add(tile.geometry,&[
                Self::FIELD_SITE_X,
                Self::FIELD_SITE_Y,
            ],&[
                FieldValue::RealValue(x),
                FieldValue::RealValue(y),
            ])?;
        Ok(())

    }


    #[allow(dead_code)] pub(crate) fn read_tiles(&mut self) -> LayerGeometryIterator {
        self.tiles.read_geometries()

    }

    pub(crate) fn calculate_neighbors<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {

        let layer = &mut self.tiles.layer;

        progress.start_known_endpoint(|| ("Calculating neighbors.",layer.feature_count() as usize));

        let features: Result<Vec<(Option<u64>,Option<Geometry>,Option<f64>,Option<f64>)>,CommandError> = layer.features().map(|feature| Ok((
            feature.fid(),
            feature.geometry().cloned(),
            feature.field_as_double_by_name(Self::FIELD_SITE_X)?,
            feature.field_as_double_by_name(Self::FIELD_SITE_Y)?,
        ))).collect();
        let features = features?;

        // # Loop through all features and find features that touch each feature
        // for f in feature_dict.values():
        for (i,(working_fid,working_geometry,site_x,site_y)) in features.iter().enumerate() {

            if let Some(working_fid) = working_fid {
                if let Some(working_geometry) = working_geometry {

                    let envelope = working_geometry.envelope();
                    layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);
        
        
                    let mut neighbors = Vec::new();
        
                    for intersecting_feature in layer.features() {
        
                        if let Some(intersecting_fid) = intersecting_feature.fid() {
                            if (working_fid != &intersecting_fid) && (!intersecting_feature.geometry().unwrap().disjoint(&working_geometry)) {

                                let neighbor_site_x = intersecting_feature.field_as_double_by_name(Self::FIELD_SITE_X)?;
                                let neighbor_site_y = intersecting_feature.field_as_double_by_name(Self::FIELD_SITE_Y)?;
                                let neighbor_angle = if let (Some(site_x),Some(site_y),Some(neighbor_site_x),Some(neighbor_site_y)) = (site_x,site_y,neighbor_site_x,neighbor_site_y) {
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
                            
                                neighbors.push(format!("{}:{}",intersecting_fid,neighbor_angle)) 
                            }
    
                        }
        
                    }
                    
                    layer.clear_spatial_filter();

                    if let Some(working_feature) = layer.feature(*working_fid) {
                        working_feature.set_field_string(Self::FIELD_NEIGHBOR_TILES, &neighbors.join(","))?;
        
                        layer.set_feature(working_feature)?;
    
                    }
        
    
                }
            }

            progress.update(|| i);

        }

        progress.finish(|| "Neighbors calculated.");

        Ok(())
    }


    pub(crate) fn sample_elevations<Progress: ProgressObserver>(&mut self, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

        let layer = &mut self.tiles.layer;

        progress.start_unknown_endpoint(|| "Reading raster");

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;

        progress.finish(|| "Raster read.");

        progress.start_known_endpoint(|| ("Reading tiles",layer.feature_count() as usize));

        let mut features = Vec::new();

        for (i,feature) in layer.features().enumerate() {
            features.push((i,
                           feature.fid(),
                           feature.field_as_double_by_name(Self::FIELD_SITE_X)?,
                           feature.field_as_double_by_name(Self::FIELD_SITE_Y)?
            ))

        }

        progress.finish(|| "Tiles read.");

        progress.start_known_endpoint(|| ("Sampling elevations.",layer.feature_count() as usize));

        for (i,fid,site_lon,site_lat) in features {


            if let (Some(fid),Some(site_lon),Some(site_lat)) = (fid,site_lon,site_lat) {

                let (x,y) = bounds.coords_to_pixels(site_lon, site_lat);

                if let Some(elevation) = band.get_value(x, y) {

                    if let Some(feature) = layer.feature(fid) {
                        feature.set_field_double(Self::FIELD_ELEVATION, *elevation)?;

                        layer.set_feature(feature)?;
        
                    }

                }

    
            }

            progress.update(|| i);




        }

        progress.finish(|| "Elevation sampled.");

        Ok(())
    }

    pub(crate) fn sample_ocean<Progress: ProgressObserver>(&mut self, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

        let layer = &mut self.tiles.layer;

        progress.start_unknown_endpoint(|| "Reading raster");

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;
        let no_data_value = band.no_data_value();

        progress.finish(|| "Raster read.");

        progress.start_known_endpoint(|| ("Reading tiles",layer.feature_count() as usize));

        let mut features = Vec::new();

        for (i,feature) in layer.features().enumerate() {
            features.push((i,
                           feature.fid(),
                           feature.field_as_double_by_name(Self::FIELD_SITE_X)?,
                           feature.field_as_double_by_name(Self::FIELD_SITE_Y)?
            ))

        }

        progress.finish(|| "Tiles read.");

        progress.start_known_endpoint(|| ("Sampling oceans.",layer.feature_count() as usize));

        for (i,fid,site_lon,site_lat) in features {


            if let (Some(fid),Some(site_lon),Some(site_lat)) = (fid,site_lon,site_lat) {

                let (x,y) = bounds.coords_to_pixels(site_lon, site_lat);

                if let Some(feature) = layer.feature(fid) {

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

                    let is_ocean = if is_ocean { 1 } else { 0 };

                    feature.set_field_integer(Self::FIELD_OCEAN, is_ocean)?;

                    layer.set_feature(feature)?;
    
                }


    
            }

            progress.update(|| i);




        }

        progress.finish(|| "Oceans sampled.");

        Ok(())
    }


}

pub(crate) struct WorldMap {
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &str = "GPKG";

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    #[allow(dead_code)] pub(crate) fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        let dataset = Dataset::open(path)?;
        Ok(Self::new(dataset))
    }


    pub(crate) fn edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open_ex(path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?))
    }

    pub(crate) fn create_or_edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(path)?;
            Ok(Self::new(dataset))
        }

    }

    pub(crate) fn with_transaction<Callback: FnOnce(&mut WorldMapTransaction) -> Result<(),CommandError>>(&mut self, callback: Callback) -> Result<(),CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        callback(&mut transaction)?;
        transaction.commit()?;
        Ok(())

    }

    pub(crate) fn save(&mut self) -> Result<(),CommandError> {
        self.dataset.flush_cache()?;
        Ok(())
    }

    pub(crate) fn points_layer(&self) -> Result<PointsLayer,CommandError> {
        PointsLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn triangles_layer(&self) -> Result<TrianglesLayer,CommandError> {
        TrianglesLayer::open_from_dataset(&self.dataset)
    }

    #[allow(dead_code)] pub(crate) fn tiles_layer(&self) -> Result<TilesLayer,CommandError> {
        TilesLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn load_points_layer<Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target_points = target.create_points_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing points.",generator.size_hint().1));
    
            for (i,point) in generator.enumerate() {
                target_points.add_point(point?)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Points written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving layer."); 
        
        self.save()?;
    
        progress.finish(|| "Layer saved.");
    
        Ok(())
    
    }

    pub(crate) fn load_triangles_layer<'lifetime, Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target = target.create_triangles_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing triangles.",generator.size_hint().1));
    
            for (i,triangle) in generator.enumerate() {
                target.add_triangle(triangle?.to_owned())?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Triangles written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())

    }

    pub(crate) fn load_tile_layer<'lifetime, Generator: Iterator<Item=Result<VoronoiTile,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target = target.create_tile_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing tiles.",generator.size_hint().1));
    
            for (i,tile) in generator.enumerate() {
                target.add_tile(tile?)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Tiles written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())

    }

    pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            tiles.calculate_neighbors(progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn sample_elevations_on_tiles<Progress: ProgressObserver>(&mut self, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            tiles.sample_elevations(raster,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn sample_ocean_on_tiles<Progress: ProgressObserver>(&mut self, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            tiles.sample_ocean(raster,method,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }



}

pub(crate) struct WorldMapTransaction<'lifetime> {
    dataset: Transaction<'lifetime>
}

impl<'lifetime> WorldMapTransaction<'lifetime> {

    fn new(dataset: Transaction<'lifetime>) -> Self {
        Self {
            dataset
        }
    }

    pub(crate) fn create_points_layer(&mut self, overwrite: bool) -> Result<PointsLayer,CommandError> {
        Ok(PointsLayer::create_from_dataset(&mut self.dataset, overwrite)?)       

    }

    pub(crate) fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TrianglesLayer,CommandError> {
        Ok(TrianglesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn create_tile_layer(&mut self, overwrite: bool) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn edit_tile_layer(&mut self) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::open_from_dataset(&mut self.dataset)?)

    }

    fn commit(self) -> Result<(),CommandError> {
        self.dataset.commit()?;
        Ok(())
    }

}

