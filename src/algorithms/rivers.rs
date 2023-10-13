use std::collections::HashMap;
use std::rc::Rc;

use crate::world_map::tile_layer::TileForRiverConnect;
use crate::world_map::tile_layer::TileLayer;
use crate::world_map::fields::RiverSegmentTo;
use crate::world_map::fields::RiverSegmentFrom;
use crate::world_map::water_layers::NewRiver;
use crate::algorithms::beziers::bezierify_points_with_phantoms;
use crate::algorithms::beziers::find_curve_making_point;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;
use crate::commands::OverwriteRiversArg;
use crate::commands::BezierScaleArg;
use crate::world_map::fields::Neighbor;
use crate::typed_map::layers::MapLayer;
use crate::world_map::tile_layer::TileSchema;
use crate::world_map::tile_layer::TileFeature;
use crate::utils::coordinates::Coordinates;
use crate::typed_map::fields::IdRef;

pub(crate) struct RiverSegment {
    pub(crate) from: IdRef,
    pub(crate) to: Neighbor,
    pub(crate) to_flow: f64,
    pub(crate) from_lake: bool,
}

fn find_flowingest_tile(list: &Vec<Rc<RiverSegment>>) -> (Rc<RiverSegment>,f64) {
    let mut chosen_segment: Option<&Rc<RiverSegment>> = None;
    let mut total_flow = 0.0;
    for segment in list {
        total_flow += segment.to_flow;
        if let Some(potential) = chosen_segment {
            if (segment.to_flow > potential.to_flow) || (((segment.to_flow - potential.to_flow).abs() < f64::EPSILON) && segment.to > potential.to) {
                chosen_segment = Some(segment)
            }
        } else {
            chosen_segment = Some(segment)
        }
    };
    (chosen_segment.expect("Whoever called this function passed an empty list.").clone(),total_flow)
}

