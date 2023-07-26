use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::collections::hash_map::Entry;
use std::cmp::Ordering;

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
use crate::world_map::TileDataSite;
use crate::world_map::TileDataSiteGeo;
use crate::world_map::TileDataLatElevOcean;
use crate::world_map::TileDataLat;
use crate::world_map::TileFeature;
use crate::progress::ProgressObserver;
use crate::raster::RasterMap;
use crate::world_map::TilesLayer;
use crate::tile_data;
use crate::world_map::TileData;

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

    let features = layer.features_to_vec::<_,TileDataSite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling elevations.",features.len()));

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(elevation) = band.get_value(x, y) {

            if let Some(mut feature) = layer.feature_by_id(feature.fid) {

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

    let features = layer.features_to_vec::<_,TileDataSite>(progress)?;

    progress.start_known_endpoint(|| ("Sampling oceans.",features.len()));

    let mut bad_ocean_tile_found = false;

    for (i,feature) in features.iter().enumerate() {


        let (x,y) = bounds.coords_to_pixels(feature.site_x, feature.site_y);

        if let Some(mut feature) = layer.feature_by_id(feature.fid) {

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

            feature.set_ocean(is_ocean)?;

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

    let features = layer.features_to_vec::<_,TileDataSiteGeo>(progress)?;

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

        if let Some(mut working_feature) = layer.feature_by_id(working_fid) {
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

    let features = layer.features_to_vec::<_,TileDataLatElevOcean>(progress)?;

    progress.start_known_endpoint(|| ("Generating temperatures.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let base_temp = equator_temp - (interpolate(feature.site_y.abs()/90.0) * temp_delta);
        let adabiatic_temp = base_temp - if !feature.ocean {
            (feature.elevation/1000.0)*6.5
        } else {
            0.0
        };
        let temp = (adabiatic_temp*100.0).round()/100.0;

        if let Some(mut working_feature) = layer.feature_by_id(feature.fid) {
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


    let features = layer.features_to_vec::<_,TileDataLat>(progress)?;

    progress.start_known_endpoint(|| ("Generating winds.",features.len()));

    for (i,feature) in features.iter().enumerate() {

        let wind_tier = ((feature.site_y - 89.0)/30.0).abs().floor() as usize;
        let wind_dir = winds[wind_tier];

        if let Some(mut working_feature) = layer.feature_by_id(feature.fid) {
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
    let cells_number_modifier = (layer.feature_count() as f64/10000.0).powf(0.25);
    let prec_input_modifier = moisture as f64/100.0;
    let modifier = cells_number_modifier * prec_input_modifier;

    tile_data!(TileDataForPrecipitation 
        elevation_scaled: i32, 
        wind: i32, 
        ocean: bool, 
        neighbors: Vec<(u64,i32)>,
        temperature: f64,
        precipitation: f64 = |_| {
            Ok::<_,CommandError>(0.0)
        },
        lat_modifier: f64 = |feature: &TileFeature| {
            let site_y = tile_data!(fieldassign@ feature site_y f64);
            let lat_band = ((site_y.abs() - 1.0) / 5.0).floor() as usize;
            let lat_modifier = LATITUDE_MODIFIERS[lat_band];
            Ok::<_,CommandError>(lat_modifier)
        }
    );

    let mut tile_map = layer.features_to_map::<_,TileDataForPrecipitation>(progress)?;

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_tiles: Vec<u64> = tile_map.keys().copied().collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_tiles.sort();
    let working_tiles = working_tiles;

    progress.start_known_endpoint(|| ("Tracing winds.",working_tiles.len()));

    for (i,start_fid) in working_tiles.iter().enumerate() {
        if let Some(tile) = tile_map.get(&start_fid).cloned() {

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
                        if current.ocean {
                            if !next.ocean {
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
                    if current.ocean {
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
        if let Some(mut working_feature) = layer.feature_by_id(fid) {

            working_feature.set_precipitation(tile.precipitation)?;

            layer.update_feature(working_feature)?;
        }


    }

    progress.finish(|| "Precipitation written.");

    Ok(())
}


pub(crate) fn generate_flowage<Progress: ProgressObserver>(_layer: &mut TilesLayer, _progress: &mut Progress) -> Result<(),CommandError> {

// TODO: Do the flow first, so I can make sure that works, then we'll go on to the lakes. In fact, this might just return the map and queue so we can link them into a separate function later.

/*
   #[derive(Clone)]
   struct WaterInfoTile {
       elevation: i32,
       is_ocean: bool,
       neighbors: Vec<u64>,
       temperature: f64,
       precipitation: f64,
       flowage: f64,
       lake_depth_scaled: i32,
   }
   
   progress.start_known_endpoint(|| ("Water: mapping tiles.",layer.feature_count() as usize));
   let mut tile_map = HashMap::new();
   let mut queue = Vec::new();

   for (i,feature) in layer.features().enumerate() {
       let fid = feature.fid();
       let elevation_scaled = feature.field_as_integer_by_name(TilesLayer::FIELD_ELEVATION_SCALED)?;
       let temperature = feature.field_as_double_by_name(TilesLayer::FIELD_TEMPERATURE)?;
       let precipitation = feature.field_as_double_by_name(TilesLayer::FIELD_PRECIPITATION)?;
       let is_ocean = feature.field_as_integer_by_name(TilesLayer::FIELD_OCEAN)?;
       let neighbors = feature.field_as_string_by_name(TilesLayer::FIELD_NEIGHBOR_TILES)?;

       if let (Some(fid),Some(elevation),Some(is_ocean),Some(temperature),Some(precipitation),Some(neighbors)) = (fid,elevation_scaled,is_ocean,temperature,precipitation,neighbors) {

           let neighbors: Vec<u64> = neighbors.split(',').filter_map(|a| {
               let mut a = a.splitn(2, ':');
               if let Some(neighbor) = a.next().map(|n| n.parse().ok()).flatten() {
                   Some(neighbor)
               } else {
                   None
               }
               
           }).collect();

           let is_ocean = is_ocean != 0;

           tile_map.insert(fid, WaterInfoTile {
               elevation,
               is_ocean,
               neighbors,
               temperature,
               precipitation,
               flowage: 0.0,
               lake_depth_scaled: 0
           });

           queue.push((fid,precipitation.floor() as i32));

           progress.update(|| i);

       }

   }

   progress.finish(|| "Water: tiles mapped.");

    /*
* read features:
    * tile_map: a map of tiles by id, with elevation, precipitation, and new fields water_flow, flow_to, lake_id for tracking data
    * tile_list: a list of **land** tile ids to work on
* queue: Create an empty queue of later "jobs"
* sort tile_list so that the highest elevations are first.
* for each tile in tile_list:
   * let water_flow = tile_map[tile].water_flow + tile_map[tile].precipitation
   * tile_map[tile].water_flow = water_flow
   * find it's lowest neighbors: there may be more than one if they're a bunch that are equal
   * if there are no lowest neighbors, which is unlikely, then just continue.
   * if lowest neighbors are below this tile's elevation:
        * let neighbor_flow = water_flow/neighbors.len
        * for each neighbor: tile_map[neighbor].water_flow += neighbor_flow
        * tile_map[tile].flow_to = neighbors
    * else if they are equal to or higher:
        * queue,push(fill_lake_task,tile,water_flow)

* let next_lake_id = (1..).iter()
* let lake_map: map of lakes by id, with contained_tiles, shoreline_tiles, outlet_tiles, lake_elevation, spillover_elevation
* for each task in queue:
    * if fill_lake_task:
        * if tile_map[tile].lake_id is none: // new lake
            * use this tile as the contained_tiles
            * copy the neighbors into shoreline_tiles
            * calculate the lowest elevation from the neighbors as spillover_elevation
            * there are no known outlets yet, so don't bother with that.
            * lake_elevation = tile.elevation
            * create new lake, give it a an id from next_lake_id and add it to the table.
        * otherwise:
            * try to get the lake off the map based on that id.
        * if there are outlet tiles:
            * queue.push(flow_water_task,outlet_tile,water_flow/outlet_tiles.len)
            * do I want to give lower tiles higher divisions of the flow?
        * else:
            * let new_lake_elevation = lake.elevation + (water_flow/lake.contained_tiles.len)
            * if new_lake_elevation > lake.spillover_elevation:
              * lake.elevation = lake.spillover_elevation
              * water_flow = (new_lake_elevation - lake.elevation) * lake.contained_tiles.len
            * if water_flow > 0: // we still have some water left over, so start "spreading" the lake.
              * let new_lake_area = (lake.contained_tiles.len.sqrt + 1).pow(2)
              * let new_lake_elevation = lake.elevation + (water_flow/(lake.contained_tiles.len.sqrt + 1.pow(2)))
                // the idea is to just assume that the lake will grow a little bigger. a 1-tile lake will become a 4 tile lake, 4 tiles will become 9, etc.
                // I'm not set on that.
              * flood-fill lake to that level:
                * let filled = copy of lake.contained_tiles
                * let checked = copy of lake.contained_tiles
                * let check_queue = copy of lake.shoreline_tiles
                * let new_shoreline = []
                * let new_outlet = []
                * while check = check_queue.pop
                  * if check is in checked, then continue
                  * add check to checked
                  * if check elevation is higher than new_lake_elevation:
                    * add check to new_shoreline
                    * continue
                  * else if check is part of a lake:
                    * if the lake elevation is higher, I'm not sure what to do, since it shouldn't be.
                    * if the lake elevation is the same or lower:
                      * add the contents of the other lake to filled
                      * add the shoreline of the other lake to the check_queue
                      * update the tiles to match this lake's id
                      * delete the old lake
                  * else if check is an ocean: add the check to new_outlet and new_shoreline and continue
                  * else if check is less than the old spillover level, which will only happen if we've gone over the first shoreline and are now going downhill
                    * add check to new_shoreline and new_outlet and continue
                  * else 
                    * add check to filled
                    * add check.neighbors to check_queue
              * replace the lake data with the new lake data from above
              * if the new lake has outlet tiles: divide the waterflow among with tiles of greater difference between their elevation and the lake level getting more. Then push a flow water task on the queue for them.
    * if flow_water task:
        * tile_map[tile].water_flow += water_flow (increase)
        * assuming there is a tile_map[tile].flow_to:
            * let neighbor_flow = water_flow/neighbors.len
            * for each neighbor: queue.push(flow_water_task,neighbor,neighbor_flow)
        * if there are no flow_to neighbors then convert this to a fill_lake task.

* finally, write this to the database for each tile:
  * flow, flow_to, lake_elevation for each lake






     */*/
    Ok(())
}
