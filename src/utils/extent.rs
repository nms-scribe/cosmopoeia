use core::cmp::Ordering;

use geo::polygon;

use crate::geometry::VariantArealGeometry;
use crate::geometry::LinearRing;
use crate::geometry::Polygon;
use crate::geometry::ChamberlainDuquetteAreaInDegrees;
use crate::errors::CommandError;
use crate::utils::edge::Edge;
use crate::utils::coordinates::Coordinates;
use crate::utils::world_shape::WorldShape;

#[derive(Clone,Debug)]
pub(crate) struct Extent {
    height: f64,
    width: f64,
    south: f64,
    west: f64,
}

impl Extent {

    pub(crate) fn from_bounds(west: f64, south: f64, east: f64, north: f64) -> Self {
        let width = east - west;
        let height = north - south;
        Self { 
            height, 
            width, 
            south, 
            west 
        }
    }


    pub(crate) const fn from_height_width_south_west(height: f64, width: f64, south: f64, west: f64) -> Self {
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

    pub(crate) fn contains(&self,point: &Coordinates) -> bool {
        let x = point.x().into_inner();
        let y = point.y().into_inner();
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

    pub(crate) fn is_off_edge(&self, point: &Coordinates) -> Option<Edge> {
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
        (self.width - 360.0).abs() < f64::EPSILON
    }

    pub(crate) fn reaches_south_pole(&self) -> bool {
        (self.south - -90.0).abs() < f64::EPSILON
    }

    pub(crate) fn reaches_north_pole(&self) -> bool {
        (self.north() - 90.0).abs() < f64::EPSILON
    }

    pub(crate) fn spherical_area(&self) -> f64 {
        let polygon = polygon![
            (x: self.west, y: self.south),
            (x: self.east(), y: self.south),
            (x: self.east(), y: self.north()),
            (x: self.west, y: self.north()),
            (x: self.west, y: self.south)
        ];
        polygon.chamberlain_duquette_area_in_degrees()
    }

    pub(crate) fn area(&self) -> f64 {
        self.width * self.height
    }

    pub(crate) fn shaped_area(&self, world_shape: &WorldShape) -> f64 {
        match world_shape {
            WorldShape::Cylinder => self.area(),
            WorldShape::Sphere => self.spherical_area()
        }

    }
    
    pub(crate) const fn height(&self) -> f64 {
        self.height
    }
    
    pub(crate) const fn width(&self) -> f64 {
        self.width
    }
    
    pub(crate) const fn south(&self) -> f64 {
        self.south
    }
    
    pub(crate) const fn west(&self) -> f64 {
        self.west
    }

}