pub(crate) fn generate_water_rivers<Progress: ProgressObserver>(target: &mut WorldMapTransaction, bezier_scale: &BezierScaleArg, overwrite_layer: &OverwriteRiversArg, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.edit_tile_layer()?;
    let extents = tiles.get_extent()?;

    let mut segments = Vec::new();

    let segment_clean_queue = gen_water_rivers_find_segments(&mut tiles, progress)?;

    let CleanedAndIndexedSegments {tile_from_index, tile_to_index, segment_draw_queue} = generate_water_rivers_clean_and_index(segment_clean_queue, progress);

    for segment in segment_draw_queue.iter().watch(progress,"Drawing segments.","Segments drawn.") {

        let (to_type, next_tile) = generate_water_river_to_type(segment, &tile_to_index, &tile_from_index);

        let (from_type, previous_tile, from_flow) = generate_water_river_from_type(segment, &tile_from_index, &tile_to_index);

        if (from_flow == 0.0) && (segment.to_flow == 0.0) {
            continue;
        }

        let from_tile_id = segment.from.clone();
        let to_flow = segment.to_flow;
        let from_tile = tiles.try_feature_by_id(&from_tile_id)?;
        let start_point = from_tile.site()?;


        let new_river_data = match &segment.to {
            end_tile @ (Neighbor::Tile(segment_to) | Neighbor::CrossMap(segment_to, _))=> {
                let across_map = match end_tile {
                    Neighbor::Tile(_) => false,
                    Neighbor::CrossMap(_, _) => true,
                    Neighbor::OffMap(_) => unreachable!("tile matched Tile and CrossMap only"),
                };

                let to_tile = tiles.try_feature_by_id(segment_to)?;
                let from_lake = from_tile.lake_id()?;
                let to_lake = to_tile.lake_id()?;
                let to_tile_id = Neighbor::Tile(segment_to.clone());

                if from_lake.is_none() || to_lake.is_none() || from_lake != to_lake {

                    let end_point = if across_map {
                        // if we're going across the map, then the end_point needs to be converted to antimeridian
                        to_tile.site()?.across_antimeridian(&start_point)
                    } else {
                        to_tile.site()?
                    };

                    // need previous and next points to give the thingy a curve.
                    let previous_point = generate_previous_segment_point(previous_tile, &tiles, &end_point, &start_point)?;

                    let next_point = {
                        if let Some(next_tile) = next_tile {
                            match next_tile {
                                Neighbor::Tile(next_tile) => if across_map {
                                    // the next one is also across the antimeridian, so move it over here for line drawing purposes
                                    tiles.try_feature_by_id(&next_tile)?.site()?.across_antimeridian(&start_point)
                                } else {
                                    // next tile is just here...
                                    tiles.try_feature_by_id(&next_tile)?.site()?
                                },
                                Neighbor::CrossMap(next_tile,_) => if across_map {
                                    // for this one, we've crossed the map twice now, so keep this one the same.
                                    tiles.try_feature_by_id(&next_tile)?.site()?
                                } else {
                                    // Need to shift the point back across the map
                                    let neighbor_site = tiles.try_feature_by_id(&next_tile)?.site()?;
                                    neighbor_site.across_antimeridian(&end_point)
                                },
                                Neighbor::OffMap(edge) => if across_map {
                                    // This is going to be to the wrong edge, so shift it back across the map
                                    end_point.to_edge(&extents,&edge)?.across_antimeridian(&start_point)
                                } else {
                                    end_point.to_edge(&extents,&edge)?
                                },
                            }
                        } else {
                            find_curve_making_point(&start_point,&end_point)
                        }
                    };

                    Some((to_tile_id,previous_point,end_point,next_point))

                } else {
                    None
                }
            },
            Neighbor::OffMap(edge) => {
                
                let end_point = start_point.to_edge(&extents,edge)?;
                // need previous and next points to give the thingy a curve.
                let previous_point = generate_previous_segment_point(previous_tile, &tiles, &end_point, &start_point)?;
                let next_point = find_curve_making_point(&start_point,&end_point);
                
                let to_tile_id = Neighbor::OffMap(edge.clone());

                Some((to_tile_id,previous_point,end_point,next_point))
            },
            
        };

        if let Some((to_tile_id,previous_point,end_point,next_point)) = new_river_data {
            // create the bezier
            let line = bezierify_points_with_phantoms(Some(&previous_point), &[start_point,end_point], Some(&next_point), bezier_scale.bezier_scale)?;
            let lines = Coordinates::clip_point_vec_across_antimeridian(line,&extents)?;
            segments.push((NewRiver {
                from_tile_id,
                from_type,
                from_flow,
                to_tile_id,
                to_type,
                to_flow
            },lines));

        }



    }

    let mut segments_layer = target.create_rivers_layer(overwrite_layer)?;

    
    for (river,segment) in segments.into_iter().watch(progress,"Writing rivers.","Rivers written.") {
        _ = segments_layer.add_segment(&river,segment)?;
    }

    Ok(())

}

fn generate_previous_segment_point<'feature>(previous_tile: Option<IdRef>, tiles: &MapLayer<'_, 'feature, TileSchema, TileFeature<'feature>>, end_point: &Coordinates, start_point: &Coordinates) -> Result<Coordinates, CommandError> {
    Ok(if let Some(x) = previous_tile {
        tiles.try_feature_by_id(&x)?.site()?
    } else {
        find_curve_making_point(end_point,start_point)
    })
}



