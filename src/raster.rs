use std::path::Path;

use gdal::Dataset;
use gdal::raster::Buffer;
use gdal::raster::GdalType;

use crate::errors::CommandError;
use crate::utils::Extent;

pub(crate) struct RasterBounds {
    coord_min_x: f64,
    transform_x_factor: f64,
    coord_min_y: f64,
    transform_y_factor: f64,
    pixel_width: usize, // TODO: Should be usize
    pixel_height: usize, // TODO: Should be usize
}

impl RasterBounds {

    #[allow(dead_code)] pub(crate) fn pixels_to_coords(&self, x: f64, y: f64) -> (f64,f64) {
        // transform the point into lat/long TODO: I'm not sure if this is correct for both lat/lon versus metric coordinates
        // https://gis.stackexchange.com/a/299572
        let lon = x * self.transform_x_factor + self.coord_min_x;
        let lat = y * self.transform_y_factor + self.coord_min_y;
        (lon,lat)
    }

    #[allow(dead_code)] pub(crate) fn coords_to_pixels(&self, lon: f64, lat: f64) -> (f64,f64) {
        // this is just the reverse of the other
        let x = (lon - self.coord_min_x)/self.transform_x_factor;
        let y = (lat - self.coord_min_y)/self.transform_y_factor;
        (x,y)

    }

    pub(crate) fn coord_width(&self) -> f64 {
        (self.pixel_width as f64 * self.transform_x_factor).abs()
    }

    pub(crate) fn coord_height(&self) -> f64 {
        (self.pixel_height as f64 * self.transform_y_factor).abs()
    }

    pub(crate) fn extent(&self) -> Extent {
        Extent {
            height: self.coord_height(),
            width: self.coord_width(),
            south: if self.transform_y_factor >= 0.0 {
                self.coord_min_y
            } else {
                -self.coord_min_y
            },
            west: if self.transform_x_factor >= 0.0 {
                self.coord_min_x
            } else {
                -self.coord_min_x
            },
        }

    }


}



#[allow(dead_code)] pub(crate) struct RasterBandBuffer<DataType: GdalType> {
    width: usize,
    buffer: Buffer<DataType>
}


impl<DataType: GdalType> RasterBandBuffer<DataType> {

    #[allow(dead_code)] pub(crate) fn get_value(&self, x: f64, y: f64) -> Option<&DataType> {
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



pub(crate) struct RasterMap {
    dataset: Dataset
}

impl RasterMap {

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    pub(crate) fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open(path)?))
    }

    pub(crate) fn read_band<DataType: GdalType + Copy>(&self,index: isize) -> Result<RasterBandBuffer<DataType>,CommandError> {
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

    pub(crate) fn bounds(&self) -> Result<RasterBounds,CommandError> {
        let [coord_left,transform_x_factor,_,coord_bottom,_,transform_y_factor] = self.dataset.geo_transform()?;
        let (pixel_width,pixel_height) = self.dataset.raster_size();
        Ok(RasterBounds { 
            coord_min_x: coord_left, 
            transform_x_factor, 
            coord_min_y: coord_bottom, 
            transform_y_factor, 
            pixel_width, 
            pixel_height 
        })

    }
}

