use core::cmp::Ordering;

use crate::geometry::VariantArealGeometry;
use crate::geometry::LinearRing;
use crate::geometry::Polygon;
use crate::errors::CommandError;
use crate::utils::edge::Edge;
use crate::utils::point::Point;

#[derive(Clone)]
pub(crate) struct Extent {
    pub(crate) height: f64,
    pub(crate) width: f64,
    pub(crate) south: f64,
    pub(crate) west: f64,
}

impl Extent {

    pub(crate) fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        let width = east - west;
        let height = north - south;
        Self { 
            height, 
            width, 
            south, 
            west 
        }
    }

    pub(crate) const fn new_with_dimensions(west: f64, south: f64, width: f64, height: f64) -> Self {
        Self {
            height,
            width,
            south,
            west,
        }
    }

    pub(crate) fn contains(&self,point: &Point) -> bool {
        let x = point.x.into_inner();
        let y = point.y.into_inner();
        (x >= self.west) &&
           (x <= (self.west + self.width)) &&
           (y >= self.south) &&
           (y <= (self.south + self.height))

    }

    pub(crate) fn is_extent_on_edge(&self, extent: &Self) -> Result<Option<Edge>,CommandError> {
        let north = extent.north();
        let east = extent.east();
        let mut edge: Option<Edge> = None;
        for (x,y) in [(extent.west,extent.south),(extent.west,north),(east,north),(east,extent.south)] {
            if let Some(point_edge) = self.is_tuple_on_edge(x,y) {
                if let Some(previous_edge) = edge {
                    edge = Some(point_edge.combine_with(previous_edge)?);
                } else {
                    edge = Some(point_edge)
                }
            } // else keep previous edge
        }
        Ok(edge)
    }

    pub(crate) fn is_tuple_on_edge(&self, x: f64, y: f64) -> Option<Edge> {
        let x_order = if x <= self.west {
            Ordering::Less
        } else if x >= (self.east()) {
            Ordering::Greater
        } else {
            Ordering::Equal
        };

        let y_order = if y <= self.south {
            Ordering::Less
        } else if y >= (self.north()) {
            Ordering::Greater
        } else {
            Ordering::Equal
        };

        match (x_order,y_order) {
            (Ordering::Less, Ordering::Less) => Some(Edge::Southwest),
            (Ordering::Less, Ordering::Equal) => Some(Edge::West),
            (Ordering::Less, Ordering::Greater) => Some(Edge::Northwest),
            (Ordering::Equal, Ordering::Less) => Some(Edge::South),
            (Ordering::Equal, Ordering::Equal) => None,
            (Ordering::Equal, Ordering::Greater) => Some(Edge::North),
            (Ordering::Greater, Ordering::Less) => Some(Edge::Southeast),
            (Ordering::Greater, Ordering::Equal) => Some(Edge::East),
            (Ordering::Greater, Ordering::Greater) => Some(Edge::Northeast),
        }
    }

    pub(crate) fn is_off_edge(&self, point: &Point) -> Option<Edge> {
        let (x,y) = point.to_tuple();
        self.is_tuple_on_edge(x, y)

    
    }

    pub(crate) fn create_polygon(&self) -> Result<Polygon,CommandError> {
        let vertices = vec![
            (self.west,self.south),
            (self.west,self.south+self.height),
            (self.west+self.width,self.south+self.height),
            (self.west+self.width,self.south),
            (self.west,self.south),
        ];
        let ring = LinearRing::from_vertices(vertices)?;
        Polygon::from_rings([ring])
    }

    pub(crate) fn create_boundary_geometry(&self) -> Result<VariantArealGeometry, CommandError> {
        let north = self.north();
        let east = self.east();
        let west = self.west;
        let south = self.south;
        let mut border_points = Vec::new();
        border_points.push((west,south));
        for y in south.ceil() as usize..north.ceil() as usize {
            border_points.push((west,y as f64))
        }
        border_points.push((west,north));
        for x in west.ceil() as usize..east.floor() as usize {
            border_points.push((x as f64,north))
        }
        border_points.push((east,north));
        for y in north.ceil() as usize..south.floor() as usize {
            border_points.push((east,y as f64))
        }
        border_points.push((east,south));
        for x in east.ceil() as usize..west.floor() as usize {
            border_points.push((x as f64,south))
        }
        border_points.push((west,south));
        let ring = LinearRing::from_vertices(border_points)?;
        let ocean = Polygon::from_rings([ring])?;
        Ok(VariantArealGeometry::Polygon(ocean))
    }    

    pub(crate) fn east(&self) -> f64 {
        self.west + self.width
    }

    pub(crate) fn north(&self) -> f64 {
        self.south + self.height
    }

    pub(crate) fn wraps_latitudinally(&self) -> bool {
        self.width == 360.0
    }

    pub(crate) fn reaches_south_pole(&self) -> bool {
        self.south == -90.0
    }

    pub(crate) fn reaches_north_pole(&self) -> bool {
        self.north() == 90.0
    }

}
