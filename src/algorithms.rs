use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::collections::hash_map::Entry;
use std::cmp::Ordering;

use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::OGRwkbGeometryType::wkbPolygon;
use gdal::vector::Layer;
use gdal::vector::LayerAccess;
use ordered_float::NotNan;

use crate::errors::CommandError;
use crate::utils::Extent;
use crate::utils::Point;
use crate::utils::GeometryGeometryIterator;
use crate::utils::create_polygon;
use crate::world_map::VoronoiTile;
use crate::progress::ProgressObserver;
use crate::raster::RasterMap;
use crate::world_map::TilesLayer;

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
                self.phase = PointGeneratorPhase::Random(0.0,0.0);
                Some(self.make_point(-self.extent.width, self.extent.height*2.0))
            },
            PointGeneratorPhase::Random(x, y) => if y < &self.extent.height {
                if x < &self.extent.width {
                    let x_j = (x + jitter!()).clamp(0.0,self.extent.width);
                    let y_j = (y + jitter!()).clamp(0.0,self.extent.height);
                    self.phase = PointGeneratorPhase::Random(x + self.spacing, *y);
                    Some(self.make_point(x_j,y_j))
                } else {
                    self.phase = PointGeneratorPhase::Random(0.0, y + self.spacing);
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

    fn create_voronoi(site: Point, voronoi: VoronoiInfo, extent: &Extent, extent_geo: &Geometry) -> Result<Option<VoronoiTile>,CommandError> {
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
            Ok(polygon.map(|a| VoronoiTile::new(a,site)))
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

    type Item = Result<VoronoiTile,CommandError>; // TODO: Should be a voronoi struct defined in world_map.

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



pub(crate) fn sample_elevations<Progress: ProgressObserver>(layer: &mut Layer, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

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

    progress.start_known_endpoint(|| ("Reading tiles",layer.feature_count() as usize));

    let mut features = Vec::new();

    for (i,feature) in layer.features().enumerate() {
        features.push((i,
                       feature.fid(),
                       feature.field_as_double_by_name(TilesLayer::FIELD_SITE_X)?,
                       feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?
        ))

    }

    progress.finish(|| "Tiles read.");

    progress.start_known_endpoint(|| ("Sampling elevations.",layer.feature_count() as usize));

    for (i,fid,site_lon,site_lat) in features {


        if let (Some(fid),Some(site_lon),Some(site_lat)) = (fid,site_lon,site_lat) {

            let (x,y) = bounds.coords_to_pixels(site_lon, site_lat);

            if let Some(elevation) = band.get_value(x, y) {

                if let Some(feature) = layer.feature(fid) {

                    let elevation_scaled = if elevation >= &0.0 {
                        20 + (elevation * positive_elevation_scale).floor() as i32
                    } else {
                        20 - (elevation.abs() * negative_elevation_scale).floor() as i32
                    };

                    feature.set_field_double(TilesLayer::FIELD_ELEVATION, *elevation)?;
                    feature.set_field_integer(TilesLayer::FIELD_ELEVATION_SCALED, elevation_scaled)?;

                    layer.set_feature(feature)?;
    
                }

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

pub(crate) fn sample_ocean<Progress: ProgressObserver>(layer: &mut Layer, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

    progress.start_unknown_endpoint(|| "Reading raster");

    let band = raster.read_band::<f64>(1)?;
    let bounds = raster.bounds()?;
    let no_data_value = band.no_data_value();

    progress.finish(|| "Raster read.");

    progress.start_known_endpoint(|| ("Reading tiles",layer.feature_count() as usize));

    let mut features = Vec::new();

    for (i,feature) in layer.features().enumerate() {
        features.push((i,
                       feature.fid(),
                       feature.field_as_double_by_name(TilesLayer::FIELD_SITE_X)?,
                       feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?
        ))

    }

    progress.finish(|| "Tiles read.");

    progress.start_known_endpoint(|| ("Sampling oceans.",layer.feature_count() as usize));

    let mut bad_ocean_tile_found = false;

    for (i,fid,site_lon,site_lat) in features {


        if let (Some(fid),Some(site_lon),Some(site_lat)) = (fid,site_lon,site_lat) {

            let (x,y) = bounds.coords_to_pixels(site_lon, site_lat);

            if let Some(feature) = layer.feature(fid) {

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

                if let Some(elevation) = feature.field_as_double_by_name(TilesLayer::FIELD_ELEVATION)? {
                    if is_ocean && (elevation > 0.0) {
                        bad_ocean_tile_found = true;
                    }

                }

                let is_ocean = if is_ocean { 1 } else { 0 };

                feature.set_field_integer(TilesLayer::FIELD_OCEAN, is_ocean)?;

                layer.set_feature(feature)?;

            }



        }

        progress.update(|| i);




    }

    progress.finish(|| "Oceans sampled.");

    if bad_ocean_tile_found {
        println!("At least one ocean tile was found with an elevation above 0.")

    }

    Ok(())
}


pub(crate) fn calculate_neighbors<Progress: ProgressObserver>(layer: &mut Layer, progress: &mut Progress) -> Result<(),CommandError> {

    progress.start_known_endpoint(|| ("Calculating neighbors.",layer.feature_count() as usize));

    let features: Result<Vec<(Option<u64>,Option<Geometry>,Option<f64>,Option<f64>)>,CommandError> = layer.features().map(|feature| Ok((
        feature.fid(),
        feature.geometry().cloned(),
        feature.field_as_double_by_name(TilesLayer::FIELD_SITE_X)?,
        feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?,
    ))).collect();
    let features = features?;

    // # Loop through all features and find features that touch each feature
    // for f in feature_dict.values():
    for (i,(working_fid,working_geometry,site_x,site_y)) in features.iter().enumerate() {

        if let Some(working_fid) = working_fid {
            if let Some(working_geometry) = working_geometry {

                let envelope = working_geometry.envelope();
                layer.set_spatial_filter_rect(envelope.MinX, envelope.MinY, envelope.MaxX, envelope.MaxY);
    
    
                let mut neighbors = Vec::new();
    
                for intersecting_feature in layer.features() {
    
                    if let Some(intersecting_fid) = intersecting_feature.fid() {
                        if (working_fid != &intersecting_fid) && (!intersecting_feature.geometry().unwrap().disjoint(&working_geometry)) {

                            let neighbor_site_x = intersecting_feature.field_as_double_by_name(TilesLayer::FIELD_SITE_X)?;
                            let neighbor_site_y = intersecting_feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?;
                            let neighbor_angle = if let (Some(site_x),Some(site_y),Some(neighbor_site_x),Some(neighbor_site_y)) = (site_x,site_y,neighbor_site_x,neighbor_site_y) {
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
                        
                            neighbors.push(format!("{}:{}",intersecting_fid,neighbor_angle)) 
                        }

                    }
    
                }
                
                layer.clear_spatial_filter();

                if let Some(working_feature) = layer.feature(*working_fid) {
                    working_feature.set_field_string(TilesLayer::FIELD_NEIGHBOR_TILES, &neighbors.join(","))?;
    
                    layer.set_feature(working_feature)?;

                }
    

            }
        }

        progress.update(|| i);

    }

    progress.finish(|| "Neighbors calculated.");

    Ok(())
}


pub(crate) fn generate_temperatures<Progress: ProgressObserver>(layer: &mut Layer, equator_temp: i8, polar_temp: i8, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications

    progress.start_known_endpoint(|| ("Generating temperatures.",layer.feature_count() as usize));

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

    let features: Result<Vec<(Option<u64>,Option<f64>,Option<f64>,Option<i32>)>,CommandError> = layer.features().map(|feature| Ok((
        feature.fid(),
        feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?,
        feature.field_as_double_by_name(TilesLayer::FIELD_ELEVATION)?,
        feature.field_as_integer_by_name(TilesLayer::FIELD_OCEAN)?,
    ))).collect();
    let features = features?;

    for (i,feature) in features.iter().enumerate() {

        if let (Some(working_fid),Some(site_y),Some(elevation),Some(is_ocean)) = feature {
            let base_temp = equator_temp - (interpolate(site_y.abs()/90.0) * temp_delta);
            let adabiatic_temp = base_temp - if is_ocean == &0 {
                (elevation/1000.0)*6.5
            } else {
                0.0
            };
            let temp = (adabiatic_temp*100.0).round()/100.0;

            if let Some(working_feature) = layer.feature(*working_fid) {
                working_feature.set_field_double(TilesLayer::FIELD_TEMPERATURE, temp)?;

                layer.set_feature(working_feature)?;

            }


        }

        progress.update(|| i);


    }

    progress.finish(|| "Temperatures calculated.");

    Ok(())
}



pub(crate) fn generate_winds<Progress: ProgressObserver>(layer: &mut Layer, winds: [f64; 6], progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications

    progress.start_known_endpoint(|| ("Generating winds.",layer.feature_count() as usize));

    let features: Result<Vec<(Option<u64>,Option<f64>)>,CommandError> = layer.features().map(|feature| Ok((
        feature.fid(),
        feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?,
    ))).collect();
    let features = features?;

    for (i,feature) in features.iter().enumerate() {

        if let (Some(working_fid),Some(site_y)) = feature {

            let wind_tier = ((site_y - 89.0)/30.0).abs().floor() as usize;
            let wind_dir = winds[wind_tier];

            if let Some(working_feature) = layer.feature(*working_fid) {
                working_feature.set_field_double(TilesLayer::FIELD_WIND, wind_dir)?;

                layer.set_feature(working_feature)?;

            }


        }

        progress.update(|| i);


    }

    progress.finish(|| "Winds generated.");

    Ok(())
}



pub(crate) fn generate_precipitation<Progress: ProgressObserver>(layer: &mut Layer, moisture: u16, progress: &mut Progress) -> Result<(),CommandError> {

    // Algorithm borrowed from AFMG with some modifications, most importantly I don't have a grid, so I follow the paths of the wind to neighbors.

    const MAX_PASSABLE_ELEVATION: i32 = 85; // FUTURE: I've found that this is unnecessary, the elevation change should drop the precipitation and prevent any from passing on. 

    // I believe what this does is scale the moisture scale correctly to the size of the map. Otherwise, I don't know.
    let cells_number_modifier = (layer.feature_count() as f64/10000.0).powf(0.25);
    let prec_input_modifier = moisture as f64/100.0;
    let modifier = cells_number_modifier * prec_input_modifier;

    // Bands of rain at different latitudes, like the ITCZ
    let latitude_modifier = [4.0, 2.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 3.0, 2.0, 2.0, 1.0, 1.0, 1.0, 0.5];

    #[derive(Clone)]
    struct PrecipitationTile {
        elevation: i32,
        wind: i32,
        is_ocean: bool,
        neighbors: Vec<(u64,i32)>,
        lat_modifier: f64,
        temperature: f64,
        precipitation: f64
    }
    
    progress.start_known_endpoint(|| ("Precipitation: mapping tiles.",layer.feature_count() as usize));
    let mut tile_map = HashMap::new();

    for (i,feature) in layer.features().enumerate() {
        let fid = feature.fid();
        let site_y = feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?;
        let elevation_scaled = feature.field_as_integer_by_name(TilesLayer::FIELD_ELEVATION_SCALED)?;
        let wind = feature.field_as_integer_by_name(TilesLayer::FIELD_WIND)?;
        let temperature = feature.field_as_double_by_name(TilesLayer::FIELD_TEMPERATURE)?;
        let is_ocean = feature.field_as_integer_by_name(TilesLayer::FIELD_OCEAN)?;
        let neighbors = feature.field_as_string_by_name(TilesLayer::FIELD_NEIGHBOR_TILES)?;

        if let (Some(fid),Some(lat),Some(elevation),Some(wind),Some(is_ocean),Some(temperature),Some(neighbors)) = (fid,site_y,elevation_scaled,wind,is_ocean,temperature,neighbors) {

            let lat_band = ((lat.abs() - 1.0) / 5.0).floor() as usize;
            let lat_modifier = latitude_modifier[lat_band];

            let neighbors: Vec<(u64,i32)> = neighbors.split(',').filter_map(|a| {
                let mut a = a.splitn(2, ':');
                if let Some(neighbor) = a.next().map(|n| n.parse().ok()).flatten() {
                    if let Some(direction) = a.next().map(|d| d.parse().ok()).flatten() {
                        if direction >= 0 {
                            Some((neighbor,direction))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
                
            }).collect();

            let is_ocean = is_ocean != 0;

            tile_map.insert(fid, PrecipitationTile {
                elevation,
                wind,
                is_ocean,
                neighbors,
                lat_modifier,
                temperature,
                precipitation: 0.0
            });

            progress.update(|| i);

        }

    }

    progress.finish(|| "Precipitation: tiles mapped.");

    // I can't work on the tiles map while also iterating it, so I have to copy the keys
    let mut working_tiles: Vec<u64> = tile_map.keys().copied().collect();
    // The order of the tiles changes the results, so make sure they are always in the same order to 
    // keep the results reproducible. I know this seems OCD, but it's important if anyone wants
    // to test things.
    working_tiles.sort();
    let working_tiles = working_tiles;

    progress.start_known_endpoint(|| ("Precipitation: tracing winds.",working_tiles.len()));

    for (i,start_fid) in working_tiles.iter().enumerate() {
        if let Some(tile) = tile_map.get(&start_fid).cloned() {

            let max_prec = 120.0 * tile.lat_modifier;
            let mut humidity = max_prec - tile.elevation as f64;

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
                            let is_passable = next.elevation < MAX_PASSABLE_ELEVATION;
                            let precipitation = if is_passable {
                                // precipitation under normal conditions
                                let normal_loss = (humidity / (10.0 * current.lat_modifier)).max(1.0);
                                // difference in height
                                let diff = (next.elevation - current.elevation).max(0) as f64;
                                // additional modifier for high elevation of mountains
                                let modifier = (next.elevation / 70).pow(2) as f64;
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

    progress.finish(|| "Precipitation: winds traced.");

    progress.start_known_endpoint(|| ("Writing precipitation",tile_map.len()));

    for (fid,tile) in tile_map {
        if let Some(working_feature) = layer.feature(fid) {

            working_feature.set_field_double(TilesLayer::FIELD_PRECIPITATION, tile.precipitation)?;

            layer.set_feature(working_feature)?;
        }


    }

    progress.finish(|| "Precipitation written.");

    Ok(())
}
