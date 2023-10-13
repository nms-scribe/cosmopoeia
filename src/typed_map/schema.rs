use gdal::vector::OGRFieldType;

use crate::geometry::GDALGeometryWrapper;

pub(crate) trait Schema {

    type Geometry: GDALGeometryWrapper;

    const LAYER_NAME: &'static str;

    fn get_field_defs() -> &'static [(&'static str,OGRFieldType::Type)];

}
