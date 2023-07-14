use rand::Rng;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Size;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;

pub const DEFAULT_POINT_COUNT: f64 = 10_000.0;


// TODO: Allow passing a progress tracking closure. Would have to be able to calculate the others.
// TODO: Also need to have a raster ocean mask, but only as an option.
pub fn generate_points_from_heightmap<Random: Rng, Progress: ProgressObserver>(source: RasterMap, target: &mut WorldMap, spacing: Option<f64>, random: &mut Random, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

    progress.start(|| ("Loading raster source.", None));
    // Sampling and randomizing algorithms borrowed from AFMG with many modifications

    let source_transformer = source.transformer()?;
    let source_buffer = source.read_band::<f64>(1)?;
    let source_size = Size::<f64>::from_usize(source.size());

    progress.finish(|| "Raster Loaded.");

    target.with_transaction(|target| {
        let mut target_points = target.create_points_layer()?;

    
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
    
        progress.start(|| ("Generating points.",Some((number_x + number_y + source_size.height * source_size.width).floor() as usize)));

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

    progress.start(|| ("Saving Layer...", None)); // FUTURE: The progress bar can't update during this part, we should change the appearance somehow.
    
    target.save()?;

    progress.finish(|| "Layer Saved.");

    Ok(())

}