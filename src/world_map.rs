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
use gdal::vector::Layer;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::utils::LayerGeometryIterator;

pub(crate) const POINTS_LAYER_NAME: &str = "points";
pub(crate) const TRIANGLES_LAYER_NAME: &str = "triangles";

pub(crate) struct PointsLayer<'lifetime> {
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

        Ok(Self {
            points
        })
    }

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.points.create_feature_fields(point,&[],&[])?;
        Ok(())

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
        LayerGeometryIterator::new(&mut self.points)

    }


}



pub(crate) struct TrianglesLayer<'lifetime> {
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

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.tiles.create_feature_fields(geo,&[],&[])?;
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
            let mut target_points = target.create_triangles_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing triangles.",generator.size_hint().1));
    
            for (i,triangle) in generator.enumerate() {
                target_points.add_triangle(triangle?.to_owned())?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Triangles written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())

/*
    progress.start_known_endpoint(|| ("Writing triangles.",triangles.geometry_count()));
    target.with_transaction(|target| {

        let mut tiles = target.create_triangles_layer(overwrite_layer)?;

        for i in 0..triangles.geometry_count() {
            let geometry = triangles.get_geometry(i); // these are wkbPolygon
            tiles.add_triangle(geometry.clone(), None)?;
        }

        progress.finish(|| "Triangles written.");

        Ok(())
    })?;
 */        

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

    fn commit(self) -> Result<(),CommandError> {
        self.dataset.commit()?;
        Ok(())
    }

}

