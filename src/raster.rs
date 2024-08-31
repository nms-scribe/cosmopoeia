use std::path::Path;

use gdal::Dataset;
use gdal::raster::Buffer;
use gdal::raster::GdalType;

use crate::errors::CommandError;
use crate::utils::extent::Extent;
use crate::world_map::property_layer::ElevationLimits;

#[derive(Debug)]
pub(crate) struct RasterBounds {
    coord_min_x: f64,
    transform_x_factor: f64,
    coord_min_y: f64,
    transform_y_factor: f64,
    pixel_width: usize,
    pixel_height: usize,
}

impl RasterBounds {

    pub(crate) fn coords_to_pixels(&self, lon: f64, lat: f64) -> (f64,f64) {
        let x = (lon - self.coord_min_x)/self.transform_x_factor;
        // rasters are stored upside down. GeoTIFF is an extension of a graphical format,
        // which usually has y = 0 at the top, but geographic coordinates usually have
        // lower y at the bottom. This is why the geotransform parameters usually have a
        // negative y.
        let y = self.pixel_height as f64 - (lat - self.coord_min_y)/self.transform_y_factor;
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



pub(crate) struct RasterBandBuffer<DataType: GdalType> {
    width: usize,
    buffer: Buffer<DataType>,
    no_data: Option<f64>
}


impl<DataType: GdalType> RasterBandBuffer<DataType> {

    pub(crate) fn get_value(&self, x: f64, y: f64) -> Option<&DataType> {
        if y.is_sign_positive() && x.is_sign_positive() {
            let idx = ((y.floor() as usize) * self.width) + (x.floor() as usize);
            let data = self.buffer.data();
            if idx < data.len() {
                Some(&data[idx])
            } else {
                None
            }
        } else {
            None
        }

    }

    pub(crate) const fn no_data_value(&self) -> &Option<f64> {
        &self.no_data
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

    pub(crate) fn read_band<DataType: GdalType + Copy>(&self,index: usize) -> Result<RasterBandBuffer<DataType>,CommandError> {
        let band = if self.dataset.raster_count() > (index - 1) {
            self.dataset.rasterband(index)? // 1-based array
        } else {
            return Err(CommandError::RasterDatasetRequired)
        };

        let buffer = band.read_band_as::<DataType>()?;
        let width = self.dataset.raster_size().0;
        let no_data = band.no_data_value();
        Ok(RasterBandBuffer { 
            width, 
            buffer, 
            no_data 
        })
    }

    pub(crate) fn bounds(&self) -> Result<RasterBounds,CommandError> {
        let [coord_left,transform_x_factor,_,coord_top,_,transform_y_factor] = self.dataset.geo_transform()?;
        let (pixel_width,pixel_height) = self.dataset.raster_size();
        // the transform_factor is usually negative, because GeoTIFFs are upside down.
        let transform_y_factor = -transform_y_factor;
        let coord_height = pixel_height as f64 * transform_y_factor;
        let coord_bottom = coord_top - coord_height;
    
        Ok(RasterBounds { 
            coord_min_x: coord_left, 
            transform_x_factor, 
            coord_min_y: coord_bottom, 
            transform_y_factor, 
            pixel_width, 
            pixel_height 
        })

    }

    pub(crate) fn compute_min_max(&self, index: usize, is_approx_ok: bool) -> Result<ElevationLimits,CommandError> {
        let band = if self.dataset.raster_count() > (index - 1) {
            self.dataset.rasterband(index)? // 1-based array
        } else {
            return Err(CommandError::RasterDatasetRequired)
        };

        let statistics = band.compute_raster_min_max(is_approx_ok)?;
        ElevationLimits::new(statistics.min,statistics.max)
    }


}

