use gdal::vector::LayerAccess as _;

use crate::errors::CommandError;
use crate::geometry::Point;
use crate::geometry::Polygon;
use crate::layer;
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::TypedFeatureIterator;

layer!(#[hide_doc(true)] Point["points"]: Point {});

impl PointLayer<'_,'_> {

    pub(crate) fn add_point(&mut self, point: Point) -> Result<IdRef,CommandError> {

        self.add_struct(&NewPoint {  }, Some(point))

    }

}

layer!(#[hide_doc(true)] Triangle["triangles"]: Polygon {});

impl TriangleLayer<'_,'_> {

    pub(crate) fn add_triangle(&mut self, geo: Polygon) -> Result<IdRef,CommandError> {

        self.add_struct(&NewTriangle {  }, Some(geo))
    
    }


}
