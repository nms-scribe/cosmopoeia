// FUTURE: This module was originall used to fake some implementations that were incomplete in the gdal crate. Those
// have now been implemented, so I don't need this anymore. But... just in case I need some more stuff later, this
// remains here.


/*
use gdal::errors::GdalError;
use gdal::vector::Feature;
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
                field_name: field_name.to_owned(),
                method_name: "OGR_F_GetFieldIndex",
            });
        }

        unsafe { gdal_sys::OGR_F_SetFieldNull(self.c_feature(), field_id) };
        Ok(())
    }
}
*/