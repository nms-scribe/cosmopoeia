use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::collections::hash_map::Entry;
use std::cmp::Ordering;
use std::collections::HashSet;

use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::OGRwkbGeometryType::wkbPolygon;
use ordered_float::NotNan;

use crate::errors::CommandError;
use crate::utils::Extent;
use crate::utils::Point;
use crate::utils::GeometryGeometryIterator;
use crate::utils::create_polygon;
use crate::world_map::VoronoiSite;
use crate::world_map::TileEntitySite;
use crate::world_map::TileEntitySiteGeo;
use crate::world_map::TileEntityLatElevOcean;
use crate::world_map::TileEntityLat;
use crate::world_map::TileFeature;
use crate::world_map::TileEntityForWaterFlow;
use crate::world_map::TileEntityForWaterFill;
use crate::world_map::TileEntityWithNeighborsElevation;
use crate::progress::ProgressObserver;
use crate::raster::RasterMap;
use crate::world_map::TilesLayer;
use crate::tile_entity;
use crate::world_map::TileEntity;

enum PointGeneratorPhase {
    NortheastInfinity,
    SoutheastInfinity,
    SouthwestInfinity,
    NorthwestInfinity,
    Random(f64,f64),
    Done
}

/// FUTURE: This one would be so much easier to read if I had real Function Generators.
pub(crate) struct PointGenerator<Random: Rng> {
    random: Random,
    extent: Extent,
    spacing: f64,
    jittering: f64,
    double_jittering: f64,
    phase: PointGeneratorPhase,
    
}

impl<Random: Rng> PointGenerator<Random> {
    const START_X: f64 = 0.0;
    // You would think I'd be able to start generating at 0, but that appears to be one pixel below the bottom of the grid on my test.
    // FUTURE: Revisit this, could this have just been bad starting data?
    const START_Y: f64 = 1.0;

    pub(crate) fn new(random: Random, extent: Extent, est_point_count: usize) -> Self {
        let density = est_point_count as f64/(extent.width*extent.height); // number of points per unit square
        let unit_point_count = density.sqrt(); // number of points along a line of unit length
        let spacing = 1.0/unit_point_count; // if there are x points along a unit, then it divides it into x spaces.
        let radius = spacing / 2.0; // FUTURE: Why is this called 'radius'?
        let jittering = radius * 0.9; // FUTURE: Customizable factor?
        let double_jittering = jittering * 2.0;
        let phase = PointGeneratorPhase::NortheastInfinity;// Top(Self::INITIAL_INDEX); 

        Self {
            random,
            extent,
            spacing,
            jittering,
            double_jittering,
            phase
        }

    }

    fn estimate_points(&self) -> usize {
        ((self.extent.width/self.spacing).floor() as usize * (self.extent.height/self.spacing).floor() as usize) + 4
    }

    fn make_point(&self, x: f64, y: f64) -> Result<Geometry,CommandError> {
        let mut point = Geometry::empty(wkbPoint)?;
        point.add_point_2d((self.extent.west + x,self.extent.south + y));
        Ok(point)
    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: The points laying beyond the edge of the heightmap looks weird. Once I get to the voronoi, see if those are absolutely necessary.
        // TODO: Those boundary points should also be jittered, at least along the line.

        // Randomizing algorithms borrowed from AFMG with many modifications


        macro_rules! jitter {
            () => {
                // gen creates random number between >= 0.0, < 1.0
                self.random.gen::<f64>() * self.double_jittering - self.jittering    
            };
        }

        match &self.phase { // TODO: Do I need a reference here?
            PointGeneratorPhase::NortheastInfinity => {
                self.phase = PointGeneratorPhase::SoutheastInfinity;
                Some(self.make_point(self.extent.width*2.0, self.extent.height*2.0))
            },
            PointGeneratorPhase::SoutheastInfinity => {
                self.phase = PointGeneratorPhase::SouthwestInfinity;
                Some(self.make_point(self.extent.width*2.0, -self.extent.height))
            },
            PointGeneratorPhase::SouthwestInfinity => {
                self.phase = PointGeneratorPhase::NorthwestInfinity;
                Some(self.make_point(-self.extent.width, -self.extent.height))
            },
            PointGeneratorPhase::NorthwestInfinity => {
                self.phase = PointGeneratorPhase::Random(Self::START_X,Self::START_Y);
                Some(self.make_point(-self.extent.width, self.extent.height*2.0))
            },
            PointGeneratorPhase::Random(x, y) => if y < &self.extent.height {
                if x < &self.extent.width {
                    let x_j = (x + jitter!()).clamp(Self::START_X,self.extent.width);
                    let y_j = (y + jitter!()).clamp(Self::START_Y,self.extent.height);
                    self.phase = PointGeneratorPhase::Random(x + self.spacing, *y);
                    Some(self.make_point(x_j,y_j))
                } else {
                    self.phase = PointGeneratorPhase::Random(Self::START_X, y + self.spacing);
                    self.next()
                }
                
            } else {
                self.phase = PointGeneratorPhase::Done;
                self.next()
            },
            PointGeneratorPhase::Done => None,
        }

    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0,Some(self.estimate_points()))
    }
}

enum DelaunayGeneratorPhase {
    Unstarted(Geometry),
    Started(GeometryGeometryIterator),
    Done
}

pub(crate) struct DelaunayGenerator {
    phase: DelaunayGeneratorPhase

}

impl DelaunayGenerator {

    pub(crate) fn new(source: Geometry) -> Self {
        let phase = DelaunayGeneratorPhase::Unstarted(source);
        Self {
            phase
        }
    }

    // this function is optional to call, it will automatically be called by the iterator.
    // However, that will cause a delay to the initial return.
    pub(crate) fn start(&mut self) -> Result<(),CommandError> {
        // NOTE: the delaunay thingie can only work if all of the points are known, so we can't work with an iterator here.
        // I'm not certain if some future algorithm might allow us to return an iterator, however.
        if let DelaunayGeneratorPhase::Unstarted(source) = &mut self.phase {
            // the delaunay_triangulation procedure requires a single geometry. Which means I've got to read all the points into one thingie.
            // FUTURE: Would it be more efficient to have my own algorithm which outputs triangles as they are generated?
            let triangles = source.delaunay_triangulation(None)?; // FUTURE: Should this be configurable?
            self.phase = DelaunayGeneratorPhase::Started(GeometryGeometryIterator::new(triangles))
        }
        Ok(())
    }

}

