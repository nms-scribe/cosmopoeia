use std::collections::HashMap;
use core::cmp::Ordering;

use gdal::vector::OGRwkbGeometryType;
use ordered_float::NotNan;
use std::collections::hash_map::IntoIter;
use gdal::vector::Geometry;

use crate::utils::create_polygon;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::NewTileSite;
use crate::utils::Extent;
use crate::utils::Point;
use crate::errors::CommandError;


pub(crate) enum VoronoiGeneratorPhase<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> {
    Unstarted(GeometryIterator),
    Started(IntoIter<Point,VoronoiInfo>,Option<usize>)
}

pub(crate) struct VoronoiGenerator<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> {
    pub(crate) phase: VoronoiGeneratorPhase<GeometryIterator>,
    pub(crate) extent: Extent,
    pub(crate) extent_geo: Geometry

}

pub(crate) struct VoronoiInfo {
    pub(crate) vertices: Vec<Point>,
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

    pub(crate) fn circumcenter(points: (&Point,&Point,&Point)) -> Point {
        // Finding the Circumcenter: https://en.wikipedia.org/wiki/Circumcircle#Cartesian_coordinates_2

        let (a,b,c) = points;
        let d = (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y)) * 2.0;
        let d_recip = d.recip();
        let (ax2,ay2,bx2,by2,cx2,cy2) = ((a.x*a.x),(a.y*a.y),(b.x*b.x),(b.y*b.y),(c.x*c.x),(c.y*c.y));
        let (ax2_ay2,bx2_by2,cx2_cy2) = (ax2+ay2,bx2+by2,cx2+cy2);
        let ux = ((ax2_ay2)*(b.y - c.y) + (bx2_by2)*(c.y - a.y) + (cx2_cy2)*(a.y - b.y)) * d_recip;
        let uy = ((ax2_ay2)*(c.x - b.x) + (bx2_by2)*(a.x - c.x) + (cx2_cy2)*(b.x - a.x)) * d_recip;

        let u: Point = (ux,uy).into();

        u

    }

    pub(crate) fn sort_clockwise(center: &Point, points: &mut [Point], extent: &Extent, needs_a_trim: &mut bool)  {
        // Sort the points clockwise to create a polygon: https://stackoverflow.com/a/6989383/300213
        // The "beginning" of this ordering is north, so the "lowest" point will be the one closest to north in the northeast quadrant.
        // when angle is equal, the point closer to the center will be lesser.

        let zero: NotNan<f64> = 0.0.try_into().expect("Why wouldn't I be able to turn 0 into a NotNaN?"); // there shouldn't be any error here.

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
                                slope_compare => {
                                    // both are in the same quadrant now, but the slopes are not the same, we can just return the result of slope comparison:
                                    // in the northeast quadrant, a lower positive slope means it is closer to east and further away.
                                    // in the southeast quadrant, a lower negative slope means it is closer to south and further away.
                                    // in the southwest quadrant, a lower positive slope means it is closer to west and further away.
                                    // in the northwest quadrant, a lower negative slope means it is closer to north and further away from the start.
                                    slope_compare
                                }
                            }

                        },
                    }
    

                },
            }

        });
    

    }

    pub(crate) fn create_voronoi(site: &Point, voronoi: VoronoiInfo, extent: &Extent, extent_geo: &Geometry) -> Result<Option<NewTileSite>,CommandError> {
        if (voronoi.vertices.len() >= 3) && extent.contains(site) {
            // * if there are less than 3 vertices, its either a line or a point, not even a sliver.
            // * if the site is not contained in the extent, it's one of our infinity points created to make it easier for us
            // to clip the edges.
            let mut vertices = voronoi.vertices;
            // sort the vertices clockwise to make sure it's a real polygon.
            let mut needs_a_trim = false;
            Self::sort_clockwise(site,&mut vertices,extent,&mut needs_a_trim);
            vertices.push(vertices[0].clone());
            let polygon = create_polygon(&vertices)?;
            let polygon = if needs_a_trim {
                // intersection code is not trivial, just let someone else do it.
                polygon.intersection(extent_geo)
            } else {
                Some(polygon)
            };
            Ok(polygon.map(|a| NewTileSite {
                geometry: a,
                site_x: *site.x,
                site_y: *site.y,
            }))
        } else {
            // In any case, these would result in either a line or a point, not even a sliver.
            Ok(None)
        }

    }

    pub(crate) fn generate_voronoi<Progress: ProgressObserver>(source: &mut GeometryIterator, progress: &mut Progress) -> Result<IntoIter<Point,VoronoiInfo>,CommandError> {

        // Calculate a map of sites with a list of triangle circumcenters
        let mut sites: HashMap<Point, VoronoiInfo> = HashMap::new(); // site, voronoi info

        for geometry in source.watch(progress,"Generating voronoi.","Voronoi generated.") {
            let geometry = geometry?;
        
            if geometry.geometry_type() != OGRwkbGeometryType::wkbPolygon {
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

            let circumcenter = Self::circumcenter((&points[0],&points[1],&points[2]));

            // collect a list of neighboring circumcenters for each site.
            for point in points {

                match sites.get_mut(&point) {
                    None => {
                        _ = sites.insert(point, VoronoiInfo {
                                                vertices: vec![circumcenter.clone()]
                                            });
                    },
                    Some(entry) => entry.vertices.push(circumcenter.clone()),
                }

            }

        }

        Ok(sites.into_iter())

    }

    // this function is optional to call, it will automatically be called by the iterator.
    // However, that will cause a delay to the initial return.
    pub(crate) fn start<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {
        // NOTE: the delaunay thingie can only work if all of the points are known, so we can't work with an iterator here.
        // I'm not certain if some future algorithm might allow us to return an iterator, however.

        if let VoronoiGeneratorPhase::Unstarted(source) = &mut self.phase {
            let len = source.size_hint().1;
            let voronoi = Self::generate_voronoi(source,progress)?; // FUTURE: Should this be configurable?
            self.phase = VoronoiGeneratorPhase::Started(voronoi.into_iter(),len)
        }
        Ok(())
    }

}

impl<GeometryIterator: Iterator<Item=Result<Geometry,CommandError>>> Iterator for VoronoiGenerator<GeometryIterator> {

    type Item = Result<NewTileSite,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.phase {
            VoronoiGeneratorPhase::Unstarted(_) => {
                match self.start(&mut ()) {
                    Ok(_) => self.next(),
                    Err(e) => Some(Err(e)),
                }
            },
            VoronoiGeneratorPhase::Started(iter,_) => {
                let mut result = None;
                for value in iter.by_ref() {
                    // create_voronoi returns none for various reasons if the polygon shouldn't be written. 
                    // If it does that, I have to keep trying. 
                    result = Self::create_voronoi(&value.0, value.1,&self.extent,&self.extent_geo).transpose();
                    if result.is_some() {
                        break;
                    }
                }
                result
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.phase {
            VoronoiGeneratorPhase::Unstarted(iterator) => iterator.size_hint(),
            VoronoiGeneratorPhase::Started(_,hint) => (0,*hint),
        }
        
    }
}
