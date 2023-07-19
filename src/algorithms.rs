use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::collections::hash_map::Entry;
use std::cmp::Ordering;

use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbPoint;
use gdal::vector::OGRwkbGeometryType::wkbPolygon;
use gdal::vector::OGRwkbGeometryType::wkbLinearRing;
use ordered_float::NotNan;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Extent;
use crate::utils::Point;
use crate::utils::GeometryGeometryIterator;
use crate::world_map::VoronoiTile;

pub(crate) const DEFAULT_POINT_COUNT: f64 = 10_000.0;

enum PointGeneratorPhase {
    Top(f64),
    Bottom(f64),
    Left(f64),
    Right(f64),
    Random(f64,f64),
    Done
}

/// FUTURE: This one would be so much easier to read if I had real Function Generators.
pub(crate) struct PointGenerator<Random: Rng> {
    random: Random,
    extent: Extent,
    spacing: f64,
    offset: f64,
    boundary_width: f64,
    boundary_height: f64,
    boundary_count_x: f64,
    boundary_count_y: f64,
    radius: f64,
    jittering: f64,
    double_jittering: f64,
    phase: PointGeneratorPhase,
    
}

impl<Random: Rng> PointGenerator<Random> {

    const INITIAL_INDEX: f64 = 0.5;

    pub(crate) fn default_spacing_for_extent(spacing: Option<f64>, extent: &Extent) -> f64 {
        if let Some(spacing) = spacing {
            spacing
        } else {
            ((extent.width * extent.height)/DEFAULT_POINT_COUNT).sqrt().round_hundredths()
        }
        
    }

    pub(crate) fn new(random: Random, extent: Extent, spacing: f64) -> Self {
        let offset = -1.0 * spacing; 
        let boundary_spacing: f64 = spacing * 2.0; 
        let boundary_width = extent.width - offset * 2.0; 
        let boundary_height = extent.height - offset * 2.0; 
        let boundary_count_x = (boundary_width/boundary_spacing).ceil() - 1.0; 
        let boundary_count_y = (boundary_height/boundary_spacing).ceil() - 1.0; 
        let radius = spacing / 2.0; // FUTURE: Why is this called 'radius'?
        let jittering = radius * 0.9; // FUTURE: Customizable factor?
        let double_jittering = jittering * 2.0;
        let phase = PointGeneratorPhase::Top(Self::INITIAL_INDEX); 

        Self {
            random,
            extent,
            spacing,
            offset,
            boundary_width,
            boundary_height,
            boundary_count_x,
            boundary_count_y,
            radius,
            jittering,
            double_jittering,
            phase
        }

    }