impl Iterator for DelaunayGenerator {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.phase {
            DelaunayGeneratorPhase::Unstarted(_) => {
                match self.start() {
                    Ok(_) => self.next(),
                    Err(e) => Some(Err(e)),
                }
            },
            DelaunayGeneratorPhase::Started(iter) => if let Some(value) = iter.next() {
                Some(Ok(value))
            } else {
                self.phase = DelaunayGeneratorPhase::Done;
                None
            },
            DelaunayGeneratorPhase::Done => None,
        }
    }
}



enum VoronoiGeneratorPhase<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> {
    Unstarted(GeometryIterator),
    Started(IntoIter<Point,VoronoiInfo>)
}

pub(crate) struct VoronoiGenerator<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> {
    phase: VoronoiGeneratorPhase<GeometryIterator>,
    extent: Extent,
    extent_geo: Geometry

}

struct VoronoiInfo {
    vertices: Vec<Point>,
}

impl<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> VoronoiGenerator<GeometryIterator> {

    pub(crate) fn new(source: GeometryIterator, extent: Extent) -> Result<Self,CommandError> {
        let phase = VoronoiGeneratorPhase::Unstarted(source);
        let extent_geo = extent.create_geometry()?;
        Ok(Self {
            phase,
            extent,
            extent_geo
        })
    }

    fn circumcenter(points: (&Point,&Point,&Point)) -> Result<Point,CommandError> {
        // TODO: Test this stuff...
        // Finding the Circumcenter: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2

        let (a,b,c) = points;
        let d = (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) * 2.0;
        let d_recip = d.recip();
        let (ax2,ay2,bx2,by2,cx2,cy2) = ((a.x*a.x),(a.y*a.y),(b.x*b.x),(b.y*b.y),(c.x*c.x),(c.y*c.y));
        let (ax2_ay2,bx2_by2,cx2_cy2) = (ax2+ay2,bx2+by2,cx2+cy2);
        let ux = ((ax2_ay2)*(b.y - c.y) + (bx2_by2)*(c.y - a.y) + (cx2_cy2)*(a.y - b.y)) * d_recip;
        let uy = ((ax2_ay2)*(c.x - b.x) + (bx2_by2)*(a.x - c.x) + (cx2_cy2)*(b.x - a.x)) * d_recip;

        let u: Point = (ux,uy).try_into()?;

        Ok(u)
    
    }

    fn sort_clockwise(center: &Point, points: &mut Vec<Point>, extent: &Extent, needs_a_trim: &mut bool)  {
        // TODO: Test this stuff...
        // Sort the points clockwise to create a polygon: https://stackoverflow.com/a/6989383/300213
        // The "beginning" of this ordering is north, so the "lowest" point will be the one closest to north in the northeast quadrant.
        // when angle is equal, the point closer to the center will be lesser.

        let zero: NotNan<f64> = 0.0.try_into().unwrap(); // there shouldn't be any error here.

        points.sort_by(|a: &Point, b: &Point| -> Ordering
        {
            if !*needs_a_trim {
                *needs_a_trim = (!extent.contains(a)) || (!extent.contains(b))
            }
            let a_run = a.x - center.x;
            let b_run = b.x - center.x;

            match (a_run >= zero,b_run >= zero) {
                (true, false) => {
                    // a is in the east, b is in the west. a is closer to north and less than b.
                    Ordering::Less
                },
                (false, true) => {
                    // a is in the west, b is in the east, a is further from north and greater than b.
                    Ordering::Greater
                },
                (east, _) => { // both are in the same east-west half
                    let a_rise = a.y - center.y;
                    let b_rise = b.y - center.y;

                    match (a_rise >= zero,b_rise >= zero) {
                        (true, false) => {
                            // a is in the north and b is in the south
                            if east {
                                // a is in northeast and b is in southeast
                                Ordering::Less
                            } else {
                                // a is in northwest and b is in southwest
                                Ordering::Greater
                            }
                        },
                        (false, true) => {
                            // a is in the south and b is in the north, a is further from north
                            if east {
                                // a is in the southeast and b is in the northeast
                                Ordering::Greater
                            } else {
                                // a is in southwest and b is in northwest
                                Ordering::Less
                            }
                        },
                        (_, _) => {
                            // both are in the same quadrant. Compare the cross-product.
                            // NOTE: I originally compared the slope, but the stackoverflow accepted solution used something like the following formula 
                            // and called it a cross-product. Assuming that is the same, it's the same thing as comparing the slope. Why?
                            // (Yes, I know a mathematician might not need this, but I'll explain my reasoning to future me)
                            // A slope is a fraction. To compare two fractions, you have to do the same thing that you do when adding fractions:
                            //   A/B cmp C/D = (A*D)/(B*D) cmp (C*B)/(B*D)
                            // For a comparison, we can then remove the denominators:
                            //   (A*D) cmp (B*D) 
                            // and that's the same thing the solution called a cross-product. 
                            // So, in order to avoid a divide by 0, I'm going to use that instead of slope.
                            match ((a_run) * (b_rise)).cmp(&((b_run) * (a_rise))) {
                                Ordering::Equal => {
                                    // The slopes are the same, compare the distance from center. The shorter distance should be closer to the beginning.
                                    let a_distance = (a_run) * (a_run) + (a_rise) * (a_rise);
                                    let b_distance = (b_run) * (b_run) + (b_rise) * (b_rise);
                                    a_distance.cmp(&b_distance)
                                },
                                a => {
                                    // both are in the same quadrant now, but the slopes are not the same, we can just return the result of slope comparison:
                                    // in the northeast quadrant, a lower positive slope means it is closer to east and further away.
                                    // in the southeast quadrant, a lower negative slope means it is closer to south and further away.
                                    // in the southwest quadrant, a lower positive slope means it is closer to west and further away.
                                    // in the northwest quadrant, a lower negative slope means it is closer to north and further away from the start.
                                    a
                                }
                            }

                        },
                    }
        

                },
            }

        });
        

    }

