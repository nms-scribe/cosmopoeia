use std::collections::HashMap;

use crate::progress::ProgressObserver;
use crate::world_map::WorldMapTransaction;
use crate::algorithms::tiles::Theme;
use crate::errors::CommandError;
use crate::progress::WatchableIterator as _;
use crate::utils::coordinates::Coordinates;
use crate::typed_map::features::TypedFeature as _;
use crate::algorithms::beziers::bezierify_points;
use crate::commands::BezierScaleArg;
use crate::typed_map::features::TypedFeatureIterator;
use crate::geometry::MultiPolygon;
use crate::geometry::Polygon;
use crate::geometry::LinearRing;
use crate::geometry::GDALGeometryWrapper as _;
use crate::geometry::VariantArealGeometry;
use crate::typed_map::fields::IdRef;

pub(crate) fn curvify_layer_by_theme<Progress: ProgressObserver, ThemeType: Theme>(target: &mut WorldMapTransaction, bezier_scale: &BezierScaleArg, progress: &mut Progress) -> Result<(),CommandError> {

    let extent_polygon: VariantArealGeometry = target.edit_tile_layer()?.get_extent()?.create_polygon()?.into();

    let mut index_subject_layer = ThemeType::edit_theme_layer(target)?;
    let index_features = ThemeType::read_theme_features(&mut index_subject_layer);
    let vertex_index = index_vertexes::<ThemeType,_>(index_features, progress)?;

    // For reasons I don't understand, if I don't do reopen the layer then rust thinks I retain a mutable borrow on subject_layer from the last read_features.
    // But it shouldn't, because the only thing that leaks from that call is an index of owned Points and integers.
    let mut subject_layer = ThemeType::edit_theme_layer(target)?;

    let mut segment_cache: HashMap<Coordinates, Vec<Vec<Coordinates>>> = HashMap::new();
    let read_features = ThemeType::read_theme_features(&mut subject_layer);
    let polygon_segments = break_segments::<ThemeType,_>(read_features, &vertex_index, &mut segment_cache, progress)?;

    for value in segment_cache.values_mut().watch(progress, "Curvifying line segments.", "Line segments curvified.") {
        for line in value {
            // if this is a polygon, the polybezier stuff should automatically be curving based off the end points.
            *line = bezierify_points(line,bezier_scale.bezier_scale)?;
        }
    }

    let layer = ThemeType::edit_theme_layer(target)?;

    for multipolygon in polygon_segments.map.iter().watch(progress, "Writing reshaped polygons.", "Reshaped polygons written.") {
        let mut polygons = Vec::new();
        let (fid,multipolygon) = multipolygon;
        for polygon in multipolygon {
            let mut rings = Vec::new();
            for ring in polygon {
                let mut points = Vec::new();

                for UniqueSegment { point,index,reversed } in ring {
                    let mut line = segment_cache.get(point).expect("Why wouldn't this key be here if we just inserted it?")[*index].clone();
                    if *reversed {
                        line.reverse();
                    }
                    for new_point in line {
                        points.push(new_point.to_tuple());
                    }

                }
                let ring_geometry = LinearRing::from_vertices(points)?;
                rings.push(ring_geometry);
            }
            let polygon_geometry = Polygon::from_rings(rings)?;
            let polygon_geometry = if polygon_geometry.is_valid() {
                polygon_geometry.into()
            } else {
                eprintln!("Fixing invalid polygon.");
                polygon_geometry.make_valid_structure()?
            };
            let polygon_geometry = polygon_geometry.intersection(&extent_polygon)?;
            polygons.push(polygon_geometry);
        }
        let multipolygon_geometry = MultiPolygon::from_variants(polygons)?;
        let mut feature = layer.try_feature_by_id(fid)?;
        feature.set_geometry(multipolygon_geometry)?;
        layer.update_feature(feature)?;

    }


    Ok(())
}