    fn estimate_points(&self) -> usize {
        (self.boundary_count_x.floor() as usize * 2) + (self.boundary_count_y.floor() as usize * 2) + ((self.extent.width/self.spacing).floor() as usize * (self.extent.height/self.spacing).floor() as usize)
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


        macro_rules! horizontal {
            ($index: ident, $this_phase: ident, $next_phase: ident, $y: expr) => {
                if $index < self.boundary_count_x {
                    let x = ((self.boundary_width * $index)/self.boundary_count_x + self.offset).ceil(); 
                    self.phase = PointGeneratorPhase::$this_phase($index + 1.0);
                    Some(self.make_point(x,$y)) 
                } else {
                    self.phase = PointGeneratorPhase::$next_phase(Self::INITIAL_INDEX);
                    self.next()
                }
            };
        }

        macro_rules! vertical {
            ($index: ident, $this_phase: ident, $next_phase: expr, $x: expr) => {
                if $index < self.boundary_count_y {
                    let y = ((self.boundary_height * $index)/self.boundary_count_y + self.offset).ceil(); 
                    self.phase = PointGeneratorPhase::$this_phase($index + 1.0);
                    Some(self.make_point($x,y))
                } else {
                    self.phase = $next_phase;
                    self.next()
                }                
            };
        }

        macro_rules! jitter {
            () => {
                // gen creates random number between >= 0.0, < 1.0
                self.random.gen::<f64>() * self.double_jittering - self.jittering    
            };
        }

        match self.phase {
            PointGeneratorPhase::Top(index) => horizontal!(index,Top,Bottom,self.offset),
            PointGeneratorPhase::Bottom(index) => horizontal!(index,Bottom,Left,self.boundary_height + self.offset),
            PointGeneratorPhase::Left(index) => vertical!(index,Left,PointGeneratorPhase::Right(Self::INITIAL_INDEX),self.offset),
            PointGeneratorPhase::Right(index) => vertical!(index,Right,PointGeneratorPhase::Random(self.radius,self.radius),self.boundary_width+ self.offset),
            PointGeneratorPhase::Random(x, y) => if y < self.extent.height {
                if x < self.extent.width {
                    let x_j = (x + jitter!()).round_hundredths().min(self.extent.width);
                    let y_j = (y + jitter!()).round_hundredths().min(self.extent.height);
                    self.phase = PointGeneratorPhase::Random(x + self.spacing, y);
                    Some(self.make_point(x_j,y_j))
                } else {
                    self.phase = PointGeneratorPhase::Random(self.radius, y + self.spacing);
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
    Started(IntoIter<Point,Vec<Point>>),
    Done
}

pub(crate) struct VoronoiGenerator<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> {
    phase: VoronoiGeneratorPhase<GeometryIterator>

}

impl<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> VoronoiGenerator<GeometryIterator> {

    pub(crate) fn new(source: GeometryIterator) -> Self {
        let phase = VoronoiGeneratorPhase::Unstarted(source);
        Self {
            phase
        }
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

    fn sort_clockwise(center: &Point, points: &mut Vec<Point>)  {
        // TODO: Test this stuff...
        // Sort the points clockwise to create a polygon: https://stackoverflow.com/a/6989383/300213
        // The "beginning" of this ordering is north, so the "lowest" point will be the one closest to north in the northeast quadrant.
        // when angle is equal, the point closer to the center will be lesser.

        let zero: NotNan<f64> = 0.0.try_into().unwrap(); // there shouldn't be any error here.

        points.sort_by(|a: &Point, b: &Point| -> Ordering
        {
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

    fn create_voronoi(site: Point, vertices: Vec<Point>) -> Result<VoronoiTile,CommandError> {
        let mut vertices = vertices;
        Self::sort_clockwise(&site,&mut vertices);
        vertices.push(vertices[0].clone());
        let mut line = Geometry::empty(wkbLinearRing)?;
        for point in vertices {
            line.add_point_2d(point.to_tuple())
        }
        let mut polygon = Geometry::empty(wkbPolygon)?;
        polygon.add_geometry(line)?;
        Ok(VoronoiTile::new(polygon,site))

    }

    fn generate_voronoi(source: &mut GeometryIterator) -> Result<IntoIter<Point,Vec<Point>>,CommandError> {

        // Calculate a map of sites with a list of triangle circumcenters
        let mut sites: HashMap<Point, Vec<Point>> = HashMap::new(); // site,triangle_circumcenter

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
                    Entry::Occupied(mut entry) => entry.get_mut().push(circumcenter.clone()),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![circumcenter.clone()]);
                    },
                }
            }

        }

        Ok(sites.into_iter())

        // TODO: Actually, this is where we can stop and return the map iterator.
        // the generator can call the "create_voronoi" on each item.
        /*


        let polygons = Vec::new();

        let geometries = sites.map(Self::create_voronoi).collect()?;


        Ok(geometries)
        */
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
            VoronoiGeneratorPhase::Started(iter) => if let Some(value) = iter.next() {
                Some(Self::create_voronoi(value.0, value.1))
            } else {
                self.phase = VoronoiGeneratorPhase::Done;
                None
            },
            VoronoiGeneratorPhase::Done => None,
        }
    }
}