    fn create_voronoi(site: Point, voronoi: VoronoiInfo, extent: &Extent, extent_geo: &Geometry) -> Result<Option<VoronoiSite>,CommandError> {
        if (voronoi.vertices.len() >= 3) && extent.contains(&site) {
            // * if there are less than 3 vertices, its either a line or a point, not even a sliver.
            // * if the site is not contained in the extent, it's one of our infinity points created to make it easier for us
            // to clip the edges.
            let mut vertices = voronoi.vertices;
            // sort the vertices clockwise to make sure it's a real polygon.
            let mut needs_a_trim = false;
            Self::sort_clockwise(&site,&mut vertices,extent,&mut needs_a_trim);
            vertices.push(vertices[0].clone());
            let polygon = create_polygon(vertices)?;
            let polygon = if needs_a_trim {
                // intersection code is not trivial, just let someone else do it.
                polygon.intersection(extent_geo)
            } else {
                Some(polygon)
            };
            Ok(polygon.map(|a| VoronoiSite::new(a,site)))
        } else {
            // In any case, these would result in either a line or a point, not even a sliver.
            Ok(None)
        }

    }

    fn generate_voronoi(source: &mut GeometryIterator) -> Result<IntoIter<Point,VoronoiInfo>,CommandError> {

        // Calculate a map of sites with a list of triangle circumcenters
        let mut sites: HashMap<Point, VoronoiInfo> = HashMap::new(); // site, voronoi info

        for geometry in source {
            let geometry = geometry?;
            
            if geometry.geometry_type() != wkbPolygon {
                return Err(CommandError::VoronoiExpectsPolygons)
            }
            
            let line = geometry.get_geometry(0); // this should be the outer ring for a triangle.

            if line.point_count() != 4 { // the line should be a loop, with the first and last elements
                return Err(CommandError::VoronoiExpectsTriangles);
            }

            let points: [Point; 3] = (0..3)
               .map(|i| Ok(line.get_point(i).try_into()?)).collect::<Result<Vec<Point>,CommandError>>()?
               .try_into()
               .map_err(|_| CommandError::VoronoiExpectsTriangles)?;

            let circumcenter = Self::circumcenter((&points[0],&points[1],&points[2]))?;

            // collect a list of neighboring circumcenters for each site.
            for point in points {

                match sites.entry(point) {
                    Entry::Occupied(mut entry) => {
                        let entry = entry.get_mut();
                        entry.vertices.push(circumcenter.clone());
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(VoronoiInfo {
                            vertices: vec![circumcenter.clone()]
                        });
                    },
                }

            }

        }

        Ok(sites.into_iter())

    }

    // this function is optional to call, it will automatically be called by the iterator.
    // However, that will cause a delay to the initial return.
    pub(crate) fn start(&mut self) -> Result<(),CommandError> {
        // NOTE: the delaunay thingie can only work if all of the points are known, so we can't work with an iterator here.
        // I'm not certain if some future algorithm might allow us to return an iterator, however.

        if let VoronoiGeneratorPhase::Unstarted(source) = &mut self.phase {
            // the delaunay_triangulation procedure requires a single geometry. Which means I've got to read all the points into one thingie.
            // FUTURE: Would it be more efficient to have my own algorithm which outputs triangles as they are generated?
            let voronoi = Self::generate_voronoi(source)?; // FUTURE: Should this be configurable?
            self.phase = VoronoiGeneratorPhase::Started(voronoi.into_iter())
        }
        Ok(())
    }

}

impl<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> Iterator for VoronoiGenerator<GeometryIterator> {

    type Item = Result<VoronoiSite,CommandError>; // TODO: Should be a voronoi struct defined in world_map.

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.phase {
            VoronoiGeneratorPhase::Unstarted(_) => {
                match self.start() {
                    Ok(_) => self.next(),
                    Err(e) => Some(Err(e)),
                }
            },
            VoronoiGeneratorPhase::Started(iter) => {
                let mut result = None;
                while let Some(value) = iter.next() {
                    // create_voronoi returns none for various reasons if the polygon shouldn't be written. 
                    // If it does that, I have to keep trying. 
                    result = Self::create_voronoi(value.0, value.1,&self.extent,&self.extent_geo).transpose();
                    if let Some(_) = result {
                        break;
                    }
                }
                result
            }
        }
    }
}



pub(crate) fn sample_elevations<Progress: ProgressObserver>(layer: &mut TilesLayer, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

    progress.start_unknown_endpoint(|| "Reading raster");

    let (min_elevation,max_elevation) = raster.compute_min_max(1,true)?;
    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;

    let positive_elevation_scale = 80.0/max_elevation;
    let negative_elevation_scale = 20.0/min_elevation.abs();

//    * find the max_elevation from the raster, if possible
//    * find the absolute value of the min_elevation from the raster, if possible
//    * if elevation >= 0
//      * elevation_scaled = (elevation*80)/max_elevation
//    * else
//      * elevation_scaled = 20 - (elevation.abs()*20)/min_elevation.abs()
    

    progress.finish(|| "Raster read.");

    let features = layer.read_entities_to_vec::<_,TileEntitySite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling elevations.",features.len()));

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(elevation) = band.get_value(x, y) {

            if let Some(mut feature) = layer.feature_by_id(&feature.fid) {

                let elevation_scaled = if elevation >= &0.0 {
                    20 + (elevation * positive_elevation_scale).floor() as i32
                } else {
                    20 - (elevation.abs() * negative_elevation_scale).floor() as i32
                };

                feature.set_elevation(*elevation)?;
                feature.set_elevation_scaled(elevation_scaled)?;

                layer.update_feature(feature)?;

            }

        }



        progress.update(|| i);




    }

    progress.finish(|| "Elevation sampled.");

    Ok(())
}


pub(crate) enum OceanSamplingMethod {
    Below(f64), // any elevation below the specified value is ocean
    AllData, // any elevation that is not nodata is ocean
    NoData, // any elevation that is nodata is ocean
    NoDataAndBelow(f64), // any elevation that is no data or below the specified value is ocean.
    // TODO: Another option: a list of points to act as seeds, along with an elevation, use a flood-fill to mark oceans that are connected to these and under that elevation.
}

