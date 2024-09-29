use gdal::vector::LayerAccess;

use crate::errors::CommandError;
use crate::geometry::NoGeometry;
use crate::layer;
use crate::utils::simple_serde::Deserialize;
use crate::utils::simple_serde::Serialize;
use crate::typed_map::fields::IdRef;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;
use crate::utils::world_shape::WorldShape;

layer!(#[hide_read(true)] Property["properties"]: NoGeometry {
    #[set(allow(dead_code))] name: String,
    value: String,
});

impl PropertySchema {
    pub(crate) const PROP_ELEVATION_LIMITS: &'static str = "elevation-limits";
    pub(crate) const PROP_WORLD_SHAPE: &'static str = "world-shape";

}

pub(crate) struct ElevationLimits {
    min_elevation: f64,
    max_elevation: f64
}

impl ElevationLimits {

    pub(crate) fn new(min_elevation: f64, max_elevation: f64) -> Result<Self,CommandError> {
        if max_elevation < 0.0 {
            Err(CommandError::MaxElevationMustBePositive(max_elevation))
            // FUTURE: or should it? What if they want to create an underwater world? That won't be possible until we allow mermaid-like cultures, however,
            // and I'm not sure how "biomes" work down there.
        } else if min_elevation >= max_elevation {
            // it doesn't necessarily have to be negative, however.
            Err(CommandError::MinElevationMustBeLess(min_elevation,max_elevation))
        } else {
            Ok(Self {
                min_elevation,
                max_elevation,
            })
        }
    }
    
    pub(crate) const fn min_elevation(&self) -> f64 {
        self.min_elevation
    }
    
    pub(crate) const fn max_elevation(&self) -> f64 {
        self.max_elevation
    }
}

impl From<&ElevationLimits> for String {

    fn from(value: &ElevationLimits) -> Self {
        // store as tuple for simplicity
        (value.min_elevation,value.max_elevation).write_to_string()
    }
}

impl TryFrom<String> for ElevationLimits {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // store as tuple for simplicity
        let input: (f64,f64) = Deserialize::read_from_str(&value).map_err(|e| CommandError::InvalidPropertyValue(PropertySchema::PROP_ELEVATION_LIMITS.to_owned(),value.clone(),format!("{e}")))?;
        Ok(Self {
            min_elevation: input.0,
            max_elevation: input.1,
        })
    }
}

impl PropertyLayer<'_,'_> {

    pub(crate) fn get_property(&mut self, name: &str) -> Result<String,CommandError> {
        let mut result = None;
        for feature in TypedFeatureIterator::<PropertySchema,PropertyFeature>::from(self.layer_mut().features()) {
            if feature.name()? == name {
                result = Some(feature.value()?);
                break;
            }
        }
        // I'm not sure whether this should be done in the gdal layer or not, but it is necessary if the 
        // loop isn't completed.
        self.layer_mut().reset_feature_reading();
        result.ok_or_else(|| CommandError::PropertyNotSet(name.to_owned()))
    }

    pub(crate) fn get_elevation_limits(&mut self) -> Result<ElevationLimits,CommandError> {
        self.get_property(PropertySchema::PROP_ELEVATION_LIMITS)?.try_into()
    }

    pub(crate) fn set_property(&mut self, name: &str, value: &str) -> Result<IdRef,CommandError> {
        let mut found = None;
        for feature in TypedFeatureIterator::<PropertySchema,PropertyFeature>::from(self.layer_mut().features()) {
            if feature.name()? == name {
                found = Some(feature.fid()?);
                break;
            }
        }
        if let Some(found) = found {
            let mut feature = self.try_feature_by_id(&found)?;
            feature.set_value(&value.to_owned())?;
            self.update_feature(feature)?;
            Ok(found)
        } else {
            self.add_struct(&NewProperty { 
                name: name.to_owned(), 
                value: value.to_owned() 
            }, None)
   
        }
    }

    pub(crate) fn set_elevation_limits(&mut self, value: &ElevationLimits) -> Result<IdRef,CommandError> {
        self.set_property(PropertySchema::PROP_ELEVATION_LIMITS, &Into::<String>::into(value))
    }

    pub(crate) fn get_world_shape(&mut self) -> Result<WorldShape,CommandError> {
        self.get_property(PropertySchema::PROP_WORLD_SHAPE)?.try_into()
    }

    pub(crate) fn set_world_shape(&mut self, value: &WorldShape) -> Result<IdRef,CommandError> {
        self.set_property(PropertySchema::PROP_WORLD_SHAPE, &Into::<String>::into(value))
    }


}
