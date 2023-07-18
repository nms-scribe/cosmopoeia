use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbGeometryCollection;
use gdal::vector::OGRwkbGeometryType::wkbPoint;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Extent;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;

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

    pub(crate) fn default_spacing(extent: &Extent) -> f64 {
        ((extent.width * extent.height)/DEFAULT_POINT_COUNT).sqrt().round_hundredths()
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

    fn next_point(&mut self) -> Option<Result<Geometry,CommandError>> {
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
                    self.next_point()
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
                    self.next_point()
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
                    self.next_point()
                }
                
            } else {
                self.phase = PointGeneratorPhase::Done;
                self.next_point()
            },
            PointGeneratorPhase::Done => None,
        }

    }


}

impl<Random: Rng> Iterator for PointGenerator<Random> {

    type Item = Result<Geometry,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_point()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0,Some(self.estimate_points()))
    }
}


// TODO: I'm leaning more and more into keeping everything in a single gpkg file as standard, as those can support multiple layers. I might
// even be able to store the non-geographic lookup tables with wkbNone geometries. I'm just not certain what to do with those.
pub(crate) fn generate_delaunary_triangles_from_points<Progress: ProgressObserver>(target: &mut WorldMap, overwrite_layer: bool, tolerance: Option<f64>, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

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

    // TODO: You can also find the dual (ie. Voronoi diagram) just by computing the circumcentres of all the triangles, and connecting any two circumcentres whose triangles share an edge.
    // - Given a list of (delaunay) triangles where each vertice is one of the sites
    // - Calculate a map (A) of triangle data with its circumcenter TODO: How?
    // - Calculate a map (B) of sites with a list of triangle circumcenters TODO: How?
    // - for each site and list in B
    //   - if list.len < 2: continue (This is a *true* edge case, see below) TODO: How to deal with these?
    //   - vertices = list.clone()
    //   - sort vertices in clockwise order TODO: How? (See below)
    //   - vertices.append(vertices[0].clone()) // to close the polygon
    //   - create new polygon D(vertices)
    //   - sample the elevation from the heightmap given the site coordinates
    //   - add polygon D to layer with site elevation attribute

    // TODO: Finding the Circumcenter: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2

    // TODO: Finding the map of sites to triangle circumcenters:
    // - create the map
    // - for each triangle in the list of triangles
    //   - for each vertex:
    //     - if the map has the site, then add this triangle's circumcenter to the list
    //     - if the map does not have the site, then add the map with a single item list containing this triangle's circumcenter

    // TODO: Actually, we can simplify this: when creating the map, just add the circumcenter vertex

    // TODO: Sorting points clockwise:
    // - https://stackoverflow.com/a/6989383/300213 -- this is relatively simple, although it does take work.
    // - Alternatively, there is a concave hull in gdal which would work, except it's not included in the rust bindings.


    // TODO: I think I'm going to rethink this, since I'm having to store things in memory anyway, and the originally generated points aren't
    // necessarily the ones I get from the database, the algorithms should deal with the types themselves and only occasionally the data files.
    // Basically:
    // - generate_random_points(extent NOT the layer) -> Points
    // - calculate_delaunay(points) -> triangles
    // - calculate_voronoi(triangles) -> voronois (polygons with "sites")
    // - create_tiles(voronois,heightmap) -> create layer with the voronoi polygons, sampling the elevations from the heightmap
    // - however, if I'm using the gdal types (until I can get better support for the geo_types), I can have stuff that will write the data to layers for visualization


}


