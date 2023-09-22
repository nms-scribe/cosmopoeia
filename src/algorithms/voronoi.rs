use std::collections::HashMap;
use core::cmp::Ordering;

use std::collections::hash_map::IntoIter;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::NewTileSite;
use crate::utils::Extent;
use crate::utils::Point;
use crate::errors::CommandError;
use crate::geometry::Polygon;
use crate::geometry::LinearRing;


pub(crate) enum VoronoiGeneratorPhase<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> {
    Unstarted(GeometryIterator),
    Started(IntoIter<Point,VoronoiInfo>,Option<usize>)
}

pub(crate) struct VoronoiGenerator<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> {
    pub(crate) phase: VoronoiGeneratorPhase<GeometryIterator>,
    pub(crate) extent: Extent,
    pub(crate) extent_geo: Polygon

}

pub(crate) struct VoronoiInfo {
    pub(crate) vertices: Vec<Point>,
}

impl<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> VoronoiGenerator<GeometryIterator> {

    pub(crate) fn new(source: GeometryIterator, extent: Extent) -> Result<Self,CommandError> {
        let phase = VoronoiGeneratorPhase::Unstarted(source);
        let extent_geo = extent.create_polygon()?;
        Ok(Self {
            phase,
            extent,
            extent_geo
        })
    }

    pub(crate) fn sort_clockwise(center: &Point, points: &mut [Point], extent: &Extent, needs_a_trim: &mut bool)  {
        // Sort the points clockwise to create a polygon: https://stackoverflow.com/a/6989383/300213
        // The "beginning" of this ordering is north, so the "lowest" point will be the one closest to north in the northeast quadrant.
        // when angle is equal, the point closer to the center will be lesser.

        points.sort_by(|a: &Point, b: &Point| -> Ordering
        {
            if !*needs_a_trim {
                *needs_a_trim = (!extent.contains(a)) || (!extent.contains(b))
            }
            Point::order_clockwise(a, b, center)

        });
    

    }

    pub(crate) fn create_voronoi(site: &Point, voronoi: VoronoiInfo, extent: &Extent, extent_geo: &Polygon) -> Result<Option<NewTileSite>,CommandError> {
        if (voronoi.vertices.len() >= 3) && extent.contains(site) {
            // * if there are less than 3 vertices, its either a line or a point, not even a sliver.
            // * if the site is not contained in the extent, it's one of our infinity points created to make it easier for us
            // to clip the edges.
            let mut vertices = voronoi.vertices;
            // sort the vertices clockwise to make sure it's a real polygon.
            let mut needs_a_trim = false;
            Self::sort_clockwise(site,&mut vertices,extent,&mut needs_a_trim);
            // push a copy of the first vertex onto the end.
            vertices.push(vertices[0].clone());
            let ring = LinearRing::from_vertices(vertices.iter().map(Point::to_tuple))?;
            let polygon = Polygon::from_rings([ring])?;
            let polygon = if needs_a_trim {
                // intersection code is not trivial, just let someone else do it.
                polygon.intersection(extent_geo)?.try_into()?
            } else {
                polygon
            };
            Ok(Some(NewTileSite {
                geometry: polygon,
                site: site.clone()
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
        
            let line = geometry.get_ring(0)?; // this should be the outer ring for a triangle.

            if line.len() != 4 { // the line should be a loop, with the first and last elements
                return Err(CommandError::VoronoiExpectsTriangles);
            }

            let points: [Point; 3] = (0..3)
               .map(|i| Ok(line.get_point(i).try_into()?)).collect::<Result<Vec<Point>,CommandError>>()?
               .try_into()
               .map_err(|_| CommandError::VoronoiExpectsTriangles)?;

            let circumcenter = Point::circumcenter((&points[0],&points[1],&points[2]));

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

impl<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> Iterator for VoronoiGenerator<GeometryIterator> {

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
