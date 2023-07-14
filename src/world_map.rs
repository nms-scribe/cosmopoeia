use std::path::Path;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::Geometry;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Layer;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::raster::RasterCoordTransformer;
use crate::raster::RasterBandBuffer;

pub const ELEVATION_FIELD_NAME: &str = "elevation";
pub const POINTS_LAYER_NAME: &str = "points";

pub struct WorldPointsLayer<'lifetime> {
    points: Layer<'lifetime>
}

impl<'lifetime> WorldPointsLayer<'lifetime> {

    fn create_from_dataset(dataset: &'lifetime mut Dataset) -> Result<Self,CommandError> {
        let points = dataset.create_layer(LayerOptions {
            name: POINTS_LAYER_NAME,
            ty: wkbPoint,
            ..Default::default()
        })?;
        // NOTE: I'm specifying the field value as real for now. Eventually I might want to allow it to choose a type based on the raster type, but there
        // really isn't much choice, just three numeric types (int, int64, and real)

        points.create_defn_fields(&[(ELEVATION_FIELD_NAME,OGRFieldType::OFTReal)])?;
        Ok(Self {
            points
        })
    }

    fn add_point(&mut self, lon: f64, lat: f64, elevation: Option<&f64>) -> Result<(),CommandError> {
        let mut point = Geometry::empty(wkbPoint)?;
        point.add_point_2d((lon,lat));

        if let Some(value) = elevation {
            self.points.create_feature_fields(point,&[ELEVATION_FIELD_NAME],&[FieldValue::RealValue(*value)])?
        } else {
            self.points.create_feature_fields(point,&[],&[])?
        }
        Ok(())

    }

    pub fn sample_point_from_raster(&mut self, x: f64, y: f64, transformer: &RasterCoordTransformer, buffer: &RasterBandBuffer<f64>) -> Result<(),CommandError> {
        let (lon,lat) = transformer.pixels_to_coords(x, y);
        self.add_point(lon, lat, buffer.get_value(x, y))
    }
}

pub struct WorldMap {
    dataset: Dataset
}

impl WorldMap {

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    pub fn create<FilePath: AsRef<Path>>(driver: &str, path: FilePath) -> Result<Self,CommandError> {
        let driver = DriverManager::get_driver_by_name(&driver)?;
        let dataset = driver.create_vector_only(path)?;
        Ok(Self::new(dataset))

    }

    pub fn with_transaction<Callback: FnMut(&mut WorldMapTransaction) -> Result<(),CommandError>>(&mut self, mut callback: Callback) -> Result<(),CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        callback(&mut transaction)?;
        transaction.commit()?;
        Ok(())

    }

    pub fn save(&mut self) -> Result<(),CommandError> {
        self.dataset.flush_cache()?;
        Ok(())
    }

}

pub struct WorldMapTransaction<'lifetime> {
    dataset: Transaction<'lifetime>
}

impl<'lifetime> WorldMapTransaction<'lifetime> {

    fn new(dataset: Transaction<'lifetime>) -> Self {
        Self {
            dataset
        }
    }

    pub fn create_points_layer(&mut self) -> Result<WorldPointsLayer,CommandError> {
        Ok(WorldPointsLayer::create_from_dataset(&mut self.dataset)?)       

    }

    fn commit(self) -> Result<(),CommandError> {
        self.dataset.commit()?;
        Ok(())
    }

}

