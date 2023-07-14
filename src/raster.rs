use std::path::Path;

use gdal::Dataset;
use gdal::raster::Buffer;
use gdal::raster::GdalType;

use crate::errors::CommandError;
use crate::utils::Size;


pub struct RasterCoordTransformer {
    trans_x_min: f64,
    trans_x_size: f64,
    trans_y_min: f64,
    trans_y_size: f64
}

impl RasterCoordTransformer {

    pub fn pixels_to_coords(&self, x: f64, y: f64) -> (f64,f64) {
        // transform the point into lat/long TODO: I'm not sure if this is correct for both lat/lon versus metric coordinates
        // https://gis.stackexchange.com/a/299572
        let lon = x * self.trans_x_size + self.trans_x_min;
        let lat = y * self.trans_y_size + self.trans_y_min;
        (lon,lat)
    }
}

pub struct RasterBandBuffer<DataType: GdalType> {
    width: usize,
    buffer: Buffer<DataType>
}


impl<DataType: GdalType> RasterBandBuffer<DataType> {

    pub fn get_value(&self, x: f64, y: f64) -> Option<&DataType> {
        if y.is_sign_positive() && x.is_sign_positive() {
            let idx = ((y.floor() as usize) * self.width) + (x.floor() as usize);
            if idx < self.buffer.data.len() {
                Some(&self.buffer.data[idx])
            } else {
                None
            }
        } else {
            None
        }

    }
    
} 



pub struct RasterMap {
    dataset: Dataset
}

impl RasterMap {

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    pub fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open(path)?))
    }

    pub fn size(&self) -> Size<usize> {
        let (width,height) = self.dataset.raster_size();
        Size {
            width,
            height
        }
    }

    pub fn read_band<DataType: GdalType + Copy>(&self,index: isize) -> Result<RasterBandBuffer<DataType>,CommandError> {
        let band = if self.dataset.raster_count() > (index - 1) {
            self.dataset.rasterband(index)? // 1-based array
        } else {
            return Err(CommandError::RasterDatasetRequired)
        };

        let buffer = band.read_band_as::<DataType>()?;
        let width = self.dataset.raster_size().0;
        Ok(RasterBandBuffer { 
            buffer,
            width
        })
    }

    pub fn transformer(&self) -> Result<RasterCoordTransformer,CommandError> {
        let [trans_x_min,trans_x_size,_,trans_y_min,_,trans_y_size] = self.dataset.geo_transform()?;
        Ok(RasterCoordTransformer {
            trans_x_min,
            trans_x_size,
            trans_y_min,
            trans_y_size
        })
    }
}

