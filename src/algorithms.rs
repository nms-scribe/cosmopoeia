use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbGeometryCollection;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Size;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;

pub const DEFAULT_POINT_COUNT: f64 = 10_000.0;


// TODO: Also need to have a raster ocean mask, but only as an option.
pub fn generate_points_from_heightmap<Random: Rng, Progress: ProgressObserver>(source: RasterMap, target: &mut WorldMap, overwrite_layer: bool, spacing: Option<f64>, random: &mut Random, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

    progress.start_unknown_endpoint(|| "Loading raster source.");
    // Sampling and randomizing algorithms borrowed from AFMG with many modifications

    let source_transformer = source.transformer()?;
    let source_buffer = source.read_band::<f64>(1)?;
    let source_size = Size::<f64>::from_usize(source.size());

    progress.finish(|| "Raster Loaded.");

    target.with_transaction(|target| {
        let mut target_points = target.create_points_layer(overwrite_layer)?;

    
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
    
        progress.start_known_endpoint(|| ("Generating points.",(number_x + number_y + source_size.height * source_size.width).floor() as usize));

        let mut i = 0.5;
        while i < number_x {
            let x = ((boundary_width*i)/number_x + offset).ceil(); // 
            target_points.sample_point_from_raster(x,offset,&source_transformer,&source_buffer)?;
            target_points.sample_point_from_raster(x,boundary_height+offset,&source_transformer,&source_buffer)?;
            progress.update(|| i.floor() as usize);
            i += 1.0;
        }
    
        let mut i = 0.5;
        while i < number_y {
            let y = ((boundary_height*i)/number_y + offset).ceil();
            target_points.sample_point_from_raster(offset,y,&source_transformer,&source_buffer)?;
            target_points.sample_point_from_raster(boundary_width+offset,y,&source_transformer,&source_buffer)?;
            progress.update(|| (number_x + i).floor() as usize);
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
                target_points.sample_point_from_raster(x_j,y_j,&source_transformer,&source_buffer)?;
                progress.update(|| (number_x + number_y + y * source_size.width + x).floor() as usize);
                x += spacing;
            }
            y += spacing;
        }

        progress.finish(|| "Points Generated.");

        Ok(())
    })?;

    progress.start_unknown_endpoint(|| "Saving Layer..."); 
    
    target.save()?;

    progress.finish(|| "Layer Saved.");

    Ok(())

}

// TODO: I'm leaning more and more into keeping everything in a single gpkg file as standard, as those can support multiple layers. I might
// even be able to store the non-geographic lookup tables with wkbNone geometries. I'm just not certain what to do with those.
pub fn generate_delaunary_triangles_from_points<Progress: ProgressObserver>(target: &mut WorldMap, overwrite_layer: bool, tolerance: Option<f64>, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

    let mut points = target.points_layer()?;

    // the delaunay_triangulation procedure requires a single geometry. Which means I've got to read all the points into one thingie.
    progress.start_known_endpoint(|| ("Reading points.",points.get_feature_count() as usize));
    let mut all_points = Geometry::empty(wkbGeometryCollection)?;
    for (i,point) in points.get_points().enumerate() {
        if let Some(geometry) = point.geometry() {
            all_points.add_geometry(geometry.clone())?;
        }
        progress.update(|| i);
    }
    progress.finish(|| "Points read.");

    progress.start_unknown_endpoint(|| "Generating triangles.");
    let triangles = all_points.delaunay_triangulation(tolerance)?; // TODO: Include snapping tolerance as a configuration.

    progress.finish(|| "Triangles generated.");

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


    progress.start_unknown_endpoint(|| "Saving Layer..."); // FUTURE: The progress bar can't update during this part, we should change the appearance somehow.
    
    target.save()?;

    progress.finish(|| "Layer Saved.");

    Ok(())

    // TODO: Now I need to actually create the voronoi and get elevation and other data off of them from the points.
    // It's basically a matter of finding the centroids of all triangles around a shared vertex and connecting them
    // into polygons. Basically it's like this:
    // - for each point
    //   - find all triangles which meet at that point
    //   - find the centroids of those triangles
    //   - create a polygon with the centroids of those triangles as vertexes.
    // - if there were a way of adding "metadata" to the triangle geometries created above, that might help. But if not I'll have to go back
    //   to the point layer.

}