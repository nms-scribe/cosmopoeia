use gdal::Dataset;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::Geometry;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use rand::Rng;

use crate::errors::CommandError;

pub const DEFAULT_POINT_COUNT: f64 = 10_000.0;
pub const ELEVATION_FIELD_NAME: &str = "elevation";

macro_rules! round {
    ($value: expr, 2) => {
        ($value * 100.0).round() / 100.0
    };
}



// TODO: Allow passing a progress tracking closure. Would have to be able to calculate the others.
pub fn generate_points_from_heightmap<Random: Rng>(source: Dataset, target: &mut Dataset, spacing: Option<f64>, random: &mut Random) -> Result<(),CommandError> {

    let (width,height) = source.raster_size(); 
    let [trans_x_min,trans_x_size,_,trans_y_min,_,trans_y_size] = source.geo_transform()?;
    let source_band = if source.raster_count() > 0 {
        source.rasterband(1)? // 1-based array
    } else {
        return Err(CommandError::RasterSourceRequired)
    };

    let source_buffer = source_band.read_band_as::<f64>()?;

    let mut target = target.start_transaction()?;
    let mut points = target.create_layer(LayerOptions {
        ty: wkbPoint,
        ..Default::default()
    })?;
    // NOTE: I'm specifying the result for now. Eventually I might want to allow it to choose a type based on the raster type, but there
    // really isn't much choice, just three numeric types (int, int64, and real)
    points.create_defn_fields(&[(ELEVATION_FIELD_NAME,OGRFieldType::OFTReal)])?;

    macro_rules! add_point {
        ($x: expr, $y: expr) => {
            let x = $x;
            let y = $y;

            // transform the point into lat/long TODO: I'm not sure if this is correct for both lat/lon versus metric coordinates
            // https://gis.stackexchange.com/a/299572
            let lon = x * trans_x_size + trans_x_min;
            let lat = y * trans_y_size + trans_y_min;
            let mut point = Geometry::empty(wkbPoint)?;
            point.add_point_2d((lon,lat));

            if y.is_sign_positive() && x.is_sign_positive() {
                let idx = ((y.floor() as usize) * width) + (x.floor() as usize);
                if idx < source_buffer.data.len() {
                    points.create_feature_fields(point,&[ELEVATION_FIELD_NAME],&[FieldValue::RealValue(source_buffer.data[idx])])?; 
                } else {
                    points.create_feature_fields(point,&[],&[])?; 
                }
            } else {
                points.create_feature_fields(point,&[],&[])?; 
            }
        };
    }

    let width = width as f64;
    let height = height as f64;
    // round spacing for simplicity FUTURE: Do I really need to do this?
    let spacing = round!(if let Some(spacing) = spacing {
        spacing
    } else {
        ((width * height)/DEFAULT_POINT_COUNT).sqrt()
    },2);

    // boundary points

    // TODO: The points laying beyond the edge of the heightmap looks weird. Once I get to the voronoi, see if those are absolutely necessary.
    // TODO: Those boundary points should also be jittered, at least along the line.

    let offset = -1.0 * spacing; // -10.0
    let boundary_spacing: f64 = spacing * 2.0; // 20.0
    let boundary_width = width - offset * 2.0; // 532
    let boundary_height = height - offset * 2.0; // 532
    let number_x = (boundary_width/boundary_spacing).ceil() - 1.0; // 26
    let number_y = (boundary_height/boundary_spacing).ceil() - 1.0; // 26

    let mut i = 0.5;
    while i < number_x {
        let x = ((boundary_width*i)/number_x + offset).ceil(); // 
        add_point!(x,offset);
        add_point!(x,boundary_height+offset);
        i += 1.0;
    }

    let mut i = 0.5;
    while i < number_y {
        let y = ((boundary_height*i)/number_y + offset).ceil();
        add_point!(offset,y);
        add_point!(boundary_width+offset,y);
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
    while y < height {
        let mut x = radius;
        while x < width {
            let x_j = round!(x + jitter!(),2).min(width);
            let y_j = round!(y + jitter!(),2).min(height);
            add_point!(x_j,y_j);
            x += spacing;
        }
        y += spacing;
    }

    target.commit()?;

    Ok(())

/*
`placePoints`: `graphWidth` and `graphHeight` appear to be the width and height of the canvas
```
function placePoints() {
  TIME && console.time("placePoints");
  const cellsDesired = +byId("pointsInput").dataset.cells; // NMS: This is `Options` > `Points Number`
  const spacing = round(Math.sqrt((graphWidth * graphHeight) / cellsDesired), 2); // spacing between points before jirrering

  const boundary = getBoundaryPoints(graphWidth, graphHeight, spacing);
  const points = getJitteredGrid(graphWidth, graphHeight, spacing); // points of jittered square grid
  const cellsX = Math.floor((graphWidth + 0.5 * spacing - 1e-10) / spacing);
  const cellsY = Math.floor((graphHeight + 0.5 * spacing - 1e-10) / spacing);
  TIME && console.timeEnd("placePoints");

  return {spacing, cellsDesired, boundary, points, cellsX, cellsY};
}
```

`getBoundaryPoints`
```
function getBoundaryPoints(width, height, spacing) {
  const offset = rn(-1 * spacing);
  const bSpacing = spacing * 2;
  const w = width - offset * 2;
  const h = height - offset * 2;
  const numberX = Math.ceil(w / bSpacing) - 1;
  const numberY = Math.ceil(h / bSpacing) - 1;
  const points = [];

  for (let i = 0.5; i < numberX; i++) {
    let x = Math.ceil((w * i) / numberX + offset);
    points.push([x, offset], [x, h + offset]);
  }

  for (let i = 0.5; i < numberY; i++) {
    let y = Math.ceil((h * i) / numberY + offset);
    points.push([offset, y], [w + offset, y]);
  }

  return points;
}
```

`getJitteredGrid`
```
function getJitteredGrid(width, height, spacing) {
  const radius = spacing / 2; // square radius
  const jittering = radius * 0.9; // max deviation
  const doubleJittering = jittering * 2;
  const jitter = () => Math.random() * doubleJittering - jittering;

  let points = [];
  for (let y = radius; y < height; y += spacing) {
    for (let x = radius; x < width; x += spacing) {
      const xj = Math.min(rn(x + jitter(), 2), width);
      const yj = Math.min(rn(y + jitter(), 2), height);
      points.push([xj, yj]);
    }
  }
  return points;
}
```
 */
}