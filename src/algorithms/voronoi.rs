use std::collections::HashMap;
use core::cmp::Ordering;

use std::collections::hash_map::IntoIter;

use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::world_map::tile_layer::NewTileSite;
use crate::utils::extent::Extent;
use crate::utils::coordinates::Coordinates;
use crate::utils::world_shape::WorldShape;
use crate::errors::CommandError;
use crate::geometry::Polygon;
use crate::geometry::LinearRing;
use crate::utils::edge::Edge;
use crate::geometry::GDALGeometryWrapper;


pub(crate) enum VoronoiGeneratorPhase<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> {
    Unstarted(GeometryIterator),
    Started(IntoIter<Coordinates,VoronoiInfo>,Option<usize>)
}

pub(crate) struct VoronoiGenerator<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> {
    phase: VoronoiGeneratorPhase<GeometryIterator>,
    world_shape: WorldShape,
    extent: Extent,
    extent_geo: Polygon

}

pub(crate) struct VoronoiInfo {
    vertices: Vec<Coordinates>,
}

impl<GeometryIterator: Iterator<Item=Result<Polygon,CommandError>>> VoronoiGenerator<GeometryIterator> {

    pub(crate) fn new(source: GeometryIterator, extent: Extent, shape: WorldShape) -> Result<Self,CommandError> {
        let phase = VoronoiGeneratorPhase::Unstarted(source);
        let extent_geo = extent.create_polygon()?;
        Ok(Self {
            phase,
            extent,
            world_shape: shape,
            extent_geo
        })
    }

    pub(crate) fn create_voronoi(site: &Coordinates, voronoi: VoronoiInfo, extent: &Extent, world_shape: &WorldShape, extent_geo: &Polygon) -> Result<Option<NewTileSite>,CommandError> {
        if (voronoi.vertices.len() >= 3) && extent.contains(site) {
            // * if there are less than 3 vertices, its either a line or a point, not even a sliver.
            // * if the site is not contained in the extent, it's one of our infinity points created to make it easier for us
            // to clip the edges.
            let mut vertices = voronoi.vertices;

            // figure out if it lay off the edge of the map:
            // Note that there may be cases where the vertices are in edges that conflict (such as northeast and southeast). While
            // this will be an error for the end, it is not an error here. Mark those as erroneous edges instead, so they will still
            // get clipped to the extent.
            let mut edge: Option<Result<Edge,()>> = None;
            for point in &vertices {
                if let Some(point_edge) = extent.is_off_edge(point) {
                    match edge {
                        Some(Ok(previous_edge)) => {
                            edge = Some(match point_edge.combine_with(previous_edge) {
                                Ok(new_edge) => Ok(new_edge),
                                Err(err) => match err {
                                    CommandError::InvalidTileEdge(_, _) => Err(()),
                                    err => return Err(err)
                                },
                            });
                        },
                        Some(Err(())) => {
                            edge = Some(Err(()));
                        }
                        _ => edge = Some(Ok(point_edge)),
                    }
                } // else keep previous edge

                
            }

            // Sort the points clockwise to create a polygon: https://stackoverflow.com/a/6989383/300213
            // The "beginning" of this ordering is north, so the "lowest" point will be the one closest to north in the northeast quadrant.
            // when angle is equal, the point closer to the center will be lesser.
            vertices.sort_by(|a: &Coordinates, b: &Coordinates| -> Ordering
            {
                Coordinates::order_clockwise(a, b, site)
            });

            // push a copy of the first vertex onto the end.
            vertices.push(vertices[0].clone());
            let ring = LinearRing::from_vertices(vertices.iter().map(Coordinates::to_tuple))?;
            let polygon = Polygon::from_rings([ring])?;
            let polygon = if edge.is_some() {
                // intersection code is not trivial, just let someone else do it.
                polygon.intersection(extent_geo)?.try_into()?
            } else {
                polygon
            };

            // there were some false positives for the diagonal edges, these need to be fixed, and it's best done now.
            // Also need to fix those which got an invalid edge originally.
            // This will usually only apply to eight or ten, so it's a small task.
            let edge = if let Some(corner) = edge {
                match corner {
                    Err(()) |
                    Ok(Edge::Northeast |
                       Edge::Southeast |
                       Edge::Southwest |
                       Edge::Northwest) => {
                        let bounds = polygon.get_envelope();
                        extent.is_extent_on_edge(&bounds)?
                    },
                    Ok(correct_edge @ (Edge::North |
                               Edge::East |
                               Edge::South |
                               Edge::West)) => Some(correct_edge)
                    
                }

            } else {
                None
            };

            let area = polygon.shaped_area(world_shape)?;

            Ok(Some(NewTileSite::new(polygon, site.clone(), edge, area)))
        } else {
            // In any case, these would result in either a line or a point, not even a sliver.
            Ok(None)
        }

    }

    pub(crate) fn generate_voronoi<Progress: ProgressObserver>(source: &mut GeometryIterator, shape: &WorldShape, progress: &mut Progress) -> Result<IntoIter<Coordinates,VoronoiInfo>,CommandError> {

        // Calculate a map of sites with a list of triangle circumcenters
        let mut sites: HashMap<Coordinates, VoronoiInfo> = HashMap::new(); // site, voronoi info

        for geometry in source.watch(progress,"Generating voronoi.","Voronoi generated.") {
            let geometry = geometry?;
        
            let line = geometry.get_ring(0)?; // this should be the outer ring for a triangle.

            if line.len() != 4 { // the line should be a loop, with the first and last elements
                return Err(CommandError::VoronoiExpectsTriangles("Not enough points in a polygon.".to_owned()));
            }

            let points: [Coordinates; 3] = (0..3)
               .map(|i| Ok(line.get_point(i).try_into()?)).collect::<Result<Vec<Coordinates>,CommandError>>()?
               .try_into()
               .map_err(|e| CommandError::VoronoiExpectsTriangles(format!("{e:?}")))?;

            let circumcenter = Coordinates::shaped_circumcenter((&points[0],&points[1],&points[2]),shape)?;

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
            let voronoi = Self::generate_voronoi(source,&self.world_shape,progress)?; // FUTURE: Should this be configurable?
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
                    Ok(()) => self.next(),
                    Err(e) => Some(Err(e)),
                }
            },
            VoronoiGeneratorPhase::Started(iter,_) => {
                let mut result = None;
                for value in iter.by_ref() {
                    // create_voronoi returns none for various reasons if the polygon shouldn't be written. 
                    // If it does that, I have to keep trying. 
                    result = Self::create_voronoi(&value.0, value.1,&self.extent,&self.world_shape,&self.extent_geo).transpose();
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
