use gdal::errors::GdalError;
use gdal::vector::Feature;
use gdal::vector::Geometry;
use gdal_sys;
use std::ffi::CString;

pub(crate) trait FeatureFix {
    fn set_field_null(&self, field_name: &str) -> Result<(),GdalError>;
}

impl FeatureFix for Feature<'_> {

    fn set_field_null(&self, field_name: &str) -> Result<(),GdalError> {

        // copied from `field_idx_from_name` because it was private
        let c_str_field_name = CString::new(field_name)?;
        let field_id = unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature(), c_str_field_name.as_ptr()) };
        if field_id == -1 {
            return Err(GdalError::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            });
        }

        unsafe { gdal_sys::OGR_F_SetFieldNull(self.c_feature(), field_id) };
        Ok(())
    }
}

pub(crate) trait GeometryFix: Sized {
    fn difference(&self, other: &Self) -> Option<Self>;
}

impl GeometryFix for Geometry {
    // FUTURE: Remove this once it's implemented in gdal itself.
    fn difference(&self, other: &Self) -> Option<Self>  {
        if !self.has_gdal_ptr() {
            return None;
        }
        if !other.has_gdal_ptr() {
            return None;
        }
        unsafe {
            let ogr_geom = gdal_sys::OGR_G_Difference(self.c_geometry(), other.c_geometry());
            if ogr_geom.is_null() {
                return None;
            }
            // Unfortunately, with_c_geometry is private, so I can't use it.
            let geometry = Self::lazy_feature_geometry();
            geometry.set_c_geometry(ogr_geom);
            // DANGER!: I can't set owned = true on the thing, there's no way.
            // However, I *think* cloning will take care of that. Because the original
            // value won't dereference the API handle, as it's not owned, but clone
            // will set it to owned.
            Some(geometry.clone())
        }
    }
}
