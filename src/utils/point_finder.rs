// FUTURE: This was an implementation I found on crates.io that allowed inserting and floating point points, and wasn't too difficult to construct. Although that could be done better. It didn't have a lot of downloads, however, so I don't know if it's really something I should be using. The alternatives were lacking features I needed.
use qutee::QuadTree; 
use qutee::Boundary;

use super::extent::Extent;
use super::coordinates::Coordinates;
use crate::errors::CommandError;
use crate::typed_map::fields::IdRef;

pub(crate) struct PointFinder {
  // It's kind of annoying, but the query method doesn't return the original point, so I have to store the point.
  inner: QuadTree<f64,Coordinates>,
  bounds: Boundary<f64>, // it also doesn't give us access to this, which is useful for cloning
  capacity: usize // or this
}

impl PointFinder {

    pub(crate) fn new(extent: &Extent, capacity: usize) -> Self {
        let bounds = Boundary::between_points((extent.west,extent.south),(extent.east(),extent.north()));
        Self {
            inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
            bounds,
            capacity
        }
    }

    pub(crate) fn add_point(&mut self, point: Coordinates) -> Result<(),CommandError> {
        self.inner.insert_at(point.to_tuple(),point).map_err(|e|  {
            match e {
                qutee::QuadTreeError::OutOfBounds(_, qutee::Point { x, y }) => CommandError::PointFinderOutOfBounds(x,y),
            }
            
        })

    }

    pub(crate) fn points_in_target(&mut self, point: &Coordinates, spacing: f64) -> bool {
        let west = point.x - spacing;
        let south = point.y - spacing;
        let north = point.x + spacing;
        let east = point.y + spacing;
        let boundary = Boundary::between_points((west.into(),south.into()),(east.into(),north.into()));
        for item in self.inner.query(boundary) {
            if item.distance(point) <= spacing {
                return true;
            }
        }
        false

    }

    pub(crate) fn fill_from(other: &Self, additional_size: usize) -> Result<Self,CommandError> {
        let bounds = other.bounds.clone();
        let capacity = other.capacity + additional_size;
        let mut result = Self {
            inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
            bounds,
            capacity
        };
        for point in other.inner.iter() {
            result.add_point(point.clone())?
        }
        Ok(result)
    }
}

pub(crate) struct TileFinder {
  inner: QuadTree<f64,(Coordinates,IdRef)>, // I need the original point to test distance
  bounds: Boundary<f64>, // see PointFinder
  //capacity: usize, // see PointFinder
  initial_search_radius: f64
}

impl TileFinder {

    pub(crate) fn new(extent: &Extent, capacity: usize, tile_spacing: f64) -> Self {
        let bounds = Boundary::between_points((extent.west,extent.south),(extent.east(),extent.north()));
        Self {
            inner: QuadTree::new_with_dyn_cap(bounds.clone(),capacity),
            bounds,
            //capacity,
            initial_search_radius: tile_spacing
        }
    }

    pub(crate) fn add_tile(&mut self, point: Coordinates, tile: IdRef) -> Result<(),CommandError> {
        self.inner.insert_at(point.to_tuple(),(point,tile)).map_err(|e|  {
            match e {
                qutee::QuadTreeError::OutOfBounds(_, qutee::Point { x, y }) => CommandError::PointFinderOutOfBounds(x,y),
            }
            
        })

    }

    pub(crate) fn find_nearest_tile(&self, point: &Coordinates) -> Result<IdRef,CommandError> {
        let mut spacing = self.initial_search_radius;

        macro_rules! calc_search_boundary {
            () => {
                {
                    let west = point.x - spacing;
                    let south = point.y - spacing;
                    let north = point.x + spacing;
                    let east = point.y + spacing;
                    Boundary::between_points((west.into(),south.into()),(east.into(),north.into()))
                }
            };
        }

        let mut search_boundary = calc_search_boundary!();

        macro_rules! find_tile {
            () => {
                let mut found = None;
                for item in self.inner.query(search_boundary) {
                    match &found {
                        None => found = Some((item.1.clone(),item.0.distance(point))),
                        Some(last_found) => {
                            let this_distance = item.0.distance(point);
                            if this_distance < last_found.1 {
                                found = Some((item.1.clone(),this_distance))
                            }
                        },
                    }
                }
                if let Some((tile,_)) = found {
                    return Ok(tile)
                }                        
            };
        }

        for _ in 0..10 { // try ten times at incrementing radiuses before giving up and searching the whole index. If they still haven't found one by then it's probably an empty tile board.
            find_tile!();
            // double the spacing and keep searching
            spacing *= 2.0;
            search_boundary = calc_search_boundary!();
        }
        // just search over the whole thing:
        search_boundary = self.bounds.clone();
        find_tile!();
        // okay, nothing was found, this is an error.
        Err(CommandError::CantFindTileNearPoint)

    }

}