pub(crate) fn generate_water_river_from_type(segment: &Rc<RiverSegment>, tile_from_index: &HashMap<IdRef, Vec<Rc<RiverSegment>>>, tile_to_index: &HashMap<IdRef, Vec<Rc<RiverSegment>>>) -> (RiverSegmentFrom, Option<IdRef>, f64) {
    // a segment starts with branching if more than one segment starts at the same point.
    let branch_start_count = {
        if let Some(tile) = tile_from_index.get(&segment.from) {
            tile.len()
        } else {
            0
        }
    };


    let (from_type,previous_tile,from_flow) = if segment.from_lake {
        // the flow for these, since there is technically no beginning segment, is the same as the ending flow.
        if branch_start_count > 1 {
            (RiverSegmentFrom::BranchingLake,None,segment.to_flow)
        } else {
            (RiverSegmentFrom::Lake,None,segment.to_flow)
        }
    } else {
        match tile_to_index.get(&segment.from) {
            // I am looking for what other segments lead to the start of this segment.
            // if no other segments, then it's a plain source
            // if 1 segment, then it's continuing, Except it could be a branch if multiple others come from the same point
            // if >1 segments, then it's a confluence, But it could be a branching confluence if multiple others go to that same point
            Some(list) => match list.len() {
                0 => {
                    // even if it's branch, as there is no previous segment, its still a source
                    // much like ending with a mouth, multiple rivers could start from the same source and not be connected.
                    (RiverSegmentFrom::Source,None,0.0)
                },
                1 => {
                    let previous_tile = Some(list[0].from.clone());
                    if branch_start_count > 1 {
                        (RiverSegmentFrom::Branch,previous_tile,list[0].to_flow/branch_start_count as f64)
                    } else {
                        (RiverSegmentFrom::Continuing,previous_tile,list[0].to_flow)
                    }
                },
                _ => {
                    let (previous_tile,total_flow) = find_flowingest_tile(list);
                    let previous_tile = Some(previous_tile.from.clone());
                    if branch_start_count > 1 {
                        (RiverSegmentFrom::BranchingConfluence,previous_tile,total_flow/branch_start_count as f64)
                    } else {
                        (RiverSegmentFrom::Confluence,previous_tile,total_flow)
                    }
                }
            },
            None => (RiverSegmentFrom::Source,None,0.0),
        }
    };
    (from_type, previous_tile, from_flow)
}

pub(crate) fn generate_water_river_to_type(segment: &Rc<RiverSegment>, tile_to_index: &HashMap<IdRef, Vec<Rc<RiverSegment>>>, tile_from_index: &HashMap<IdRef, Vec<Rc<RiverSegment>>>) -> (RiverSegmentTo, Option<Neighbor>) {
    match &segment.to {
        Neighbor::Tile(segment_to) | Neighbor::CrossMap(segment_to,_) => {
            // a segment ends with a confluence if more than one segment ends at the same to point.
            let ends_with_confluence = {
                if let Some(tile) = tile_to_index.get(segment_to) {
                    tile.len() > 1
                } else {
                    false
                }
            };

            // Get start and end topological types, as well as potential previous and next tiles for curve manipulation.

            let (to_type,next_tile) = match tile_from_index.get(segment_to) {
                // I am looking for what other segments come from the end of this segment.
                // if no other segments, then it's a mouth
                // if 1 segment, then it's continuing, Except it could be a confluence if multiple others go to that same point
                // if >1 segments, then it's branching, But it could be a branching confluence if multiple others go to that same point
                Some(list) => match list.len() {
                    0 => (RiverSegmentTo::Mouth,None), // if it ends with a mouth, then it isn't a confluence even if other segments end here.
                    1 => {
                        let next_tile = Some(list[0].to.clone());
                        if ends_with_confluence {
                            (RiverSegmentTo::Confluence,next_tile)
                        } else {
                            (RiverSegmentTo::Continuing,next_tile)
                        }
                    },
                    _ => {
                        let (next_tile,_) = find_flowingest_tile(list);
                        if ends_with_confluence {
                            (RiverSegmentTo::BranchingConfluence,Some(next_tile.to.clone()))
                        } else {
                            (RiverSegmentTo::Branch,Some(next_tile.to.clone()))
                        }

                    }
                },
                None => (RiverSegmentTo::Mouth,None),
            };
            (to_type, next_tile)
        }
        Neighbor::OffMap(_) => (RiverSegmentTo::Continuing,Some(segment.to.clone())),
    }

}

