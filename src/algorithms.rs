use rand::Rng;
use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType::wkbGeometryCollection;
use gdal::vector::OGRwkbGeometryType::wkbPoint;

use crate::errors::CommandError;
use crate::utils::RoundHundredths;
use crate::utils::Size;
use crate::raster::RasterCoordTransformer;
use crate::raster::RasterMap;
use crate::world_map::WorldMap;
use crate::progress::ProgressObserver;

pub const DEFAULT_POINT_COUNT: f64 = 10_000.0;

enum PointGeneratorPhase {
    Top(f64),
    Bottom(f64),
    Left(f64),
    Right(f64),
    Random(f64,f64),
    Done
}

/// FUTURE: This one would be so much easier to read if I had real Function Generators.
struct PointGenerator<Random: Rng> {
    phase: PointGeneratorPhase,
    spacing: f64,
    size: Size<f64>,
    offset: f64,
    boundary_width: f64,
    boundary_height: f64,
    number_x: f64,
    number_y: f64,
    radius: f64,
    jittering: f64,
    double_jittering: f64,
    random: Random,
    coord_transformer: RasterCoordTransformer
}

impl<Random: Rng> PointGenerator<Random> {

    const INITIAL_INDEX: f64 = 0.5;

    fn new(random: Random, coord_transformer: RasterCoordTransformer, spacing: f64, size: Size<f64>) -> Self {
        let offset = -1.0 * spacing; // -10.0
        let boundary_spacing: f64 = spacing * 2.0; // 20.0
        let boundary_width = size.width - offset * 2.0; // 532
        let boundary_height = size.height - offset * 2.0; // 532
        let number_x = (boundary_width/boundary_spacing).ceil() - 1.0; // 26
        let number_y = (boundary_height/boundary_spacing).ceil() - 1.0; // 26
        let radius = spacing / 2.0;
        let jittering = radius * 0.9; // FUTURE: Customizable factor?
        let double_jittering = jittering * 2.0;

        Self {
            phase: PointGeneratorPhase::Top(Self::INITIAL_INDEX),
            spacing,
            size,
            offset,
            boundary_width,
            boundary_height,
            number_x,
            number_y,
            radius,
            jittering,
            double_jittering,
            random,
            coord_transformer
        }

    }

    fn estimate_points(&self) -> usize {
        (self.number_x.floor() as usize * 2) + (self.number_y.floor() as usize * 2) + (self.size.width * self.size.height).floor() as usize
    }

    fn make_point(&self, x: f64, y: f64) -> Result<Geometry,CommandError> {
        let (lon,lat) = self.coord_transformer.pixels_to_coords(x, y);
        let mut point = Geometry::empty(wkbPoint)?;
        point.add_point_2d((lon,lat));
        Ok(point)
    }

    fn next_point(&mut self) -> Option<Result<Geometry,CommandError>> {
        // TODO: The points laying beyond the edge of the heightmap looks weird. Once I get to the voronoi, see if those are absolutely necessary.
        // TODO: Those boundary points should also be jittered, at least along the line.

        // Randomizing algorithms borrowed from AFMG with many modifications


        macro_rules! horizontal {
            ($index: ident, $this_phase: ident, $next_phase: ident, $y: expr) => {
                if $index < self.number_x {
                    let x = ((self.boundary_width * $index)/self.number_x + self.offset).ceil(); 
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
                if $index < self.number_y {
                    let y = ((self.boundary_height * $index)/self.number_y + self.offset).ceil(); 
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
            PointGeneratorPhase::Random(x, y) => if y < self.size.height {
                if x < self.size.width {
                    let x_j = (x + jitter!()).round_hundredths().min(self.size.width);
                    let y_j = (y + jitter!()).round_hundredths().min(self.size.height);
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


// TODO: I don't need the heightmap to sample from now, that's coming later. Which means I don't really need the
// map itself, I just need the size and the transformer.
// -- Actually, I could design this so I only need the extent of the target layer in lat/lon. It would require reworking
// what "spacing" means, and a few other things. I might not want to be rounding anymore. I would no longer need the transformer object at that point.
pub fn generate_points_from_heightmap<Random: Rng, Progress: ProgressObserver>(source: RasterMap, target: &mut WorldMap, overwrite_layer: bool, spacing: Option<f64>, random: &mut Random, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

    progress.start_unknown_endpoint(|| "Loading raster source.");

    let source_transformer = source.transformer()?;
    let source_size = Size::<f64>::from_usize(source.size());

    // round spacing for simplicity FUTURE: Do I really need to do this?
    let spacing = if let Some(spacing) = spacing {
        spacing.round_hundredths()
    } else {
        ((source_size.width * source_size.height)/DEFAULT_POINT_COUNT).sqrt().round_hundredths()
    };

    let generator = PointGenerator::new(random, source_transformer, spacing, source_size);

    progress.finish(|| "Raster Loaded.");

    target.load_points_layer(overwrite_layer, generator, progress)

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


