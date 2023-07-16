use std::path::Path;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::OGRwkbGeometryType::wkbPolygon;
use gdal::vector::Geometry;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Layer;
use gdal::vector::FeatureIterator;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::raster::RasterCoordTransformer;
use crate::raster::RasterBandBuffer;

pub const ELEVATION_FIELD_NAME: &str = "elevation";
pub const POINTS_LAYER_NAME: &str = "points";
pub const TRIANGLES_LAYER_NAME: &str = "triangles";

pub struct PointsLayer<'lifetime> {
    points: Layer<'lifetime>
}

impl<'lifetime> PointsLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let points = dataset.layer_by_name(POINTS_LAYER_NAME)?;
        Ok(Self {
            points
        })
    }

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let points = dataset.create_layer(LayerOptions {
            name: POINTS_LAYER_NAME,
            ty: wkbPoint,
            options: if overwrite {
                Some(&["OVERWRITE=YES"])
            } else {
                None
            },
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

    pub fn get_feature_count(&self) -> u64 {
        self.points.feature_count()
    }

    pub fn get_points(&mut self) -> FeatureIterator {
        self.points.features()
    }

}


pub struct TrianglesLayer<'lifetime> {
    tiles: Layer<'lifetime>
}

impl<'lifetime> TrianglesLayer<'lifetime> {

    fn _open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = dataset.layer_by_name(TRIANGLES_LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = dataset.create_layer(LayerOptions {
            name: TRIANGLES_LAYER_NAME,
            ty: wkbPolygon,
            options: if overwrite {
                Some(&["OVERWRITE=YES"])
            } else {
                None
            },
            ..Default::default()
        })?;
        // NOTE: I'm specifying the field value as real for now. Eventually I might want to allow it to choose a type based on the raster type, but there
        // really isn't much choice, just three numeric types (int, int64, and real)

        tiles.create_defn_fields(&[(ELEVATION_FIELD_NAME,OGRFieldType::OFTReal)])?;
        Ok(Self {
            tiles
        })
    }

    pub fn add_triangle(&mut self, geo: Geometry, elevation: Option<&f64>) -> Result<(),CommandError> {

        if let Some(value) = elevation {
            self.tiles.create_feature_fields(geo,&[ELEVATION_FIELD_NAME],&[FieldValue::RealValue(*value)])?
        } else {
            self.tiles.create_feature_fields(geo,&[],&[])?
        }
        Ok(())

    }


}

pub struct WorldMap {
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &str = "GPKG";

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    pub fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        let dataset = Dataset::open(path)?;
        Ok(Self::new(dataset))
    }


    pub fn edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open_ex(path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?))
    }

    pub fn create_or_edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(path)?;
            Ok(Self::new(dataset))
        }

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

    pub fn points_layer(&self) -> Result<PointsLayer,CommandError> {
        PointsLayer::open_from_dataset(&self.dataset)
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

    pub fn create_points_layer(&mut self, overwrite: bool) -> Result<PointsLayer,CommandError> {
        Ok(PointsLayer::create_from_dataset(&mut self.dataset, overwrite)?)       

    }

    pub fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TrianglesLayer,CommandError> {
        Ok(TrianglesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    fn commit(self) -> Result<(),CommandError> {
        self.dataset.commit()?;
        Ok(())
    }

}

