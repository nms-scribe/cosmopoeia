use std::path::PathBuf;

use clap::Args;

use crate::errors::CommandError;
use crate::world_map::EntityIndex;
use crate::world_map::TileSchema;
use crate::world_map::TileForTerrain;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::raster::RasterMap;
use crate::world_map::Grouping;
use crate::subcommand_def;

/*
TODO: (see ideas for algorithms)
  * Recipe(path)
  * AddHill(count: Range<usize>, height: Range, range_x, range_y); -- can also work to add pits if the height range is negative
  * AddRange(count: Range, height: Range, range_x, range_y); -- can also work to add a trench if the height range is negative
  * AddStrait(width: Range, direction: (Horizontal,Vertical));
  * Mask(power = 1);
  * Invert(probability, axes: X, Y, or XY); -- probability is a probability that the inversion will actually happen
  * Modify(range: Range<f64>, add, mult); -- range is a range of elevations to process. add is a number to add to the elevation (or 0), mul is a number to multiply (or 1)
  * Smooth(a2);
  * SeedOcean(seeds: usize, range_x, range_y)
  * FillOcean
  * SampleHeightmap(path)
  * SampleOceanMask(path,method: OceanSamplingMethod)
  * FloodOcean

*/

pub(crate) struct TerrainSettings {
    min_elevation: f64,
    max_elevation: f64
}

impl TerrainSettings {

    pub(crate) fn new(min_elevation: f64, max_elevation: f64) -> Result<Self,CommandError> {
        if max_elevation < 0.0 {
            Err(CommandError::MaxElevationMustBePositive(max_elevation))
            // FUTURE: or should it? What if they want to create an underwater world? That won't be possible until we allow mermaid-like cultures, however,
            // and I'm not sure how "biomes" work down there.
        } else if min_elevation >= max_elevation {
            // it doesn't necessarily have to be negative, however.
            Err(CommandError::MinElevationMustBeLess(min_elevation,max_elevation))
        } else {
            Ok(Self {
                min_elevation,
                max_elevation,
            })
        }
    }

    pub(crate) fn from_raster<Progress: ProgressObserver>(raster: &RasterMap, progress: &mut Progress) -> Result<Self,CommandError> {
        progress.start_unknown_endpoint(|| "Calculating min/max from raster.");
        let (min_elevation,max_elevation) = raster.compute_min_max(1,true)?;
        progress.finish(|| "Min/max calculated.");
        // FUTURE: Should I use progress to output this? I want to make sure the user sees this when it's calculated,
        // they'll need it for the next one.
        println!("Using min elevation {}",min_elevation);
        println!("Using max elevation {}",max_elevation);

        Self::new(min_elevation,max_elevation)

    }
}

trait ProcessTerrain {

    fn process_terrain_tiles<Progress: ProgressObserver>(&self, settings: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError>;
}

subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean.
    pub(crate) struct SampleOceanBelow {

        /// The raster to sample from
        source: PathBuf,

        /// The elevation to compare to
        #[arg(allow_negative_numbers=true)]
        elevation: f64
    }
}

impl ProcessTerrain for SampleOceanBelow {

    fn process_terrain_tiles<Progress: ProgressObserver>(&self, _: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling ocean data");

        progress.start_unknown_endpoint(|| "Reading raster");

        let raster = RasterMap::open(&self.source)?;

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;
        let no_data_value = band.no_data_value();
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling oceans.","Oceans sampled.") {
    
            let (x,y) = bounds.coords_to_pixels(tile.site_x, tile.site_y);

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                let is_no_data = match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => elevation.is_nan(),
                    Some(no_data_value) => elevation == no_data_value,
                    None => false,
                };

                if !is_no_data {
                    elevation < &self.elevation
                } else {
                    false
                }


            } else {

                false

            };

            // only apply if the data actually is ocean now, so one can use multiple ocean methods
            if is_ocean {
                tile.grouping = Grouping::Ocean;
            }

        }
    
        Ok(())        
    }
}

subcommand_def!{

    /// Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean.
    pub(crate) struct SampleOceanMasked {

        /// The raster to read ocean data from
        source: PathBuf
    }
}

impl ProcessTerrain for SampleOceanMasked {

    fn process_terrain_tiles<Progress: ProgressObserver>(&self, _: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling ocean data");

        progress.start_unknown_endpoint(|| "Reading raster");

        let raster = RasterMap::open(&self.source)?;

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;
        let no_data_value = band.no_data_value();
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling oceans.","Oceans sampled.") {
    
            let (x,y) = bounds.coords_to_pixels(tile.site_x, tile.site_y);

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => !elevation.is_nan(),
                    Some(no_data_value) => elevation != no_data_value,
                    None => true,
                }

            } else {

                false

            };

            // only apply if the data actually is ocean now, so one can use multiple ocean methods
            if is_ocean {
                tile.grouping = Grouping::Ocean;
            }

        }
    
        Ok(())
    }
}