pub(crate) fn sample_ocean<Progress: ProgressObserver>(layer: &mut TilesLayer, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

    progress.start_unknown_endpoint(|| "Reading raster");

    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;
    let no_data_value = band.no_data_value();

    progress.finish(|| "Raster read.");

    let features = layer.read_entities_to_vec::<_,TileEntitySite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling oceans.",features.len()));

    let mut bad_ocean_tile_found = false;

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(mut feature) = layer.feature_by_id(&feature.fid) {

            let is_ocean = if let Some(elevation) = band.get_value(x, y) {
                let is_no_data = match no_data_value {
                    Some(no_data_value) if no_data_value.is_nan() => elevation.is_nan(),
                    Some(no_data_value) => elevation == no_data_value,
                    None => false,
                };

                match method {
                    OceanSamplingMethod::Below(_) if is_no_data => false,
                    OceanSamplingMethod::Below(below) => elevation < &below,
                    OceanSamplingMethod::AllData => !is_no_data,
                    OceanSamplingMethod::NoData => is_no_data,
                    OceanSamplingMethod::NoDataAndBelow(below) => is_no_data || (elevation < &below),
                }

            } else {

                match method {
                    OceanSamplingMethod::Below(_) => false,
                    OceanSamplingMethod::AllData => false,
                    OceanSamplingMethod::NoData => true,
                    OceanSamplingMethod::NoDataAndBelow(_) => true,
                }

            };

            if let Some(elevation) = feature.elevation()? {
                if is_ocean && (elevation > 0.0) {
                    bad_ocean_tile_found = true;
                }

            }

            feature.set_is_ocean(is_ocean)?;

            layer.update_feature(feature)?;

        }


        progress.update(|| i);




    }

    progress.finish(|| "Oceans sampled.");

    if bad_ocean_tile_found {
        println!("At least one ocean tile was found with an elevation above 0.")

    }

    Ok(())
}


pub(crate) fn calculate_neighbors<Progress: ProgressObserver>(layer: &mut TilesLayer, progress: &mut Progress) -> Result<(),CommandError> {

    let features = layer.read_entities_to_vec::<_,TileEntitySiteGeo>(progress)?;

    progress.start_known_endpoint(|| ("Calculating neighbors.",features.len()));

    // # Loop through all features and find features that touch each feature
    // for f in feature_dict.values():
    for (i,feature) in features.iter().enumerate() {

        let working_fid = feature.fid;
        let working_geometry = &feature.geometry;

        let envelope = working_geometry.envelope();
        layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);


        let mut neighbors = Vec::new();

        for intersecting_feature in layer.read_features() {

            if let Some(intersecting_fid) = intersecting_feature.fid() {
                if (working_fid != intersecting_fid) && (!intersecting_feature.geometry().unwrap().disjoint(&working_geometry)) {

                    let neighbor_site_x = intersecting_feature.site_x()?;
                    let neighbor_site_y = intersecting_feature.site_y()?;
                    let neighbor_angle = if let (site_x,site_y,Some(neighbor_site_x),Some(neighbor_site_y)) = (feature.site_x,feature.site_y,neighbor_site_x,neighbor_site_y) {
                        // needs to be clockwise, from the north, with a value from 0..360
                        // the result below is counter clockwise from the east, but also if it's in the south it's negative.
                        let counter_clockwise_from_east = ((neighbor_site_y-site_y).atan2(neighbor_site_x-site_x).to_degrees()).round();
                        // 360 - theta would convert the direction from counter clockwise to clockwise. Adding 90 shifts the origin to north.
                        let clockwise_from_north = 450.0 - counter_clockwise_from_east; 
                        // And then, to get the values in the range from 0..360, mod it.
                        let clamped = clockwise_from_north % 360.0;
                        clamped
                    } else {
                        // in the off chance that we actually are missing data, this marks an appropriate angle.
                        -360.0 
                    };
                
                    neighbors.push((intersecting_fid,neighbor_angle.floor() as i32)) 
                }

            }

        }
        
        layer.clear_spatial_filter();

        if let Some(mut working_feature) = layer.feature_by_id(&working_fid) {
            working_feature.set_neighbors(&neighbors)?;

            layer.update_feature(working_feature)?;

        }


        progress.update(|| i);

    }

    progress.finish(|| "Neighbors calculated.");

    Ok(())
}


pub(crate) fn generate_temperatures<Progress: ProgressObserver>(layer: &mut TilesLayer, equator_temp: i8, polar_temp: i8, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications

    let equator_temp = equator_temp as f64;
    let polar_temp = polar_temp as f64;
    let temp_delta = equator_temp - polar_temp;
    const EXPONENT: f64 = 0.5;

    fn interpolate(t: f64) -> f64 { // TODO: Test this somehow...
        // From AFMG/d3: `t` is supposed to be a value from 0 to 1. If t <= 0.5 (`(t *= 2) <= 1`) then the function above is `y = ((2x)^(1/2))/2`. If t is greater, then the function is `y = (2 - (2-x)^(1/2))/2`. The two functions both create a sort of parabola. The first one starts curving up steep at 0 (the pole) and then flattens out to almost diagonal at 0.5. The second one continues the diagonal that curves more steeply up towards 1 (the equator). I'm not sure whey this curve was chosen, I would have expected a flatter curve at the equator.
        let t = t * 2.0;
        (if t <= 1.0 {
            t.powf(EXPONENT)
        } else {
            2.0 - (2.0-t).powf(EXPONENT)
        })/2.0
    }

    let features = layer.read_entities_to_vec::<_,TileEntityLatElevOcean>(progress)?;

    progress.start_known_endpoint(|| ("Generating temperatures.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let base_temp = equator_temp - (interpolate(feature.site_y.abs()/90.0) * temp_delta);
        let adabiatic_temp = base_temp - if !feature.is_ocean {
            (feature.elevation/1000.0)*6.5
        } else {
            0.0
        };
        let temp = (adabiatic_temp*100.0).round()/100.0;

        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_temperature(temp)?;

            layer.update_feature(working_feature)?;

        }



        progress.update(|| i);


    }

    progress.finish(|| "Temperatures calculated.");

    Ok(())
}



