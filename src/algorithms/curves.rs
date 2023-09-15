use std::collections::HashMap;

use gdal::vector::Geometry;
use gdal::vector::OGRwkbGeometryType;

use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::algorithms::tiles::Theme;
use crate::errors::CommandError;
use crate::progress::WatchableIterator;
use crate::utils::Point;
use crate::world_map::TypedFeature;
use crate::utils::PolyBezier;

pub(crate) fn curvify_layer_by_theme<'target,Progress: ProgressObserver, ThemeType: Theme>(target: &'target mut WorldMapTransaction, bezier_scale: f64, progress: &mut Progress) -> Result<(),CommandError> {

    let mut vertex_index = HashMap::new();

    let mut subject_layer = ThemeType::edit_theme_layer(target)?;

    for multipolygon in subject_layer.read_geometries().watch(progress,"Indexing vertexes.","Vertexes indexed.") {
        let multipolygon = multipolygon?;

        for i in 0..multipolygon.geometry_count() {
            let polygon = multipolygon.get_geometry(i);
            for i in 0..polygon.geometry_count() {
                let ring = polygon.get_geometry(i);
                for i in 0..ring.point_count() {
                    let vertex: Point = ring.get_point(i as i32).try_into()?;
                    match vertex_index.get_mut(&vertex) {
                        Some(entry) => *entry += 1,
                        None => {
                            vertex_index.insert(vertex, 1);
                        },
                    }
                }
            }
        }
    }

    let mut segment_cache: HashMap<Point, Vec<Vec<Point>>> = HashMap::new();
    let mut polygon_segments = HashMap::new();

    // break the ring into segments where the ends indicate intersections with the boundaries of other polygons.
    for feature in ThemeType::read_features(&mut subject_layer).watch(progress,"Breaking segments.","Segments broken.") {
        let fid = feature.fid()?;
        let multipolygon = feature.geometry()?;
        let mut polygons = Vec::new();

        // each feature is a multipolygon, so iterate through polygons
        for polygon_index in 0..multipolygon.geometry_count() {
            let polygon = multipolygon.get_geometry(polygon_index);
            let mut rings = Vec::new();

            // iterate through rings on polygon
            for ring_index in 0..polygon.geometry_count() {
                let ring = polygon.get_geometry(ring_index);

                let mut segments = Vec::new();

                // convert the vertexes in the ring to points -- not that they have to be points, but they do have to be NotNaN, so this is good enough.
                let mut vertexes = (0..ring.point_count()).map(|i| {
                    ring.get_point(i as i32).try_into()
                });

                if let Some(vertex) = vertexes.next() {
                    let mut prev_vertex = vertex?;
                    let mut prev_share_count = vertex_index.get(&prev_vertex).expect("Why wouldn't this key be here if we just inserted it?");
                    let mut current_segment = vec![prev_vertex.clone()];

                    while let Some(vertex) = vertexes.next() {
                        let next_vertex = vertex?;
                        let next_share_count = vertex_index.get(&next_vertex).expect("Why wouldn't this key be here if we just inserted it?");

                        // a segment should break at all intersections:
                        if (next_share_count == &1) && (prev_share_count == &2) {
                            // the previous vertex was shared by 2, but the next one is not. The previous was a two-way intersection.
                            // if it had been three or more, then it was already broken.
                            segments.push(current_segment);
                            current_segment = vec![prev_vertex,next_vertex.clone()];
                        } else if (next_share_count == &2) && (prev_share_count == &1) || 
                                  (next_share_count > &2) { 
                            // the previous was not shared, the next one is shared by 2 OR this one was shared by 3 or more no matter what the previous was.
                            // A three-way intersection always causes a break because there should be no way for more than two polygons to share a single line segment.
                            current_segment.push(next_vertex.clone());
                            segments.push(current_segment);
                            current_segment = vec![next_vertex.clone()];
                        } else { // no intersection, segment continues as normal.
                            current_segment.push(next_vertex.clone())
                        };
                        prev_vertex = next_vertex;
                        prev_share_count = next_share_count;
                    }
                    // push the remaining current segment onto the segments
                    segments.push(current_segment);

                    if segments.len() > 1 {
                        // more than one segment in the polygon, so join the first segment to the last, because there was no
                        // reason to split them here.
                        // the last vertex should be the same as the first vertex. We will already have split them appropriately.
                        let first = segments.remove(0);
                        let last = segments.last_mut().expect("Why wouldn't there be a last if the length is greater than 1?");
                        last.truncate(last.len() - 1);
                        last.extend(first.into_iter());
                    }

                }

                // now cache them and map them to ids, because I only want to curvify a line segment once.
                let mut unique_segment_ids = Vec::new();
                for segment in segments {
                    let mut matched_id = None;
                    if let Some(match_segments) = segment_cache.get(segment.last().expect("Why wouldn't this key be here if we just inserted it?")) { // look from last first, because it's the more likely we're matching the reversed order.
                        matched_id = find_segment_match(match_segments, &segment,true);
                    } 
                    
                    if matched_id.is_none() {
                        // nothing was found, so look through in the ordinary order, there's a small chance it might match.
                        if let Some(match_segments) = segment_cache.get(&segment[0]) {
                            matched_id = find_segment_match(match_segments, &segment, false);
                        }
                    }

                    let matched_id = if let Some(matched_id) = matched_id {
                        matched_id
                    } else {
                        // still nothing, so cache it here.
                        let point = segment[0].clone();
                        let index = match segment_cache.get_mut(&segment[0]) {
                            None => {
                                segment_cache.insert(segment[0].clone(), vec![segment]);
                                0
                            },
                            Some(cache) => {
                                cache.push(segment);
                                cache.len() - 1
                            },
                        };
                        (point,index,false)

                    };

                    unique_segment_ids.push(matched_id);

                }

                rings.push(unique_segment_ids);

            }

            polygons.push(rings);
        }

        polygon_segments.insert(fid, polygons);
    }

    for value in segment_cache.values_mut().watch(progress, "Curvifying line segments.", "Line segments curvified.") {
        for line in value {
            // if this is a polygon, the polybezier stuff should automatically be curving based off the end points.
            let bezier = PolyBezier::from_poly_line(line);
            *line = bezier.to_poly_line(bezier_scale)?;
        }
    }

    let layer = ThemeType::edit_theme_layer(target)?;

    for multipolygon in polygon_segments.iter().watch(progress, "Writing reshaped polygons.", "Reshaped polygons written.") {
        let mut multipolygon_geometry = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
        let (fid,multipolygon) = multipolygon;
        for polygon in multipolygon {
            let mut polygon_geometry = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
            for ring in polygon {
                let mut ring_geometry = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;

                for (point,index,reversed) in ring {
                    let mut line = segment_cache.get(point).expect("Why wouldn't this key be here if we just inserted it?")[*index].clone();
                    if *reversed {
                        line.reverse();
                    }
                    for point in line {
                        ring_geometry.add_point_2d(point.to_tuple());
                    }
                        
                }
                polygon_geometry.add_geometry(ring_geometry)?;
            }
            multipolygon_geometry.add_geometry(polygon_geometry)?;
        }
        let mut feature = layer.try_feature_by_id(fid)?;
        feature.set_geometry(multipolygon_geometry)?;
        layer.update_feature(feature)?;

    }


    Ok(())
}

fn find_segment_match(match_segments: &Vec<Vec<Point>>, segment: &Vec<Point>, reverse: bool) -> Option<(Point, usize, bool)> {
    for (i,match_segment) in match_segments.iter().enumerate() {
        if (match_segment.len() > 0) && match_segment.len() == segment.len() {
            // search by reversed
            let matched = if reverse {
                match_segment.iter().eq(segment.iter().rev()) 
            } else {
                match_segment.iter().eq(segment.iter())
            };
            if matched {
                return Some((segment.last().expect("Why wouldn't last work if we know the len > 0?").clone(),i,true));
            }
        }
    }
    None
}