pub(crate) struct SampleElevationWithRaster {
    // TODO: Once complete, change the cli commands to use this.
    raster: RasterMap
}
impl SampleElevationWithRaster {
    pub(crate) fn new(raster: RasterMap) -> TerrainProcess {
        TerrainProcess::SampleElevationWithRaster(Self {
            raster
        })
    }
}

impl ProcessTerrain for SampleElevationWithRaster {

    fn process_terrain_tiles<Progress: ProgressObserver>(&self, _: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        // NOTE: No 'announce', this is called by functions which have already announced the algorithm.

        progress.start_unknown_endpoint(|| "Reading raster");

        let raster = &self.raster;

        let band = raster.read_band::<f64>(1)?;
        let bounds = raster.bounds()?;
    
        progress.finish(|| "Raster read.");
    
        for (_,tile) in tile_map.iter_mut().watch(progress,"Sampling elevations.","Elevations sampled.") {
    
    
            let (x,y) = bounds.coords_to_pixels(tile.site_x, tile.site_y);
    
            if let Some(elevation) = band.get_value(x, y) {

                tile.elevation = *elevation;
    
            }
    
    
        }

        Ok(())
    }
}

subcommand_def!{

    /// Replaces elevations by sampling from a heightmap
    pub(crate) struct SampleElevation {

        /// Raster file defining new elevations
        source: PathBuf
    }
}

impl ProcessTerrain for SampleElevation {
    fn process_terrain_tiles<Progress: ProgressObserver>(&self, settings: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {

        progress.announce("Sampling elevations");

        let raster = RasterMap::open(&self.source)?;

        SampleElevationWithRaster { raster }.process_terrain_tiles(settings, tile_map, progress)
    
    }
}



pub(crate) enum TerrainProcess {
    SampleOceanMasked(SampleOceanMasked),
    SampleOceanBelow(SampleOceanBelow),
    SampleElevation(SampleElevation),
    // Create-from-heightmap already has the raster open, this makes things easier to get this done.
    SampleElevationWithRaster(SampleElevationWithRaster)
}

impl TerrainProcess {

    pub(crate) fn process_terrain<Progress: ProgressObserver>(&self,settings: TerrainSettings, target: &mut WorldMapTransaction, progress: &mut Progress) -> Result<(),CommandError> {


        let mut layer = target.edit_tile_layer()?;

        let mut tile_map = layer.read_features().to_entities_index::<_,TileForTerrain>(progress)?;

        let positive_elevation_scale = 80.0/settings.max_elevation;
        let negative_elevation_scale = if settings.min_elevation < 0.0 { 
            20.0/settings.min_elevation.abs()
        } else {
            0.0
        };
    
    //    * find the max_elevation from the raster, if possible
    //    * find the absolute value of the min_elevation from the raster, if possible
    //    * if elevation >= 0
    //      * elevation_scaled = (elevation*80)/max_elevation
    //    * else
    //      * elevation_scaled = 20 - (elevation.abs()*20)/min_elevation.abs()

        self.process_terrain_tiles(settings, &mut tile_map, progress)?;

        let mut bad_ocean_tiles_found = Vec::new();
    
        for (fid,tile) in tile_map.into_iter().watch(progress,"Writing data.","Data written.") {

            // TODO: Also need to update elevation_scale in all of this...
            let elevation_changed = tile.elevation_changed();
            let grouping_changed = tile.grouping_changed();
            if elevation_changed || grouping_changed {
                let mut feature = layer.try_feature_by_id(&fid)?;
                if elevation_changed {

                    let elevation = tile.elevation;
                    let elevation_scaled = if elevation >= 0.0 {
                        20 + (elevation * positive_elevation_scale).floor() as i32
                    } else {
                        20 - (elevation.abs() * negative_elevation_scale).floor() as i32
                    };
    
   
                    feature.set_elevation(elevation)?;
                    feature.set_elevation_scaled(elevation_scaled)?;
                }
                if grouping_changed {

                    // warn user if a tile was set to ocean that's above 0.
                    if matches!(tile.grouping,Grouping::Ocean) && (tile.elevation > 0.0) {
                        bad_ocean_tiles_found.push(fid);
                    }        
        
                    // Should I check to make sure?
                    feature.set_grouping(&tile.grouping)?;
                }
                layer.update_feature(feature)?;

            }

        }

        if bad_ocean_tiles_found.len() > 0 {
            progress.warning(|| format!("At least one ocean tile was found with an elevation above 0 (id: {}).",bad_ocean_tiles_found[0]))
        }



        Ok(())
    }



}

impl ProcessTerrain for TerrainProcess {

    fn process_terrain_tiles<Progress: ProgressObserver>(&self, settings: TerrainSettings, tile_map: &mut EntityIndex<TileSchema,TileForTerrain>, progress: &mut Progress) -> Result<(),CommandError> {
        match self {
            Self::SampleOceanMasked(params) => params.process_terrain_tiles(settings,tile_map,progress),
            Self::SampleOceanBelow(params) => params.process_terrain_tiles(settings,tile_map,progress),
            Self::SampleElevation(params) => params.process_terrain_tiles(settings,tile_map,progress),
            Self::SampleElevationWithRaster(params) => params.process_terrain_tiles(settings,tile_map,progress),
        }
    }

}