fn index_vertexes<'feature, ThemeType: Theme, Progress: ProgressObserver>(read_features: TypedFeatureIterator<'feature, <ThemeType as Theme>::ThemeSchema, <ThemeType as Theme>::Feature<'feature>>, progress: &mut Progress) -> Result<HashMap<Coordinates, i32>, CommandError> {
    let mut vertex_index = HashMap::new();

    for multipolygon in read_features.watch(progress,"Indexing vertexes.","Vertexes indexed.").map(|f| f.geometry()) {
        for polygon in multipolygon? {
            for ring in polygon? {
                for vertex in ring? {
                    let vertex: Coordinates = vertex.try_into()?;
                    match vertex_index.get_mut(&vertex) {
                        Some(entry) => *entry += 1,
                        None => {
                            _ = vertex_index.insert(vertex, 1);
                        },
                    }
                }
            }
        }
    }

    Ok(vertex_index)
}

struct UniqueSegment {
    point: Coordinates,
    index: usize,
    reversed: bool
}

struct BrokenSegments {
    map: HashMap<IdRef, Vec<Vec<Vec<UniqueSegment>>>>
}

fn break_segments<'feature, ThemeType: Theme, Progress: ProgressObserver>(read_features: TypedFeatureIterator<'feature, <ThemeType as Theme>::ThemeSchema, <ThemeType as Theme>::Feature<'feature>>, vertex_index: &HashMap<Coordinates, i32>, segment_cache: &mut HashMap<Coordinates, Vec<Vec<Coordinates>>>, progress: &mut Progress) -> Result<BrokenSegments, CommandError> {
    let mut polygon_segments = HashMap::new();
    for feature in read_features.watch(progress,"Breaking segments.","Segments broken.") {
        let fid = feature.fid()?;
        let multipolygon = feature.geometry()?;
        let mut polygons = Vec::new();

        // each feature is a multipolygon, so iterate through polygons
        for polygon in multipolygon {
            let mut rings = Vec::new();

            // iterate through rings on polygon
            for ring in polygon? {

                let mut segments = Vec::new();

                // convert the vertexes in the ring to points -- not that they have to be points, but they do have to be NotNaN, so this is good enough.
                let mut vertexes = ring?.into_iter().map(Coordinates::try_from)/*.map(|i| {
                    ring.get_point(i as i32).try_into()
                })*/;

                if let Some(vertex) = vertexes.next() {
                    let mut prev_vertex = vertex?;
                    let mut prev_share_count = vertex_index.get(&prev_vertex).expect("Why wouldn't this key be here if we just inserted it?");
                    let mut current_segment = vec![prev_vertex.clone()];

                    for next_vertex in vertexes {
                        let next_vertex = next_vertex?;
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
                        }
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
                                _ = segment_cache.insert(segment[0].clone(), vec![segment]);
                                0
                            },
                            Some(cache) => {
                                cache.push(segment);
                                cache.len() - 1
                            },
                        };
                        UniqueSegment {
                            point,
                            index,
                            reversed: false
                        }

                    };

                    unique_segment_ids.push(matched_id);

                }

                rings.push(unique_segment_ids);

            }

            polygons.push(rings);
        }

        _ = polygon_segments.insert(fid, polygons);
    }
    Ok(BrokenSegments {
        map: polygon_segments
    })
}

fn find_segment_match(match_segments: &[Vec<Coordinates>], segment: &[Coordinates], reverse: bool) -> Option<UniqueSegment> {
    for (index,match_segment) in match_segments.iter().enumerate() {
        if (!match_segment.is_empty()) && match_segment.len() == segment.len() {
            // search by reversed
            let matched = if reverse {
                match_segment.iter().eq(segment.iter().rev())
            } else {
                match_segment.iter().eq(segment.iter())
            };
            if matched {
                return Some(UniqueSegment {
                    point: segment.last().expect("Why wouldn't last work if we know the len > 0?").clone(),
                    index,
                    reversed: true
                });
            }
        }
    }
    None
}
