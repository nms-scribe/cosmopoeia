use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;

use crate::world_map::TileForRiverConnect;
use crate::world_map::TilesLayer;
use crate::world_map::RiverSegmentTo;
use crate::world_map::RiverSegmentFrom;
use crate::world_map::NewRiver;
use crate::utils::PolyBezier;
use crate::utils::find_curve_making_point;
use crate::algorithms::tiles::find_tile_site_point;
use crate::errors::CommandError;
use crate::world_map::WorldMapTransaction;
use crate::progress::ProgressObserver;
use crate::progress::WatchableIterator;
use crate::progress::WatchableQueue;

pub(crate) struct RiverSegment {
    pub(crate) from: u64,
    pub(crate) to: u64,
    pub(crate) to_flow: f64,
    pub(crate) from_lake: bool,
}

fn find_flowingest_tile(list: &Vec<Rc<RiverSegment>>) -> (Rc<RiverSegment>,f64) {
    let mut chosen_segment: Option<&Rc<RiverSegment>> = None;
    let mut total_flow = 0.0;
    for segment in list {
        total_flow += segment.to_flow;
        if let Some(potential) = chosen_segment {
            if segment.to_flow > potential.to_flow {
                chosen_segment = Some(segment)
            } else if (segment.to_flow == potential.to_flow) && segment.to > potential.to {
                // I want this algorithm to be reproducible.
                chosen_segment = Some(segment)
            }
        } else {
            chosen_segment = Some(segment)
        }
    };
    (chosen_segment.unwrap().clone(),total_flow)
}

pub(crate) fn generate_water_rivers<Progress: ProgressObserver>(target: &mut WorldMapTransaction, bezier_scale: f64, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

    let mut tiles = target.edit_tile_layer()?;

    let mut segments = Vec::new();

    let segment_clean_queue = gen_water_rivers_find_segments(&mut tiles, progress)?;

    let (tile_from_index, tile_to_index, segment_draw_queue) = generate_water_rivers_clean_and_index(segment_clean_queue, progress);

    for segment in segment_draw_queue.iter().watch(progress,"Drawing segments.","Segments drawn.") {

        let (to_type, next_tile) = generate_water_river_to_type(segment, &tile_to_index, &tile_from_index);

        let (from_type, previous_tile, from_flow) = generate_water_river_from_type(segment, &tile_from_index, &tile_to_index);

        if (from_flow == 0.0) && (segment.to_flow == 0.0) {
            continue;
        }

        if let (Some(from_tile),Some(to_tile)) = (tiles.feature_by_id(&segment.from),tiles.feature_by_id(&segment.to)) {
            let from_lake = from_tile.lake_id()?;
            let to_lake = to_tile.lake_id()?;
            if from_lake.is_none() || to_lake.is_none() || from_lake != to_lake {
                let start_point = from_tile.site()?;
                let end_point = to_tile.site()?;
                // need previous and next points to give the thingy a curve.
                let previous_point = find_tile_site_point(previous_tile, &tiles)?.or_else(|| Some(find_curve_making_point(&end_point,&start_point)));
                let next_point = find_tile_site_point(next_tile, &tiles)?.or_else(|| Some(find_curve_making_point(&start_point,&end_point)));
                // create the bezier
                let bezier = PolyBezier::from_poly_line_with_phantoms(previous_point.as_ref(),&[start_point,end_point],next_point.as_ref());
                // convert that to a polyline.
                let line = bezier.to_poly_line(bezier_scale)?;
                segments.push(NewRiver {
                    from_tile: segment.from as i64,
                    from_type,
                    from_flow: from_flow,
                    to_tile: segment.to as i64,
                    to_type,
                    to_flow: segment.to_flow,
                    line
                })

            } // I don't want to add segments that are going between tiles in the same lake. As that can create weird arms in lakes with concave sides

        }

    }

    let mut segments_layer = target.create_rivers_layer(overwrite_layer)?;

    
    for segment in segments.iter().watch(progress,"Writing rivers.","Rivers written.") {
        segments_layer.add_segment(segment)?;
    }

    Ok(())

}