pub(crate) fn generate_winds<Progress: ProgressObserver>(layer: &mut TilesLayer, winds: [i32; 6], progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications


    let features = layer.read_entities_to_vec::<_,TileEntityLat>(progress)?;

    progress.start_known_endpoint(|| ("Generating winds.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let wind_tier = ((feature.site_y - 89.0)/30.0).abs().floor() as usize;
        let wind_dir = winds[wind_tier];

        if let Some(mut working_feature) = layer.feature_by_id(&feature.fid) {
            working_feature.set_wind(wind_dir)?;

            layer.update_feature(working_feature)?;

        }


        progress.update(|| i);


    }

    progress.finish(|| "Winds generated.");

    Ok(())
}



pub(crate) fn generate_precipitation<Progress: ProgressObserver>(layer: &mut TilesLayer, moisture: u16, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications, most importantly I don't have a grid, so I follow the paths of the wind to neighbors.

    const MAX_PASSABLE_ELEVATION: i32 = 85; // FUTURE: I've found that this is unnecessary, the elevation change should drop the precipitation and prevent any from passing on. 

    // Bands of rain at different latitudes, like the ITCZ
    const LATITUDE_MODIFIERS: [f64; 18] = [4.0, 2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 2.0, 2.0, 1.0, 1.0, 1.0, 0.5];

    // I believe what this does is scale the moisture scale correctly to the size of the map. Otherwise, I don't know.
    let cells_number_modifier = (layer.feature_count() as f64 / 10000.0).powf(0.25);
    let prec_input_modifier = moisture as f64/100.0;
    let modifier = cells_number_modifier * prec_input_modifier;

    tile_entity!(TileDataForPrecipitation 
        elevation_scaled: i32, 
        wind: i32, 
        is_ocean: bool, 
        neighbors: Vec<(u64,i32)>,
        temperature: f64,
        precipitation: f64 = |_| {
            Ok::<_,CommandError>(0.0)
        },
        lat_modifier: f64 = |feature: &TileFeature| {
            let site_y = tile_entity!(fieldassign@ feature site_y f64);
            let lat_band = ((site_y.abs() - 1.0) / 5.0).floor() as usize;
            let lat_modifier = LATITUDE_MODIFIERS[lat_band];
            Ok::<_,CommandError>(lat_modifier)
        }
    );

    let mut tile_map = layer.read_entities_to_index::<_,TileDataForPrecipitation>(progress)?;

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_tiles: Vec<u64> = tile_map.keys().copied().collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_tiles.sort();
    let working_tiles = working_tiles;

    progress.start_known_endpoint(|| ("Tracing winds.",working_tiles.len()));

    for (i,start_fid) in working_tiles.iter().enumerate() {
        if let Some(tile) = tile_map.get(start_fid).cloned() {

            let max_prec = 120.0 * tile.lat_modifier;
            let mut humidity = max_prec - tile.elevation_scaled as f64;

            let mut current = tile;
            let mut current_fid = *start_fid;
            let mut visited = vec![current_fid];

            loop {
                if humidity < 0.0 {
                    // there is no humidity left to work with.
                    break;
                }

                // TODO: I wonder if I should be sending the precipitation to all tiles in the general direction of the wind, not just
                // one. That changes this algorithm a lot, though.

                // find neighbor closest to wind direction
                let mut best_neighbor: Option<(_,_)> = None;
                for (fid,direction) in &current.neighbors {
                    // calculate angle difference
                    let angle_diff = (direction - current.wind).abs();
                    let angle_diff = if angle_diff > 180 {
                        360 - angle_diff
                    } else {
                        angle_diff
                    };
                    
                    // if the angle difference is greater than 45, it's not going the right way, so don't even bother with this one.
                    if angle_diff < 45 {
                        if let Some(better_neighbor) = best_neighbor {
                            if better_neighbor.1 > angle_diff {
                                best_neighbor = Some((*fid,angle_diff));
                            }

                        } else {
                            best_neighbor = Some((*fid,angle_diff));
                        }
    
                    }

                }

                let next = if let Some((next_fid,_)) = best_neighbor {
                    if visited.contains(&next_fid) {
                        // we've reached one we've already visited, I don't want to go in circles.
                        None
                    } else {
                        // visit it so we don't do this one again.
                        visited.push(next_fid);
                        tile_map.get(&next_fid).map(|tile| (next_fid,tile.clone()))
                    }

                } else {
                    None
                };

                if let Some((next_fid,mut next)) = next {
                    if current.temperature >= -5.0 { // no humidity change across permafrost? FUTURE: I'm not sure this is right. There should still be precipitation in the cold, and if there's a bunch of humidity it should all precipitate in the first cell, shouldn't it?
                        if current.is_ocean {
                            if !next.is_ocean {
                                // coastal precipitation
                                // FUTURE: The AFMG code uses a random number between 10 and 20 instead of 15. I didn't feel like this was
                                // necessary considering it's the only randomness I would use, and nothing else is randomized.
                                next.precipitation += (humidity / 15.0).max(1.0);
                            } else {
                                // add more humidity
                                humidity = (humidity + 5.0 * current.lat_modifier).max(max_prec);
                                // precipitation over water cells
                                current.precipitation += 5.0 * modifier;
                            }
                        } else {
                            let is_passable = next.elevation_scaled < MAX_PASSABLE_ELEVATION;
                            let precipitation = if is_passable {
                                // precipitation under normal conditions
                                let normal_loss = (humidity / (10.0 * current.lat_modifier)).max(1.0);
                                // difference in height
                                let diff = (next.elevation_scaled - current.elevation_scaled).max(0) as f64;
                                // additional modifier for high elevation of mountains
                                let modifier = (next.elevation_scaled / 70).pow(2) as f64;
                                (normal_loss + diff + modifier).clamp(1.0,humidity.max(1.0))
                            } else {
                                humidity
                            };
                            current.precipitation = precipitation;
                            // sometimes precipitation evaporates
                            humidity = if is_passable {
                                // FUTURE: I feel like this evaporation was supposed to be a multiplier not an addition. Not much is evaporating.
                                // FUTURE: Shouldn't it also depend on temperature?
                                let evaporation = if precipitation > 1.5 { 1.0 } else { 0.0 };
                                (humidity - precipitation + evaporation).clamp(0.0,max_prec)
                            } else {
                                0.0
                            };
    
                        }
    
                        if let Some(real_current) = tile_map.get_mut(&current_fid) {
                            real_current.precipitation = current.precipitation;
                        }
    
                        if let Some(real_next) = tile_map.get_mut(&next_fid) {
                            real_next.precipitation = next.precipitation;
                        }
    
                    }

                    (current_fid,current) = (next_fid,next);
                } else {
                    if current.is_ocean {
                        // precipitation over water cells
                        current.precipitation += 5.0 * modifier;
                    } else {
                        current.precipitation = humidity;
                    }

                    if let Some(real_current) = tile_map.get_mut(&current_fid) {
                        real_current.precipitation = current.precipitation;
                    }

                    break;

                }
            }
            
        }

        progress.update(|| i);

    }

    progress.finish(|| "Winds traced.");

    progress.start_known_endpoint(|| ("Writing precipitation",tile_map.len()));

    for (fid,tile) in tile_map {
        if let Some(mut working_feature) = layer.feature_by_id(&fid) {

            working_feature.set_precipitation(tile.precipitation)?;

            layer.update_feature(working_feature)?;
        }


    }

    progress.finish(|| "Precipitation written.");

    Ok(())
}

fn find_lowest_neighbors<Data: TileEntityWithNeighborsElevation>(entity: &Data, tile_map: &HashMap<u64,Data>) -> (Vec<u64>, Option<f64>) {
    let mut lowest = Vec::new();
    let mut lowest_elevation = None;

    // find the lowest neighbors
    for (neighbor_fid,_) in entity.neighbors() {
        if let Some(neighbor) = tile_map.get(&neighbor_fid) {
            let neighbor_elevation = neighbor.elevation();
            if let Some(lowest_elevation) = lowest_elevation.as_mut() {
                if neighbor_elevation < *lowest_elevation {
                    *lowest_elevation = neighbor_elevation;
                    lowest = vec![*neighbor_fid];
                } else if neighbor_elevation == *lowest_elevation {
                    lowest.push(*neighbor_fid)
                }
            } else {
                lowest_elevation = Some(neighbor_elevation);
                lowest.push(*neighbor_fid)
            }

        }

    }
    (lowest,lowest_elevation.copied())

}

pub(crate) fn generate_water_flow<Progress: ProgressObserver>(layer: &mut TilesLayer, progress: &mut Progress) -> Result<(HashMap<u64,TileEntityForWaterFill>,Vec<u64>),CommandError> {


    // from the AFMG code, this is also done in calculating precipitation. I'm wondering if it's unscaling the precipitation somehow?
    let cells_number_modifier = ((layer.feature_count() / 10000) as f64).powf(0.25);

    let mut tile_map = HashMap::new();
    let mut tile_list = Vec::new();
    let mut lake_queue = Vec::new();

    progress.start_known_endpoint(|| ("Indexing data.",layer.feature_count() as usize));

    for (i,data) in layer.read_entities::<TileEntityForWaterFlow>().enumerate() {
        let (fid,entity) = data?;
        if !entity.is_ocean {
            // pushing the elevation onto here is easier than trying to map out the elevation during the sort, 
            // FUTURE: Although it takes about twice as much memory, which could be important in the future.
            tile_list.push((fid,entity.elevation));
        }
        tile_map.insert(fid, entity);
        progress.update(|| i);

    }
    progress.finish(|| "Data indexed.");
    
    // sort tile list so the highest is first.
    tile_list.sort_by(|(_,a),(_,b)| 
        if a > b {
            Ordering::Less
        } else if a < b {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    );

    progress.start_known_endpoint(|| ("Calculating initial flow",tile_list.len()));
    
    for (i,(fid,elevation)) in tile_list.iter().enumerate() {
        let (water_flow,lowest,lowest_elevation) = if let Some(entity) = tile_map.get(fid) {
            let water_flow = entity.water_flow + entity.precipitation / cells_number_modifier;
            let (lowest,lowest_elevation) = find_lowest_neighbors(entity,&tile_map);

            (water_flow,lowest,lowest_elevation)

        } else {
            (0.0,vec![],None)
        };

        let (water_accumulation,flow_to) = if let Some(lowest_elevation) = lowest_elevation {

            if &lowest_elevation < elevation {
                let neighbor_flow = water_flow/lowest.len() as f64;
                //println!("flowing {} to {} neighbors",neighbor_flow,lowest.len());
                for neighbor in &lowest {
                    if let Some(neighbor) = tile_map.get_mut(&neighbor) {
                        neighbor.water_flow += neighbor_flow;
                    }
                }
                (0.0,lowest)
            } else {
                lake_queue.push(*fid);
                (water_flow,Vec::new())
            }

        } else { // else there are... no neighbors? for some reason? I'm not going to start a lake, though.
            (water_flow,Vec::new())
        };

        if let Some(tile) = tile_map.get_mut(&fid) {
            tile.water_flow = water_flow;
            tile.water_accumulation += water_accumulation;
            tile.flow_to = flow_to;
        }

        progress.update(|| i);

    }

    progress.finish(|| "Flow calculated.");

    progress.start_known_endpoint(|| ("Writing flow",tile_map.len()));

    for (fid,tile) in &tile_map {
        if let Some(mut working_feature) = layer.feature_by_id(&fid) {

            working_feature.set_water_flow(tile.water_flow)?;
            working_feature.set_water_accumulation(tile.water_accumulation)?;
            working_feature.set_flow_to(&tile.flow_to)?;

            layer.update_feature(working_feature)?;
        }


    }

    progress.finish(|| "Flow written.");

    Ok((tile_map.into_iter().map(|(k,v)| (k,v.into())).collect(),lake_queue))





}



// this one is quite tight with generate_water_flow, it even shares some pre-initialized data.
pub(crate) fn generate_water_fill<Progress: ProgressObserver>(layer: &mut TilesLayer, tile_map: HashMap<u64,TileEntityForWaterFill>, tile_queue: Vec<(u64,f64)>, progress: &mut Progress) -> Result<(),CommandError> {

    struct Lake {
        elevation: f64,
        spillover_elevation: f64,
        contained_tiles: Vec<u64>,
        shoreline_tiles: Vec<u64>,
        outlet_tiles: Vec<u64>
    }

    enum Task {
        FillLake(usize, f64),
        AddToFlow(f64)
    }

    let mut tile_queue = tile_queue;
    let mut tile_map = tile_map;
    let mut next_lake_id = (0..).into_iter();
    let mut lake_map = HashMap::new();

    println!("tile_queue {}",tile_queue.len());

    progress.start_unknown_endpoint(|| "Filling lakes.");

    while let Some((tile_fid,accumulation)) = tile_queue.pop() {
        println!("tile_queue pop {}",tile_fid);

        // figure out what we've got to do.
        let task = if let Some(tile) = tile_map.get(&tile_fid) {

            // we don't bother with accumulation in ocean.
            if tile.is_ocean {
                continue;
            }

            // if the tile has no accumulation, there's nothing to do:
            if accumulation <= 0.0 {
                continue;
            }

            // look for an existing lake
            if let Some(lake_id) = tile.lake_id {
                // we're already in a lake, so the accumulation is intended to fill it.
                Task::FillLake(lake_id, accumulation)
            } else {
                // there is no lake here, so this is a flow task, unless it turns out we need a lake here.
                // we already calculated the lowest neighbors that are actually below the tile in Flow, so let's just check that first.

                let flow_to = &tile.flow_to;
                if flow_to.len() > 0 {
                    // we've got tiles that are lowever in elevation to go to...
                    let neighbor_flow = accumulation/flow_to.len() as f64;

                    for neighbor_fid in flow_to {
                        // add a task to the queue to flow this down.
                        tile_queue.push((*neighbor_fid,neighbor_flow));
                        println!("tile_queue.push (flow_to) {}",neighbor_fid);
                    }
                    // and the task for this one is to add to the flow:
                    Task::AddToFlow(accumulation)
                } else {
                    // we need to recalculate to find the lowest neighbors that we can assume are above:
                    let (_,lowest_elevation) = find_lowest_neighbors(tile,&tile_map);

                    // assuming that succeeded, we can create a new lake now.
                    if let Some(lowest_elevation) = lowest_elevation {
                        // we need to be in a lake, so create a new one.
                        let lake_id = next_lake_id.next().unwrap(); // it should be an infinite iterator, so it should always return Some.

                        let new_lake = Lake {
                            elevation: tile.elevation,
                            spillover_elevation: lowest_elevation,
                            contained_tiles: vec![tile_fid],
                            shoreline_tiles: tile.neighbors.iter().map(|(a,_)| *a).collect(),
                            outlet_tiles: Vec::new(),
                        };

                        lake_map.insert(lake_id, new_lake);
                        Task::FillLake(lake_id,accumulation) // I just inserted it, it should exist here.
    
                    } else {
                        // this is a tile with no neighbors, which should be impossible. but there is nothing I can do.
                        continue;
                    }
    

                }
                

    
            }

        } else {
            continue;
        };

        match task {
            Task::AddToFlow(accumulation) => {
                if let Some(tile) = tile_map.get_mut(&tile_fid) {
                    tile.water_flow += accumulation;
                    if let Some(mut feature) = layer.feature_by_id(&tile_fid) {

                        feature.set_water_flow(tile.water_flow)?;

                        layer.update_feature(feature)?;
                    }
                }

            }
            Task::FillLake(lake_id,accumulation) => {
                let (new_lake,accumulation) = if let Some(lake) = lake_map.get(&lake_id) {
                    let outlet_tiles = &lake.outlet_tiles;
                    if outlet_tiles.len() > 0 {
                        // we can automatically flow to those tiles.
                        let neighbor_flow = accumulation/outlet_tiles.len() as f64;
    
                        for neighbor_fid in outlet_tiles {
                            // add a task to the queue to flow this down.
                            tile_queue.push((*neighbor_fid,neighbor_flow));
                            println!("tile_queue.push (first outlet) {}",neighbor_fid);
                        }
                        continue;
    
                    } else {
                        // no outlet tiles, so we have to grow the lake.
    
                        let accumulation_per_tile = accumulation/lake.contained_tiles.len() as f64;
                        let spillover_difference = lake.spillover_elevation - lake.elevation;
                        let lake_increase = accumulation_per_tile.min(spillover_difference);
                        let new_lake_elevation = lake.elevation + lake_increase;
                        let remaining_accum_per_tile = accumulation_per_tile - lake_increase;
                        let accumulation = remaining_accum_per_tile * lake.contained_tiles.len() as f64;

                        if remaining_accum_per_tile > 0.0 {
                            // we need to increase the size of the lake. Right now, we are at the spillover level.
                            // Basically, pretend that we are making the lake deeper by 0.0001 (or some small amount)
                            // and walk the shoreline and beyond looking for:
                            // * tiles that are in a lake already:
                            //   * if the lake elevation is between this lake elevation and the test elevation, we need to "swallow" the lake.
                            //   * if the lake is shorter than this lake's elevation, then this is the same as if the tile were a lower shoreline.
                            // * tiles that are between the lake elevation and this test elevation (new part of a lake, and keep walking it's neighbors)
                            // * tiles that are taller than than the test elevation:
                            // * tiles that are shorter than the lake elevation (since lake elevation is at spillover, this means we're starting to go downhill again, so this is a new outlet and new shoreline, as above, we'll also add some flow to this eventually)

                            let test_elevation = new_lake_elevation + 0.001;
                            let mut walk_queue = lake.shoreline_tiles.clone();
                            let mut new_shoreline = Vec::new();
                            let mut new_outlets = Vec::new();
                            let mut new_contained_tiles = lake.contained_tiles.clone();
                            let mut checked_tiles: HashSet<u64> = HashSet::from_iter(new_contained_tiles.iter().copied());
                            let mut new_spillover_elevation = None;

                            while let Some(check_fid) = walk_queue.pop() {
                                if checked_tiles.contains(&check_fid) {
                                    continue;
                                }
                                checked_tiles.insert(check_fid);

                                if let Some(check) = tile_map.get(&check_fid) {

                                    if let Some(lake_id) = check.lake_id {
                                        // it's in a lake already...
                                        if let Some(other_lake) = lake_map.get(&lake_id) {
                                            if (other_lake.elevation <= test_elevation) && (other_lake.elevation >= new_lake_elevation) {
                                                // merge the other one into this one.
                                                // it's contained tiles become part of this one
                                                new_contained_tiles.extend(other_lake.contained_tiles.iter());
                                                // plus, we've already checked them.
                                                checked_tiles.extend(other_lake.contained_tiles.iter());
                                                // add it's shoreline to the check queue
                                                walk_queue.extend(other_lake.shoreline_tiles.iter());
                                                // TODO: Don't worry about deleting the old lake, just make sure we update the lake id for the new lake on the
                                                // tiles, and make sure we check those when updating the lake data.
                                            } else {
                                                // otherwise, add this as an outlet. (I'm assuming that the lake is lower in elevation, I'm not sure how else we could have reached it)
                                                new_outlets.push(check_fid);
                                                new_shoreline.push(check_fid);
                                            }

                                        } else {
                                            // TODO: Is this an error?
                                            continue;
                                        }
                                    } else {
                                        // it's not in a lake, just check it's elevation
                                        if check.elevation > test_elevation {
                                            // it's too high to flood. This is part of the shoreline.
                                            new_shoreline.push(check_fid);
                                            // And this might change our spillover elevation
                                            new_spillover_elevation = new_spillover_elevation.map(|e: f64| e.min(check.elevation)).or_else(|| Some(check.elevation));
                                        } else if check.elevation < new_lake_elevation {
                                            // it's below the original spillover, which means it's beyond our initial shoreline. This is an outlet.
                                            new_outlets.push(check_fid);
                                            new_shoreline.push(check_fid);
                                        } else {
                                            // it's floodable.
                                            new_contained_tiles.push(check_fid);
                                            walk_queue.extend(check.neighbors.iter().map(|(id,_)| id));

                                        }
                                    }

                                } else {
                                    continue;
                                }

                            }

                            (Lake {
                                elevation: new_lake_elevation,
                                spillover_elevation: new_spillover_elevation.unwrap_or_else(|| new_lake_elevation),
                                contained_tiles: new_contained_tiles,
                                shoreline_tiles: new_shoreline,
                                outlet_tiles: new_outlets,
                            },accumulation)

                        
                        } else {
                            (Lake {
                                elevation: new_lake_elevation,
                                spillover_elevation: lake.spillover_elevation,
                                contained_tiles: lake.contained_tiles.clone(),
                                shoreline_tiles: lake.shoreline_tiles.clone(),
                                outlet_tiles: lake.outlet_tiles.clone(),
                            },accumulation)
                        }
    
                    }
    
                } else {
                    continue;
                };

                // update the new lake.
                for tile in &new_lake.contained_tiles {
                    if let Some(tile) = tile_map.get_mut(&tile) {
                        tile.lake_id = Some(lake_id);
                    }
                }

                if accumulation > 0.0 { // we're still not done we have to do something with the remaining water.
                    let outlet_tiles = &new_lake.outlet_tiles;
                    if outlet_tiles.len() > 0 {
                        // this is the same as above, but with the new lake.
                        // we can automatically flow to those tiles.
                        let neighbor_flow = accumulation/outlet_tiles.len() as f64;
    
                        for neighbor_fid in outlet_tiles {
                            // add a task to the queue to flow this down.
                            tile_queue.push((*neighbor_fid,neighbor_flow));
                            println!("tile_queue.push (second outlet) {}",neighbor_fid);
                        }
                    } else {
                        // add this task back to the queue so it can try to flood the lake to the next spillover.
                        tile_queue.push((tile_fid,accumulation));
                        println!("tile_queue.push (remaining accum) {}",tile_fid);

                    }

                }

                // replace it in the map.
                lake_map.insert(lake_id, new_lake);
            },
            
        }

    }

   broken!("I've got an infinite loop jumping back and forth between 4062 and 1210. Figure out what's going on with that. Both are being marked as outlets.");

    progress.finish(|| "Lakes filled.");

    for lake in lake_map.values() {
        for tile in &lake.contained_tiles {
            if let Some(mut feature) = layer.feature_by_id(&tile) {

                feature.set_lake_elevation(lake.elevation)?;

                layer.update_feature(feature)?;
            }
    
        }
    }

    // TODO: Last thing is to take the "lakes" and update all contained tiles with their lake_elevation in the actual layers. Any neighboring tiles with 
    // the same lake elevation are considered part of the same lake.

    /*

  TODO: From here...
  * we are now working with the lake.
  * if that lake has outlet tiles:
    * divide the accumulation between the outlet tiles and add them to the queue
    * set the accumulation for this tile to 0.
  * if that lake does not have any outlet tiles
    * distributed_accumulation = accumulation / number of contained tiles
    * spillover_difference = spillover - lake elevation
    * lake_increase = distributed_accumulation.min(spillover_difference)
    * new lake elevation = lake elevation + lake_increase
    * distributed_accumulation -= new_lake_elevation
    * new accumulation = distributed accumulation * lake.tile.count.
    * if distributed_accumulation > 0:
        * use a flood-fill type algorithm to see what would happen if we raised the lake by some small amount (such as 0.001):
            * set test_elevation to that new_lake_elevation + the small amount
            * create a queue out of the lake's shoreline tiles
            * create a new contained tiles
            * create a new shoreline tiles
            * create a list of outlet tiles
            * create a checked map to ignore tiles that are already checked
            * while let new_lake_tile = queue.pop
                * if the tile is checked already, continue
                * if new_lake_tile is part of a lake, and that lake elevation is between spillover elevation and the test elevation: TODO: Under what conditions would it be over?
                * merge that lake into this new data:
                    * add it's contained tiles to the new contained tiles
                    * add it's shoreline to the queue
                    * delete the old lake
                * else if new_lake_tile is lower in elevation than the spillover elevation: (will happen if there's a lower spot on the other side of the spillover tiles)
                * add to the list of outlet tiles
                * else if new_lake_tile is equal in greater than and equal in elevation to the spillover and less than or equal to the test_elevation:
                * add the tile to contained tiles and the shoreline
                * add it's neighbors to the queue
                * else if the new_lake_tile is greater than the test_elevation:
                * add the tile to the shorelines
                * set the new spillover to the minimum of spillover and this tile's elevation.
        * replace the lake with this lake information, and apply the lake information to all of the contained tiles.
  * set the remaining accumulation to this tile, and add it back to the queue (further outlet or flooding up to spillover will occur next time.)

* Now that lakes are calculated, we now apply lake_elevation to all of the tiles which are contained in lakes.

* Write to the database:
  * lake_elevation for each tile in a lake.
  * new values for water_flow




     */    
    Ok(())


}
