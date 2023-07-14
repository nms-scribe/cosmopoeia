use gdal::Dataset;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::Geometry;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Layer;
use gdal::raster::Buffer;
use gdal::raster::GdalType;
use rand::Rng;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;

pub const DEFAULT_POINT_COUNT: f64 = 10_000.0;
pub const ELEVATION_FIELD_NAME: &str = "elevation";

struct RasterCoordTransformer {
    trans_x_min: f64,
    trans_x_size: f64,
    trans_y_min: f64,
    trans_y_size: f64
}

impl RasterCoordTransformer {

    fn pixels_to_coords(&self, x: f64, y: f64) -> (f64,f64) {
        // transform the point into lat/long TODO: I'm not sure if this is correct for both lat/lon versus metric coordinates
        // https://gis.stackexchange.com/a/299572
        let lon = x * self.trans_x_size + self.trans_x_min;
        let lat = y * self.trans_y_size + self.trans_y_min;
        (lon,lat)
    }
}

struct ElevationBandBuffer<DataType: GdalType> {
    width: usize,
    buffer: Buffer<DataType>
}


impl<DataType: GdalType> ElevationBandBuffer<DataType> {

    fn get_value(&self, x: f64, y: f64) -> Option<&DataType> {
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

struct Size<DataType> {
    height: DataType,
    width: DataType
}

impl Size<f64> {

    fn from_usize(source: Size<usize>) -> Self {
        let width = source.width as f64;
        let height = source.height as f64;
        Self {
            width,
            height
        }
    }
}

struct Heightmap {
    dataset: Dataset
}

impl Heightmap {

    fn new(dataset: Dataset) -> Result<Self,CommandError> {
    
        Ok(Self {
            dataset
        })
    }

    fn size(&self) -> Size<usize> {
        let (width,height) = self.dataset.raster_size();
        Size {
            width,
            height
        }
    }

    fn read_band<DataType: GdalType + Copy>(&self,index: isize) -> Result<ElevationBandBuffer<DataType>,CommandError> {
        let band = if self.dataset.raster_count() > (index - 1) {
            self.dataset.rasterband(index)? // 1-based array
        } else {
            return Err(CommandError::RasterDatasetRequired)
        };

        let buffer = band.read_band_as::<DataType>()?;
        let width = self.dataset.raster_size().0;
        Ok(ElevationBandBuffer { 
            buffer,
            width
        })
    }

    fn transformer(&self) -> Result<RasterCoordTransformer,CommandError> {
        let [trans_x_min,trans_x_size,_,trans_y_min,_,trans_y_size] = self.dataset.geo_transform()?;
        Ok(RasterCoordTransformer {
            trans_x_min,
            trans_x_size,
            trans_y_min,
            trans_y_size
        })
    }
}

struct TerrainPointsLayer<'lifetime> {
    points: Layer<'lifetime>
}

impl<'lifetime> TerrainPointsLayer<'lifetime> {

    fn create_from_dataset(dataset: &'lifetime mut Dataset) -> Result<Self,CommandError> {
        let points = dataset.create_layer(LayerOptions {
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

    fn sample_point(&mut self, x: f64, y: f64, transformer: &RasterCoordTransformer, buffer: &ElevationBandBuffer<f64>) -> Result<(),CommandError> {
        let (lon,lat) = transformer.pixels_to_coords(x, y);
        self.add_point(lon, lat, buffer.get_value(x, y))
    }
}


// TODO: Allow passing a progress tracking closure. Would have to be able to calculate the others.
// TODO: Also need to have a raster ocean mask, but only as an option.
pub fn generate_points_from_heightmap<Random: Rng>(source: Dataset, target: &mut Dataset, spacing: Option<f64>, random: &mut Random) -> Result<(),CommandError> {

    // Sampling and randomizing algorithms borrowed from AFMG with many modifications

    let source = Heightmap::new(source)?;
    let source_transformer = source.transformer()?;
    let source_buffer = source.read_band::<f64>(1)?;
    let source_size = Size::<f64>::from_usize(source.size());

    let mut target = target.start_transaction()?;
    let mut target_points = TerrainPointsLayer::create_from_dataset(&mut target)?;

    
    // round spacing for simplicity FUTURE: Do I really need to do this?
    let spacing = if let Some(spacing) = spacing {
        spacing.round_hundredths()
    } else {
        ((source_size.width * source_size.height)/DEFAULT_POINT_COUNT).sqrt().round_hundredths()
    };

    // boundary points

    // TODO: The points laying beyond the edge of the heightmap looks weird. Once I get to the voronoi, see if those are absolutely necessary.
    // TODO: Those boundary points should also be jittered, at least along the line.

    let offset = -1.0 * spacing; // -10.0
    let boundary_spacing: f64 = spacing * 2.0; // 20.0
    let boundary_width = source_size.width - offset * 2.0; // 532
    let boundary_height = source_size.height - offset * 2.0; // 532
    let number_x = (boundary_width/boundary_spacing).ceil() - 1.0; // 26
    let number_y = (boundary_height/boundary_spacing).ceil() - 1.0; // 26

    let mut i = 0.5;
    while i < number_x {
        let x = ((boundary_width*i)/number_x + offset).ceil(); // 
        target_points.sample_point(x,offset,&source_transformer,&source_buffer)?;
        target_points.sample_point(x,boundary_height+offset,&source_transformer,&source_buffer)?;
        i += 1.0;
    }

    let mut i = 0.5;
    while i < number_y {
        let y = ((boundary_height*i)/number_y + offset).ceil();
        target_points.sample_point(offset,y,&source_transformer,&source_buffer)?;
        target_points.sample_point(boundary_width+offset,y,&source_transformer,&source_buffer)?;
        i += 1.0;
    }

    // jittered internal points
    let radius = spacing / 2.0;
    let jittering = radius * 0.9; // FUTURE: Customizable factor?
    let double_jittering = jittering * 2.0;

    macro_rules! jitter {
        () => {
            // gen creates random number between >= 0.0, < 1.0
            random.gen::<f64>() * double_jittering - jittering    
        };
    }

    let mut y = radius;
    while y < source_size.height {
        let mut x = radius;
        while x < source_size.width {
            let x_j = (x + jitter!()).round_hundredths().min(source_size.width);
            let y_j = (y + jitter!()).round_hundredths().min(source_size.height);
            target_points.sample_point(x_j,y_j,&source_transformer,&source_buffer)?;
            x += spacing;
        }
        y += spacing;
    }

    target.commit()?;

    Ok(())

}