struct CleanedAndIndexedSegments {
    tile_from_index: HashMap<IdRef, Vec<Rc<RiverSegment>>>, 
    tile_to_index: HashMap<IdRef, Vec<Rc<RiverSegment>>>, 
    segment_draw_queue: Vec<Rc<RiverSegment>>
}

fn generate_water_rivers_clean_and_index<Progress: ProgressObserver>(segment_clean_queue: Vec<Rc<RiverSegment>>, progress: &mut Progress) -> CleanedAndIndexedSegments {


    let mut segment_clean_queue = segment_clean_queue;
    let mut tile_from_index = HashMap::new();
    let mut tile_to_index = HashMap::new();
    let mut segment_draw_queue = Vec::new();

    // sort so that segments with the same to and from are equal, as we need to go through them in groups.
    segment_clean_queue.sort_by(|a,b| {
        if a.from == b.from {
            a.to.cmp(&b.to)
        } else {
            a.from.cmp(&b.from)
        }

    });

    let mut segment_clean_queue = segment_clean_queue.watch_queue(progress,"Cleaning and indexing segments.","Segments cleaned and indexed.");
    while let Some(segment) = segment_clean_queue.pop() {

        // look for duplicates and merge them
        if let Some(next) = segment_clean_queue.last() {
            if (segment.from == next.from) && (segment.to == next.to) {
                // we found a duplicate, pop it off and merge it.
                let duplicate = segment_clean_queue.pop().expect("Why would pop fail if we just found a value with last?");
                let merged = Rc::from(RiverSegment {
                    from: segment.from.clone(),
                    to: segment.to.clone(),
                    to_flow: segment.to_flow.max(duplicate.to_flow),
                    from_lake: segment.from_lake || duplicate.from_lake, // if one is from a lake, then both are from a lake
                });
                // put the merged back on the queue for the next processing.
                segment_clean_queue.push(merged);
                // continue, that new one will be checked and merged with the next if there are more duplicates.
                continue;
            }
        }

        // otherwise, we don't have a duplicate, let's map it and add it to the queue.
        match tile_from_index.get_mut(&segment.from) {
            None => {
                _ = tile_from_index.insert(segment.from.clone(),vec![segment.clone()]);
            },
            Some(entry) => entry.push(segment.clone()),
        };
        match &segment.to {
            Neighbor::Tile(segment_to) | Neighbor::CrossMap(segment_to,_)=> {
                match tile_to_index.get_mut(segment_to) {
                    None => {
                        _ = tile_to_index.insert(segment_to.clone(),vec![segment.clone()]);
                    },
                    Some(entry) => entry.push(segment.clone()),
                };
            }
            // don't add to 'to' index if it leads off the map. Those don't necessarily go to the same point.
            Neighbor::OffMap(_) => (),
        }
        segment_draw_queue.push(segment);

    }

    CleanedAndIndexedSegments {
        tile_from_index,
        tile_to_index,
        segment_draw_queue
    }

}

pub(crate) fn gen_water_rivers_find_segments<Progress: ProgressObserver>(tiles: &mut TileLayer<'_,'_>, progress: &mut Progress) -> Result<Vec<Rc<RiverSegment>>, CommandError> {
    let mut result = Vec::new();

    for entity in tiles.read_features().into_entities::<TileForRiverConnect>().watch(progress,"Finding segments.","Segments found.") {
        let (fid,tile) = entity?;
        for flow_to in &tile.flow_to {
            let flow_to_len = tile.flow_to.len() as f64;
            result.push(Rc::from(RiverSegment {
                from: fid.clone(),
                to: flow_to.clone(),
                to_flow: tile.water_flow/flow_to_len,
                from_lake: false,
            }))
        }
        if let Some(outlet_from) = &tile.outlet_from_id {
            // get the flow for the outlet from the current tile?
            result.push(Rc::from(RiverSegment {
                from: outlet_from.clone(),
                to: Neighbor::Tile(fid.clone()),
                to_flow: tile.water_flow,
                from_lake: true,
            }));
        }

    };

    Ok(result)
}