pub(crate) fn generate_water_river_from_type(segment: &Rc<RiverSegment>, tile_from_index: &HashMap<u64, Vec<Rc<RiverSegment>>>, tile_to_index: &HashMap<u64, Vec<Rc<RiverSegment>>>) -> (RiverSegmentFrom, Option<u64>, f64) {
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
                    if branch_start_count > 1 {
                        // even if it's branch, as there is no previous segment, its still a source
                        (RiverSegmentFrom::Source,None,0.0)

                    } else {
                        (RiverSegmentFrom::Source,None,0.0) // much like ending with a mouth, multiple rivers could start from the same source and not be connected.
                    }
                },
                1 => {
                    let previous_tile = Some(list[0].from);
                    if branch_start_count > 1 {
                        (RiverSegmentFrom::Branch,previous_tile,list[0].to_flow/branch_start_count as f64)
                    } else {
                        (RiverSegmentFrom::Continuing,previous_tile,list[0].to_flow)
                    }
                },
                _ => {
                    let (previous_tile,total_flow) = find_flowingest_tile(list);
                    let previous_tile = Some(previous_tile.from);
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

pub(crate) fn generate_water_river_to_type(segment: &Rc<RiverSegment>, tile_to_index: &HashMap<u64, Vec<Rc<RiverSegment>>>, tile_from_index: &HashMap<u64, Vec<Rc<RiverSegment>>>) -> (RiverSegmentTo, Option<u64>) {
    // a segment ends with a confluence if more than one segment ends at the same to point.
    let ends_with_confluence = {
        if let Some(tile) = tile_to_index.get(&segment.to) {
            tile.len() > 1
        } else {
            false
        }
    };

    // Get start and end topological types, as well as potential previous and next tiles for curve manipulation.

    let (to_type,next_tile) = match tile_from_index.get(&segment.to) {
        // I am looking for what other segments come from the end of this segment.
        // if no other segments, then it's a mouth
        // if 1 segment, then it's continuing, Except it could be a confluence if multiple others go to that same point
        // if >1 segments, then it's branching, But it could be a branching confluence if multiple others go to that same point
        Some(list) => match list.len() {
            0 => (RiverSegmentTo::Mouth,None), // if it ends with a mouth, then it isn't a confluence even if other segments end here.
            1 => {
                let next_tile = Some(list[0].to);
                if ends_with_confluence {
                    (RiverSegmentTo::Confluence,next_tile)
                } else {
                    (RiverSegmentTo::Continuing,next_tile)
                }
            },
            _ => {
                let (next_tile,_) = find_flowingest_tile(list);
                if ends_with_confluence {
                    (RiverSegmentTo::BranchingConfluence,Some(next_tile.to))
                } else {
                    (RiverSegmentTo::Branch,Some(next_tile.to))
                }

            }
        },
        None => (RiverSegmentTo::Mouth,None),
    };
    (to_type, next_tile)
}

pub(crate) fn generate_water_rivers_clean_and_index<Progress: ProgressObserver>(segment_clean_queue: Vec<Rc<RiverSegment>>, progress: &mut Progress) -> (HashMap<u64, Vec<Rc<RiverSegment>>>, HashMap<u64, Vec<Rc<RiverSegment>>>, Vec<Rc<RiverSegment>>) {


    let mut segment_clean_queue = segment_clean_queue;
    let mut tile_from_index = HashMap::new();
    let mut tile_to_index = HashMap::new();
    let mut result_queue = Vec::new();

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
                let next = segment_clean_queue.pop().unwrap();
                let merged = Rc::from(RiverSegment {
                    from: segment.from,
                    to: segment.to,
                    to_flow: segment.to_flow.max(next.to_flow),
                    from_lake: segment.from_lake || next.from_lake, // if one is from a lake, then both are from a lake
                });
                // put the merged back on the queue for the next processing.
                segment_clean_queue.push(merged);
                // continue, that new one will be checked and merged with the next if there are more duplicates.
                continue;
            }
        }

        // otherwise, we don't have a duplicate, let's map it and add it to the queue.
        match tile_from_index.entry(segment.from) {
            Entry::Vacant(entry) => {
                entry.insert(vec![segment.clone()]);
            },
            Entry::Occupied(mut entry) => {
                let list = entry.get_mut();
                list.push(segment.clone());
            },
        };
        match tile_to_index.entry(segment.to) {
            Entry::Vacant(entry) => {
                entry.insert(vec![segment.clone()]);
            },
            Entry::Occupied(mut entry) => {
                let list = entry.get_mut();
                list.push(segment.clone());
            },
        };
        result_queue.push(segment);

    }

    (tile_from_index, tile_to_index, result_queue)
}

pub(crate) fn gen_water_rivers_find_segments<Progress: ProgressObserver>(tiles: &mut TilesLayer<'_,'_>, progress: &mut Progress) -> Result<Vec<Rc<RiverSegment>>, CommandError> {
    let mut result = Vec::new();

    for entity in tiles.read_features().into_entities::<TileForRiverConnect>().watch(progress,"Finding segments.","Segments found.") {
        let (fid,tile) = entity?;
        for flow_to in &tile.flow_to {
            let flow_to_len = tile.flow_to.len() as f64;
            result.push(Rc::from(RiverSegment {
                from: fid,
                to: *flow_to,
                to_flow: tile.water_flow/flow_to_len,
                from_lake: false,
            }))
        }
        for outlet_from in &tile.outlet_from {
            // get the flow for the outlet from the current tile?
            result.push(Rc::from(RiverSegment {
                from: *outlet_from,
                to: fid,
                to_flow: tile.water_flow,
                from_lake: true,
            }));
        }

    };

    Ok(result)
}
