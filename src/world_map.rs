use std::path::Path;
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;

use gdal::DriverManager;
use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use gdal::LayerOptions;
use gdal::vector::LayerAccess;
use gdal::vector::OGRwkbGeometryType;
use gdal::vector::OGRFieldType;
use gdal::vector::FieldValue;
use gdal::vector::Geometry;
use gdal::vector::Layer;
use gdal::vector::Feature;
use gdal::vector::FeatureIterator;
use gdal::Transaction;

use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::utils::LayerGeometryIterator;
use crate::utils::Point;
use crate::utils::create_line;
use crate::raster::RasterMap;
use crate::algorithms::OceanSamplingMethod;
use crate::algorithms::sample_elevations;
use crate::algorithms::sample_ocean;
use crate::algorithms::calculate_neighbors;
use crate::algorithms::generate_temperatures;
use crate::algorithms::generate_winds;
use crate::algorithms::generate_precipitation;
use crate::algorithms::generate_water_flow;
use crate::algorithms::generate_water_fill;
use crate::algorithms::generate_water_rivers;


// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.


pub(crate) struct WorldLayer<'lifetime> {
    layer: Layer<'lifetime>
}

impl<'lifetime> WorldLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset, name: &str) -> Result<Self,CommandError> {
        let layer = dataset.layer_by_name(name)?;
        Ok(Self {
            layer
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, name: &str, geometry_type: OGRwkbGeometryType::Type, field_defs: Option<&[(&str, OGRFieldType::Type)]>, overwrite: bool) -> Result<Self,CommandError> {
        let layer = dataset.create_layer(LayerOptions {
            name,
            ty: geometry_type,
            options: if overwrite {
                Some(&["OVERWRITE=YES"])
            } else {
                None
            },
            ..Default::default()
        })?;
        if let Some(field_defs) = field_defs {
            layer.create_defn_fields(field_defs)?;
        }
        // NOTE: I'm specifying the field value as real for now. Eventually I might want to allow it to choose a type based on the raster type, but there
        // really isn't much choice, just three numeric types (int, int64, and real)

        Ok(Self {
            layer
        })
    }

    fn add(&mut self, geometry: Geometry, field_names: &[&str], field_values: &[FieldValue]) -> Result<(),CommandError> {
        // I dug out the source to get this. I wanted to be able to return the feature being created.
        let mut feature = gdal::vector::Feature::new(self.layer.defn())?;
        feature.set_geometry(geometry)?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            feature.set_field(&field, value)?;
        }
        feature.create(&self.layer)?;
        Ok(())
    }

    fn add_without_geometry(&mut self, field_names: &[&str], field_values: &[FieldValue]) -> Result<(),CommandError> {
        // This function is used for lookup tables, like biomes.

        // I had to dig into the source to get this stuff...
        let feature = gdal::vector::Feature::new(self.layer.defn())?;
        for (field, value) in field_names.iter().zip(field_values.iter()) {
            feature.set_field(field, value)?;
        }
        feature.create(&self.layer)?;
        Ok(())

    }


    fn read_geometries(&mut self) -> LayerGeometryIterator {
        LayerGeometryIterator::new(&mut self.layer)

    }




}

pub(crate) struct PointsLayer<'lifetime> {
    points: WorldLayer<'lifetime>
}

impl<'lifetime> PointsLayer<'lifetime> {

    const LAYER_NAME: &str = "points";
    

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let points = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            points
        })
    }

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let points = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbPoint, None, overwrite)?;

        Ok(Self {
            points
        })
    }

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.points.add(point,&[],&[])?;
        Ok(())

    }

    pub(crate) fn read_points(&mut self) -> LayerGeometryIterator {
        self.points.read_geometries()

    }


}



pub(crate) struct TrianglesLayer<'lifetime> {
    tiles: WorldLayer<'lifetime>
}

impl<'lifetime> TrianglesLayer<'lifetime> {

    const LAYER_NAME: &str = "triangles";


    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbPolygon, None, overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.tiles.add(geo,&[],&[])?;
        Ok(())

    }


    pub(crate) fn read_triangles(&mut self) -> LayerGeometryIterator {
        self.tiles.read_geometries()

    }



}

#[derive(Clone)]
pub(crate) enum RiverSegmentFrom {
    Source,
    Lake,
    Branch,
    Continuing,
    BranchingLake,
    BranchingConfluence,
    Confluence,
}

impl TryFrom<String> for RiverSegmentFrom {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "source" => Ok(Self::Source),
            "lake" => Ok(Self::Lake),
            "branch" => Ok(Self::Branch),
            "continuing" => Ok(Self::Continuing),
            "lake-branch" => Ok(Self::BranchingLake),
            "branch-confluence" => Ok(Self::BranchingConfluence),
            "confluence" => Ok(Self::Confluence),
            a => Err(CommandError::InvalidValueForSegmentFrom(a.to_owned()))
        }
    }
}

impl Into<&str> for &RiverSegmentFrom {

    fn into(self) -> &'static str {
        match self {
            RiverSegmentFrom::Source => "source",
            RiverSegmentFrom::Lake => "lake",
            RiverSegmentFrom::Branch => "branch",
            RiverSegmentFrom::Continuing => "continuing",
            RiverSegmentFrom::BranchingLake => "lake-branch",
            RiverSegmentFrom::BranchingConfluence => "branch-confluence",
            RiverSegmentFrom::Confluence => "confluence",
        }
    }
}

#[derive(Clone)]
pub(crate) enum RiverSegmentTo {
    Mouth,
    Confluence,
    Continuing,
    Branch,
    BranchingConfluence,
}

impl TryFrom<String> for RiverSegmentTo {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "mouth" => Ok(Self::Mouth),
            "confluence" => Ok(Self::Confluence),
            "continuing" => Ok(Self::Continuing),
            "branch" => Ok(Self::Branch),
            "branch-confluence" => Ok(Self::BranchingConfluence),
            a => Err(CommandError::InvalidValueForSegmentTo(a.to_owned()))
        }
    }
}

impl Into<&str> for &RiverSegmentTo {

    fn into(self) -> &'static str {
        match self {
            RiverSegmentTo::Mouth => "mouth",
            RiverSegmentTo::Confluence => "confluence",
            RiverSegmentTo::Continuing => "continuing",
            RiverSegmentTo::Branch => "branch",
            RiverSegmentTo::BranchingConfluence => "branch-confluence",
        }
    }
}




macro_rules! feature {
    (bool_to_int@ $value: ident) => {
        if $value { 1 } else { 0 }
    };
    (id_list_to_string@ $value: ident) => {
        $value.iter().map(|fid| format!("{}",fid)).collect::<Vec<String>>().join(",")
    };
    (neighbor_directions_to_string@ $value: ident) => {
        $value.iter().map(|(fid,dir)| format!("{}:{}",fid,dir)).collect::<Vec<String>>().join(",")
    };
    (getfieldtype@ f64) => {
        f64
    };
    (getfieldtype@ i64) => {
        i64
    };
    (getfieldtype@ i32) => {
        i32
    };
    (getfieldtype@ bool) => {
        bool
    };
    (getfieldtype@ option_f64) => {
        f64 // this is the same because everything's an option, the option tag only means it can accept options
    };
    (getfieldtype@ neighbor_directions) => {
        Vec<(u64,i32)>
    };
    (getfieldtype@ id_list) => {
        Vec<u64>
    };
    (getfieldtype@ river_segment_from) => {
        RiverSegmentFrom
    };
    (getfieldtype@ river_segment_to) => {
        RiverSegmentTo
    };
    (getfieldtype@ string) => {
        String
    };
    (getfieldtype@ biome_criteria) => {
        BiomeCriteria
    };
    (setfieldtype@ f64) => {
        f64
    };
    (setfieldtype@ option_f64) => {
        Option<f64>
    };
    (setfieldtype@ i64) => {
        i64
    };
    (setfieldtype@ i32) => {
        i32
    };
    (setfieldtype@ bool) => {
        bool
    };
    (setfieldtype@ neighbor_directions) => {
        &Vec<(u64,i32)>
    };
    (setfieldtype@ id_list) => {
        &Vec<u64>
    };
    (setfieldtype@ river_segment_from) => {
        &RiverSegmentFrom
    };
    (setfieldtype@ river_segment_to) => {
        &RiverSegmentTo
    };
    (setfieldtype@ string) => {
        &str
    };
    (setfieldtype@ biome_criteria) => {
        &BiomeCriteria
    };
    (getfield@ $self: ident f64 $field: path) => {
        Ok($self.feature.field_as_double_by_name($field)?)
    };
    (getfield@ $self: ident option_f64 $field: path) => {
        // see above for getfieldtype option_f64
        Ok($self.feature.field_as_double_by_name($field)?)
    };
    (getfield@ $self: ident i64 $field: path) => {
        Ok($self.feature.field_as_integer64_by_name($field)?)
    };
    (getfield@ $self: ident i32 $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?)
    };
    (getfield@ $self: ident bool $field: path) => {
        Ok($self.feature.field_as_integer_by_name($field)?.map(|n| n != 0))
    };
    (getfield@ $self: ident neighbor_directions $field: path) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name($field)? {
            Ok(Some(neighbors.split(',').filter_map(|a| {
                let mut a = a.splitn(2, ':');
                if let Some(neighbor) = a.next().map(|n| n.parse().ok()).flatten() {
                    if let Some(direction) = a.next().map(|d| d.parse().ok()).flatten() {
                        if direction >= 0 {
                            Some((neighbor,direction))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
                
            }).collect()))
        } else {
            Ok(Some(Vec::new()))
        }

    };
    (getfield@ $self: ident id_list $field: path) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name($field)? {
            Ok(Some(neighbors.split(',').filter_map(|a| {
                a.parse().ok()
            }).collect()))
        } else {
            Ok(Some(Vec::new()))
        }

    };
    (getfield@ $self: ident river_segment_from $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(Some(RiverSegmentFrom::try_from(value)?))
        } else {
            Ok(None)
        }

    };
    (getfield@ $self: ident river_segment_to $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(Some(RiverSegmentTo::try_from(value)?))
        } else {
            Ok(None)
        }

    };
    (getfield@ $self: ident string $field: path) => {
        Ok($self.feature.field_as_string_by_name($field)?)
    };
    (getfield@ $self: ident biome_criteria $field: path) => {
        if let Some(value) = $self.feature.field_as_string_by_name($field)? {
            Ok(Some(BiomeCriteria::try_from(value)?))
        } else {
            Ok(None)
        }

    };
    (setfield@ $self: ident $value: ident f64 $field: path) => {
        Ok($self.feature.set_field_double($field, $value)?)
    };
    (setfield@ $self: ident $value: ident option_f64 $field: path) => {
        if let Some(value) = $value {
            Ok($self.feature.set_field_double($field, value)?)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            Ok($self.feature.set_field_double($field,f64::NAN)?)
        }
    };
    (setfield@ $self: ident $value: ident i32 $field: path) => {
        Ok($self.feature.set_field_integer($field, $value)?)
    };
    (setfield@ $self: ident $value: ident i64 $field: path) => {
        Ok($self.feature.set_field_integer64($field, $value)?)
    };
    (setfield@ $self: ident $value: ident bool $field: path) => {
        Ok($self.feature.set_field_integer($field, feature!(bool_to_int@ $value))?)
    };
    (setfield@ $self: ident $value: ident neighbor_directions $field: path) => {{
        let neighbors = feature!(neighbor_directions_to_string@ $value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    (setfield@ $self: ident $value: ident id_list $field: path) => {{
        let neighbors = feature!(id_list_to_string@ $value);
        Ok($self.feature.set_field_string($field, &neighbors)?)
    }};
    (setfield@ $self: ident $value: ident river_segment_from $field: path) => {{
        Ok($self.feature.set_field_string($field, $value.into())?)
    }};
    (setfield@ $self: ident $value: ident river_segment_to $field: path) => {{
        Ok($self.feature.set_field_string($field, $value.into())?)
    }};
    (setfield@ $self: ident $value: ident string $field: path) => {{
        Ok($self.feature.set_field_string($field, $value)?)
    }};
    (setfield@ $self: ident $value: ident biome_criteria $field: path) => {{
        Ok($self.feature.set_field_string($field, &Into::<String>::into($value))?)
    }};
    (to_value@ $prop: ident f64) => {
        FieldValue::RealValue($prop)
    };
    (to_value@ $prop: ident i32) => {
        FieldValue::IntegerValue($prop)
    };
    (to_value@ $prop: ident bool) => {
        FieldValue::IntegerValue(feature!(bool_to_int@ $prop))
    };
    (to_value@ $prop: ident option_f64) => {
        if let Some(value) = $prop {
            FieldValue::RealValue(value)
        } else {
            // There's no unsetfield, but this should have the same effect.
            // FUTURE: I've put in a feature request to gdal crate.
            FieldValue::RealValue(f64::NAN)
        }
    };
    (to_value@ $prop: ident id_list) => {
        FieldValue::StringValue(feature!(id_list_to_string@ $prop))
    };
    (to_value@ $prop: ident neighbor_directions) => {
        FieldValue::StringValue(feature!(neighbor_directions_to_string@ $prop))
    };
    (to_value@ $prop: ident i64) => {
        FieldValue::Integer64Value($prop)
    };
    (to_value@ $prop: ident river_segment_from) => {{
        FieldValue::StringValue(Into::<&str>::into($prop).to_owned())
    }};
    (to_value@ $prop: ident river_segment_to) => {{
        FieldValue::StringValue(Into::<&str>::into($prop).to_owned())
    }};
    (to_value@ $prop: ident string) => {{
        FieldValue::StringValue($prop.to_owned())
    }};
    (to_value@ $prop: ident biome_criteria) => {{
        FieldValue::StringValue(Into::<String>::into($prop))
    }};
    (get@ $(#[$attr: meta])* $prop: ident $type: ident $field: path) => {
        $(#[$attr])* pub(crate) fn $prop(&self) -> Result<Option<feature!(getfieldtype@ $type)>,CommandError> {
            feature!(getfield@ self $type $field)
        }
    };
    (set@ $(#[$attr: meta])* $set_prop: ident $type: ident $field: path) => {
        $(#[$attr])* pub(crate) fn $set_prop(&mut self, value: feature!(setfieldtype@ $type)) -> Result<(),CommandError> {
            feature!(setfield@ self value $type $field)
        }            
    };
    (props@ $(#[$get_attr: meta])* $prop: ident $(#[$set_attr: meta])* $set_prop: ident $type: ident $field: path) => {
        feature!(get@ $(#[$get_attr])* $prop $type $field);

        feature!(set@ $(#[$set_attr])* $set_prop $type $field);

    };
    ($field_count: literal; $(fid: #[$fid_attr: meta])? $(geometry: #[$geometry_attr: meta])? $(to_values: #[$to_values_attr: meta])? {$($(#[$get_attr: meta])* $prop: ident $(#[$set_attr: meta])* $set_prop: ident $prop_type: ident $field: ident $name: literal $field_type: path;)*}) => {

        $(pub(crate) const $field: &str = $name;)*

        const FIELD_DEFS: [(&str,OGRFieldType::Type); $field_count] = [
            $((Self::$field,$field_type)),*
        ];

        $(#[$fid_attr])? pub(crate) fn fid(&self) -> Option<u64> {
            self.feature.fid()
        }
    
        $(#[$geometry_attr])? pub(crate) fn geometry(&self) -> Option<&Geometry> {
            self.feature.geometry()
        }

        $(#[$to_values_attr])? pub(crate) fn to_field_names_values($($prop: feature!(setfieldtype@ $prop_type)),*) -> ([&'static str; $field_count],[FieldValue; $field_count]) {
            ([
                $(Self::$field),*
            ],[
                $(feature!(to_value@ $prop $prop_type)),*
            ])

        }
    
        $(
            feature!(props@ $(#[$get_attr])* $prop $(#[$set_attr])* $set_prop $prop_type Self::$field);
        )*
    };
}


pub(crate) struct TypedFeatureIterator<'lifetime, TypedFeature: From<Feature<'lifetime>>> {
    features: FeatureIterator<'lifetime>,
    _phantom: std::marker::PhantomData<TypedFeature>
}

impl<'lifetime, TypedFeature: From<Feature<'lifetime>>> Iterator for TypedFeatureIterator<'lifetime, TypedFeature> {
    type Item = TypedFeature;

    fn next(&mut self) -> Option<Self::Item> {
        self.features.next().map(TypedFeature::from)
    }
}

impl<'lifetime, TypedFeature: From<Feature<'lifetime>>> From<FeatureIterator<'lifetime>> for TypedFeatureIterator<'lifetime, TypedFeature> {
    fn from(features: FeatureIterator<'lifetime>) -> Self {
        Self {
            features,
            _phantom: std::marker::PhantomData::default()
        }
    }
}

#[macro_export]
macro_rules! entity {
    (variables@ $feature: ident, $($field: ident),*) => {
        $(
            let $field = $feature.$field()?;
        )*
    };
    (fieldassign@ $feature: ident geometry $type: ty) => {
        $feature.geometry().cloned().ok_or_else(|| CommandError::MissingGeometry)?
    };
    (fieldassign@ $feature: ident fid $type: ty) => {
        $feature.fid().ok_or_else(|| CommandError::MissingField("fid"))?
    };
    (fieldassign@ $feature: ident $field: ident $type: ty) => {
        $feature.$field()?.ok_or_else(|| CommandError::MissingField(stringify!($field)))?
    };
    (fieldassign@ $feature: ident $field: ident $type: ty = $function: expr) => {
        $function(&$feature)?
    };
    (constructor@ $name: ident  $feature: ident $($field: ident: $type: ty $(= $function: expr)?),*) => {
        Ok($name {
            $(
                $field: entity!(fieldassign@ $feature $field $type $(= $function)?)
            ),*
        })

    };
    (from_data@ $name: ident $feature: ident, $($field: ident: $type: ty $(= $function: expr)?),*) => {{
        entity!(constructor@ $name $feature $($field: $type $(= $function)? ),*)
    }};
    (fielddef@ $type: ty [$function: expr]) => {
        $type
    };
    (fielddef@ $type: ty) => {
        $type
    };
    (feature_class@ TileEntity) => {
        TileFeature
    };
    (feature_class@ RiverEntity) => {
        RiverFeature
    };
    (feature_class@ LakeEntity) => {
        LakeFeature
    };
    (feature_class@ BiomeEntity) => {
        BiomeFeature
    };
    ($name: ident $entity_class: ident {$($field: ident: $type: ty $(= $function: expr)?),*}) => {
        #[derive(Clone)]
        pub(crate) struct $name {
            $(
                pub(crate) $field: entity!(fielddef@ $type $([$function])?)
            ),*
        }

        impl $entity_class for $name {

            fn try_from_feature(feature: entity!(feature_class@ $entity_class)) -> Result<Self,CommandError> {
                entity!(from_data@ $name feature, $($field: $type $(= $function)?),*)
            }

        }

    };
}

pub(crate) struct TileFeature<'lifetime> {

    feature: Feature<'lifetime>
}

impl<'lifetime> From<Feature<'lifetime>> for TileFeature<'lifetime> {

    fn from(feature: Feature<'lifetime>) -> Self {
        Self {
            feature
        }
    }
}

impl<'lifetime> TileFeature<'lifetime> {

    feature!(14; to_values: #[allow(dead_code)] {
        site_x #[allow(dead_code)] set_site_x f64 FIELD_SITE_X "site_x" OGRFieldType::OFTReal;
        site_y #[allow(dead_code)] set_site_y f64 FIELD_SITE_Y "site_y" OGRFieldType::OFTReal;
        elevation set_elevation f64 FIELD_ELEVATION "elevation" OGRFieldType::OFTReal;
        // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
        // If I ever get rid of those algorithms, this field can go away.
        elevation_scaled set_elevation_scaled i32 FIELD_ELEVATION_SCALED "elevation_scaled" OGRFieldType::OFTInteger;
        is_ocean set_is_ocean bool FIELD_IS_OCEAN "is_ocean" OGRFieldType::OFTInteger;
        temperature set_temperature f64 FIELD_TEMPERATURE "temperature" OGRFieldType::OFTReal;
        wind set_wind i32 FIELD_WIND "wind_dir" OGRFieldType::OFTInteger;
        precipitation set_precipitation f64 FIELD_PRECIPITATION "precipitation" OGRFieldType::OFTReal;
        #[allow(dead_code)] water_flow set_water_flow f64 FIELD_WATER_FLOW "water_flow" OGRFieldType::OFTReal;
        #[allow(dead_code)] water_accumulation set_water_accumulation f64 FIELD_WATER_ACCUMULATION "water_accum" OGRFieldType::OFTReal;
        #[allow(dead_code)] lake_elevation set_lake_elevation option_f64 FIELD_LAKE_ELEVATION "lake_elev" OGRFieldType::OFTReal;
        #[allow(dead_code)] flow_to set_flow_to id_list FIELD_FLOW_TO "flow_to" OGRFieldType::OFTString;
        // NOTE: This field should only ever have one value or none. However, as I have no way of setting None
        // on a u64 field (until gdal is updated to give me access to FieldSetNone), I'm going to use a vector
        // to store it. In any way, you never know when I might support outlet from multiple points.
        #[allow(dead_code)] outlet_from set_outlet_from id_list FIELD_OUTLET_FROM "outlet_from" OGRFieldType::OFTString;
        neighbors set_neighbors neighbor_directions FIELD_NEIGHBOR_TILES "neighbor_tiles" OGRFieldType::OFTString;

    });

    pub(crate) fn site_point(&self) -> Result<Point,CommandError> {
        if let (Some(x),Some(y)) = (self.site_x()?,self.site_y()?) {
            Ok(Point::try_from((x,y))?)
        } else {
            Err(CommandError::MissingField("site"))
        }
    }

}

// NOTE: I tried using a TryFrom, but because TileFeature requires a lifetime, I had to add that in as well, and it started to propagate. 
// This is a much easier version of the same thing.
pub(crate) trait TileEntity: Sized {

    fn try_from_feature(feature: TileFeature) -> Result<Self,CommandError>;

}

pub(crate) trait TileEntityWithNeighborsElevation {

    fn neighbors(&self) -> &Vec<(u64,i32)>;

    fn elevation(&self) -> &f64;
}


entity!(NewTileEntity TileEntity {
    geometry: Geometry,
    site_x: f64, 
    site_y: f64
}); 
entity!(TileEntitySite TileEntity {
    fid: u64, 
    site_x: f64, 
    site_y: f64
});
entity!(TileEntitySiteGeo TileEntity {
    fid: u64, 
    geometry: Geometry, 
    site_x: f64, 
    site_y: f64
});
entity!(TileEntityLatElevOcean TileEntity {
    fid: u64, 
    site_y: f64, 
    elevation: f64, 
    is_ocean: bool
});
entity!(TileEntityLat TileEntity {
    fid: u64, 
    site_y: f64
});
entity!(TileEntityForWaterFlow TileEntity {
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    precipitation: f64,
    water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
    water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    flow_to: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new())
});

impl TileEntityWithNeighborsElevation for TileEntityForWaterFlow {

    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}

// Basically the same struct as WaterFlow, except that the fields are initialized differently. I can't
// just use a different function because it's based on a trait. I could take this one out
// of the macro and figure something out, but this is easier.
entity!(TileEntityForWaterFill TileEntity {
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    water_flow: f64,
    water_accumulation: f64,
    flow_to: Vec<u64>,
    outlet_from: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new()),
    lake_id: Option<usize> = |_| Ok::<_,CommandError>(None)
});

entity!(TileEntityForRiverConnect TileEntity {
    water_flow: f64,
    flow_to: Vec<u64>,
    outlet_from: Vec<u64>
});

impl From<TileEntityForWaterFlow> for TileEntityForWaterFill {

    fn from(value: TileEntityForWaterFlow) -> Self {
        Self {
            elevation: value.elevation,
            is_ocean: value.is_ocean,
            neighbors: value.neighbors,
            water_flow: value.water_flow,
            water_accumulation: value.water_accumulation,
            flow_to: value.flow_to,
            outlet_from: Vec::new(),
            lake_id: None
        }
    }
}

impl TileEntityWithNeighborsElevation for TileEntityForWaterFill {

    fn neighbors(&self) -> &Vec<(u64,i32)> {
        &self.neighbors
    }

    fn elevation(&self) -> &f64 {
        &self.elevation
    }
}


pub(crate) struct TileEntityIterator<'lifetime, Data: TileEntity> {
    features: TypedFeatureIterator<'lifetime,TileFeature<'lifetime>>,
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'lifetime,Data: TileEntity> Iterator for TileEntityIterator<'lifetime,Data> {
    type Item = Result<(u64,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from_feature(feature)) {
                (Some(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (None, Ok(_)) => Some(Err(CommandError::MissingField("fid"))),
                (_, Err(e)) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

impl<'lifetime,Data: TileEntity> From<TypedFeatureIterator<'lifetime,TileFeature<'lifetime>>> for TileEntityIterator<'lifetime,Data> {
    fn from(features: TypedFeatureIterator<'lifetime,TileFeature<'lifetime>>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}



pub(crate) struct TilesLayer<'lifetime> {
    tiles: WorldLayer<'lifetime>
}

impl<'lifetime> TilesLayer<'lifetime> {

    pub(crate) const LAYER_NAME: &str = "tiles";

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbPolygon, Some(&TileFeature::FIELD_DEFS), overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_tile(&mut self, tile: NewTileEntity) -> Result<(),CommandError> {

        self.tiles.add(tile.geometry,&[
                TileFeature::FIELD_SITE_X,
                TileFeature::FIELD_SITE_Y,
            ],&[
                FieldValue::RealValue(tile.site_x),
                FieldValue::RealValue(tile.site_y),
            ])?;
        Ok(())

    }

    pub(crate) fn read_entities_to_vec<Progress: ProgressObserver, Data: TileEntity>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        progress.start_known_endpoint(|| ("Reading tiles.",self.tiles.layer.feature_count() as usize));
        let mut result = Vec::new();
        for (i,feature) in self.read_features().enumerate() {
            result.push(Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "Tiles read.");
        Ok(result)
    }

    pub(crate) fn read_entities_to_index<Progress: ProgressObserver, Data: TileEntity>(&mut self, progress: &mut Progress) -> Result<HashMap<u64,Data>,CommandError> {
        progress.start_known_endpoint(|| ("Indexing tiles.",self.tiles.layer.feature_count() as usize));
        let mut result = HashMap::new();
        for (i,feature) in self.read_features().enumerate() {
            result.insert(entity!(fieldassign@ feature fid u64),Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "Tiles indexed.");
        Ok(result)
    }

    pub(crate) fn feature_by_id(&self, fid: &u64) -> Option<TileFeature> {
        self.tiles.layer.feature(*fid).map(TileFeature::from)
    }

    #[allow(dead_code)] pub(crate) fn entity_by_id<Data: TileEntity>(&mut self, fid: &u64) -> Result<Option<Data>,CommandError> {
        self.feature_by_id(fid).map(Data::try_from_feature).transpose()
    }

    pub(crate) fn update_feature(&self, feature: TileFeature) -> Result<(),CommandError> {
        Ok(self.tiles.layer.set_feature(feature.feature)?)
    }

    // FUTURE: It would be nice if we could set the filter and retrieve the features all at once. But then I have to implement drop.
    pub(crate) fn set_spatial_filter_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.tiles.layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y)
    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<TileFeature> {
        self.tiles.layer.features().into()
    }

    pub(crate) fn read_entities<Data: TileEntity>(&mut self) -> TileEntityIterator<Data> {
        self.read_features().into()
    }

    pub(crate) fn clear_spatial_filter(&mut self) {
        self.tiles.layer.clear_spatial_filter()
    }

    pub(crate) fn feature_count(&self) -> usize {
        self.tiles.layer.feature_count() as usize
    }

    pub(crate) fn estimate_average_tile_area(&self) -> Result<f64,CommandError> {
        let extent = self.tiles.layer.get_extent()?;
        let width = extent.MaxX - extent.MinX;
        let height = extent.MaxY - extent.MinY;
        let tiles = self.feature_count();
        Ok((width*height)/tiles as f64)
    }

}


pub(crate) struct RiverFeature<'lifetime> {

    feature: Feature<'lifetime>
}

impl<'lifetime> From<Feature<'lifetime>> for RiverFeature<'lifetime> {

    fn from(feature: Feature<'lifetime>) -> Self {
        Self {
            feature
        }
    }
}

impl<'lifetime> RiverFeature<'lifetime> {

    feature!(6; geometry: #[allow(dead_code)] {
        from_tile #[allow(dead_code)] set_from_tile i64 FIELD_FROM_TILE "from_tile" OGRFieldType::OFTInteger64;
        from_type #[allow(dead_code)] set_from_type river_segment_from FIELD_FROM_TYPE "from_type" OGRFieldType::OFTString;
        from_flow #[allow(dead_code)] set_from_flow f64 FIELD_FROM_FLOW "from_flow" OGRFieldType::OFTReal;
        to_tile #[allow(dead_code)] set_to_tile i64 FIELD_TO_TILE "to_tile" OGRFieldType::OFTInteger64;
        to_type #[allow(dead_code)] set_to_type river_segment_to FIELD_TO_TYPE "to_type" OGRFieldType::OFTString;
        to_flow #[allow(dead_code)] set_to_flow f64 FIELD_TO_FLOW "to_flow" OGRFieldType::OFTReal;
    });

}

// NOTE: I tried using a TryFrom, but because TileFeature requires a lifetime, I had to add that in as well, and it started to propagate. 
// This is a much easier version of the same thing.
pub(crate) trait RiverEntity: Sized {

    fn try_from_feature(feature: RiverFeature) -> Result<Self,CommandError>;

}

pub(crate) struct RiverEntityIterator<'lifetime, Data: RiverEntity> {
    features: TypedFeatureIterator<'lifetime,RiverFeature<'lifetime>>,
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'lifetime,Data: RiverEntity> Iterator for RiverEntityIterator<'lifetime,Data> {
    type Item = Result<(u64,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from_feature(feature)) {
                (Some(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (None, Ok(_)) => Some(Err(CommandError::MissingField("fid"))),
                (_, Err(e)) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

impl<'lifetime,Data: RiverEntity> From<TypedFeatureIterator<'lifetime,RiverFeature<'lifetime>>> for RiverEntityIterator<'lifetime,Data> {
    fn from(features: TypedFeatureIterator<'lifetime,RiverFeature<'lifetime>>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}

entity!(NewRiver RiverEntity {
    from_tile: i64,
    from_type: RiverSegmentFrom,
    from_flow: f64,
    to_tile: i64,
    to_type: RiverSegmentTo,
    to_flow: f64,
    line: Vec<Point> = |_| Ok::<_,CommandError>(Vec::new())
});



pub(crate) struct RiversLayer<'lifetime> {
    segments: WorldLayer<'lifetime>
}

impl<'lifetime> RiversLayer<'lifetime> {

    pub(crate) const LAYER_NAME: &str = "rivers";

    #[allow(dead_code)] fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let segments = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            segments
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let segments = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbLineString, Some(&RiverFeature::FIELD_DEFS), overwrite)?;

        Ok(Self {
            segments
        })
    }

    pub(crate) fn add_segment(&mut self, segment: &NewRiver) -> Result<(),CommandError> {
        let geometry = create_line(&segment.line)?;
        let (field_names,field_values) = RiverFeature::to_field_names_values(
            segment.from_tile, 
            &segment.from_type, 
            segment.from_flow, 
            segment.to_tile, 
            &segment.to_type,
            segment.to_flow);
        self.segments.add(geometry, &field_names, &field_values)
    }


    #[allow(dead_code)] pub(crate) fn read_entities_to_vec<Progress: ProgressObserver, Data: RiverEntity>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        progress.start_known_endpoint(|| ("Reading river segments.",self.segments.layer.feature_count() as usize));
        let mut result = Vec::new();
        for (i,feature) in self.read_features().enumerate() {
            result.push(Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "River segments read.");
        Ok(result)
    }

    #[allow(dead_code)] pub(crate) fn read_entities_to_index<Progress: ProgressObserver, Data: RiverEntity>(&mut self, progress: &mut Progress) -> Result<HashMap<u64,Data>,CommandError> {
        progress.start_known_endpoint(|| ("Indexing river segments.",self.segments.layer.feature_count() as usize));
        let mut result = HashMap::new();
        for (i,feature) in self.read_features().enumerate() {
            result.insert(entity!(fieldassign@ feature fid u64),Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "River segments indexed.");
        Ok(result)
    }

    pub(crate) fn feature_by_id(&self, fid: &u64) -> Option<RiverFeature> {
        self.segments.layer.feature(*fid).map(RiverFeature::from)
    }

    #[allow(dead_code)] pub(crate) fn entity_by_id<Data: RiverEntity>(&mut self, fid: &u64) -> Result<Option<Data>,CommandError> {
        self.feature_by_id(fid).map(Data::try_from_feature).transpose()
    }

    #[allow(dead_code)] pub(crate) fn update_feature(&self, feature: RiverFeature) -> Result<(),CommandError> {
        Ok(self.segments.layer.set_feature(feature.feature)?)
    }

    // FUTURE: It would be nice if we could set the filter and retrieve the features all at once. But then I have to implement drop.
    #[allow(dead_code)] pub(crate) fn set_spatial_filter_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.segments.layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y)
    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<RiverFeature> {
        self.segments.layer.features().into()
    }

    #[allow(dead_code)] pub(crate) fn read_entities<Data: RiverEntity>(&mut self) -> RiverEntityIterator<Data> {
        self.read_features().into()
    }

    #[allow(dead_code)] pub(crate) fn clear_spatial_filter(&mut self) {
        self.segments.layer.clear_spatial_filter()
    }

    #[allow(dead_code)] pub(crate) fn feature_count(&self) -> usize {
        self.segments.layer.feature_count() as usize
    }


}


pub(crate) struct LakeFeature<'lifetime> {

    feature: Feature<'lifetime>
}

impl<'lifetime> From<Feature<'lifetime>> for LakeFeature<'lifetime> {

    fn from(feature: Feature<'lifetime>) -> Self {
        Self {
            feature
        }
    }
}

impl<'lifetime> LakeFeature<'lifetime> {

    feature!(1; geometry: #[allow(dead_code)] {
        #[allow(dead_code)] elevation #[allow(dead_code)] set_elevation f64 FIELD_ELEVATION "elevation" OGRFieldType::OFTReal;
    });

}

pub(crate) trait LakeEntity: Sized {

    fn try_from_feature(feature: LakeFeature) -> Result<Self,CommandError>;

}

pub(crate) struct LakeEntityIterator<'lifetime, Data: LakeEntity> {
    features: TypedFeatureIterator<'lifetime,LakeFeature<'lifetime>>,
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'lifetime,Data: LakeEntity> Iterator for LakeEntityIterator<'lifetime,Data> {
    type Item = Result<(u64,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from_feature(feature)) {
                (Some(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (None, Ok(_)) => Some(Err(CommandError::MissingField("fid"))),
                (_, Err(e)) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

impl<'lifetime,Data: LakeEntity> From<TypedFeatureIterator<'lifetime,LakeFeature<'lifetime>>> for LakeEntityIterator<'lifetime,Data> {
    fn from(features: TypedFeatureIterator<'lifetime,LakeFeature<'lifetime>>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}

entity!(NewLake LakeEntity {
    elevation: f64,
    geometry: Geometry
});


pub(crate) struct LakesLayer<'lifetime> {
    lakes: WorldLayer<'lifetime>
}

impl<'lifetime> LakesLayer<'lifetime> {

    pub(crate) const LAYER_NAME: &str = "lakes";

    #[allow(dead_code)] fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let lakes = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            lakes
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let lakes = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbPolygon, Some(&LakeFeature::FIELD_DEFS), overwrite)?;

        Ok(Self {
            lakes
        })
    }

    pub(crate) fn add_lake(&mut self, lake: NewLake) -> Result<(),CommandError> {
        let (field_names,field_values) = LakeFeature::to_field_names_values(
            lake.elevation);
        self.lakes.add(lake.geometry, &field_names, &field_values)
    }

    #[allow(dead_code)] pub(crate) fn read_entities_to_vec<Progress: ProgressObserver, Data: LakeEntity>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        progress.start_known_endpoint(|| ("Reading lakes.",self.lakes.layer.feature_count() as usize));
        let mut result = Vec::new();
        for (i,feature) in self.read_features().enumerate() {
            result.push(Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "Lakes read.");
        Ok(result)
    }

    #[allow(dead_code)] pub(crate) fn read_entities_to_index<Progress: ProgressObserver, Data: LakeEntity>(&mut self, progress: &mut Progress) -> Result<HashMap<u64,Data>,CommandError> {
        progress.start_known_endpoint(|| ("Indexing lakes.",self.lakes.layer.feature_count() as usize));
        let mut result = HashMap::new();
        for (i,feature) in self.read_features().enumerate() {
            result.insert(entity!(fieldassign@ feature fid u64),Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "Lakes indexed.");
        Ok(result)
    }

    pub(crate) fn feature_by_id(&self, fid: &u64) -> Option<LakeFeature> {
        self.lakes.layer.feature(*fid).map(LakeFeature::from)
    }

    #[allow(dead_code)] pub(crate) fn entity_by_id<Data: LakeEntity>(&mut self, fid: &u64) -> Result<Option<Data>,CommandError> {
        self.feature_by_id(fid).map(Data::try_from_feature).transpose()
    }

    #[allow(dead_code)] pub(crate) fn update_feature(&self, feature: LakeFeature) -> Result<(),CommandError> {
        Ok(self.lakes.layer.set_feature(feature.feature)?)
    }

    // FUTURE: It would be nice if we could set the filter and retrieve the features all at once. But then I have to implement drop.
    #[allow(dead_code)] pub(crate) fn set_spatial_filter_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.lakes.layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y)
    }

    pub(crate) fn read_features(&mut self) -> TypedFeatureIterator<LakeFeature> {
        self.lakes.layer.features().into()
    }

    #[allow(dead_code)] pub(crate) fn read_entities<Data: LakeEntity>(&mut self) -> LakeEntityIterator<Data> {
        self.read_features().into()
    }

    #[allow(dead_code)] pub(crate) fn clear_spatial_filter(&mut self) {
        self.lakes.layer.clear_spatial_filter()
    }

    #[allow(dead_code)] pub(crate) fn feature_count(&self) -> usize {
        self.lakes.layer.feature_count() as usize
    }


}

#[derive(Clone)]
pub(crate) enum BiomeCriteria {
    Matrix(Vec<(usize,usize)>), // temperature band, moisture band
    Wetland,
    Glacier
}

impl TryFrom<String> for BiomeCriteria {
    type Error = CommandError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "wetland" => Ok(Self::Wetland),
            "glacier" => Ok(Self::Glacier),
            list => {
                let mut result = Vec::new();
                for value in list.split(',') {
                    let value = value.splitn(2,':');
                    let mut value = value.map(str::parse).map(|a| a.map_err(|_| CommandError::InvalidBiomeMatrixValue(list.to_owned())));
                    let temperature = value.next().ok_or_else(|| CommandError::InvalidBiomeMatrixValue(list.to_owned()))??;
                    let moisture = value.next().ok_or_else(|| CommandError::InvalidBiomeMatrixValue(list.to_owned()))??;
                    result.push((temperature,moisture));
                }
                Ok(Self::Matrix(result))
            }
        }
    }
}

impl Into<String> for &BiomeCriteria {

    fn into(self) -> String {
        match self {
            BiomeCriteria::Wetland => "wetland".to_owned(),
            BiomeCriteria::Glacier => "glacier".to_owned(),
            BiomeCriteria::Matrix(list) => {
                list.iter().map(|(temperature,moisture)| format!("{}:{}",temperature,moisture)).collect::<Vec<String>>().join(",")

            }
        }
    }
}

struct BiomeMatrix<'lifetime> {
    matrix: [[&'lifetime str; 26]; 5],
    glacier: &'lifetime str,
    wetland: &'lifetime str
}

pub(crate) struct BiomeFeature<'lifetime> {

    feature: Feature<'lifetime>
}

impl<'lifetime> From<Feature<'lifetime>> for BiomeFeature<'lifetime> {

    fn from(feature: Feature<'lifetime>) -> Self {
        Self {
            feature
        }
    }
}

// TODO: You know, that feature! and entity! macro could be joined together into one big layer! macro.
impl<'lifetime> BiomeFeature<'lifetime> {

    feature!(3; geometry: #[allow(dead_code)] {
        #[allow(dead_code)] name #[allow(dead_code)] set_name string FIELD_NAME "name" OGRFieldType::OFTString;
        #[allow(dead_code)] habitability #[allow(dead_code)] set_habitability i32 FIELD_HABITABILITY "habitability" OGRFieldType::OFTInteger;
        #[allow(dead_code)] criteria #[allow(dead_code)] set_criteria biome_criteria FIELD_CRITERIA "criteria" OGRFieldType::OFTString;
    });

    const HOT_DESERT: &str = "Hot desert";
    const COLD_DESERT: &str = "Cold desert";
    const SAVANNA: &str = "Savanna";
    const GRASSLAND: &str = "Grassland";
    const TROPICAL_SEASONAL_FOREST: &str = "Tropical seasonal forest";
    const TEMPERATE_DECIDUOUS_FOREST: &str = "Temperate deciduous forest";
    const TROPICAL_RAINFOREST: &str = "Tropical rainforest";
    const TEMPERATE_RAINFOREST: &str = "Temperate rainforest";
    const TAIGA: &str = "Taiga";
    const TUNDRA: &str = "Tundra";
    const GLACIER: &str = "Glacier";
    const WETLAND: &str = "Wetland";
    

    const DEFAULT_BIOMES: [(&str, i32, BiomeCriteria); 12] = [
        (Self::HOT_DESERT,4,BiomeCriteria::Matrix(vec![])),
        (Self::COLD_DESERT,10,BiomeCriteria::Matrix(vec![])),
        (Self::SAVANNA,22,BiomeCriteria::Matrix(vec![])),
        (Self::GRASSLAND,30,BiomeCriteria::Matrix(vec![])),
        (Self::TROPICAL_SEASONAL_FOREST,50,BiomeCriteria::Matrix(vec![])),
        (Self::TEMPERATE_DECIDUOUS_FOREST,100,BiomeCriteria::Matrix(vec![])),
        (Self::TROPICAL_RAINFOREST,80,BiomeCriteria::Matrix(vec![])),
        (Self::TEMPERATE_RAINFOREST,90,BiomeCriteria::Matrix(vec![])),
        (Self::TAIGA,12,BiomeCriteria::Matrix(vec![])),
        (Self::TUNDRA,4,BiomeCriteria::Matrix(vec![])),
        (Self::GLACIER,0,BiomeCriteria::Glacier),
        (Self::WETLAND,12,BiomeCriteria::Wetland),
    ];

    //these constants make the default matrix easier to read.
    const HDT: &str = Self::HOT_DESERT;
    const CDT: &str = Self::COLD_DESERT;
    const SAV: &str = Self::SAVANNA;
    const GRA: &str = Self::GRASSLAND;
    const TRF: &str = Self::TROPICAL_SEASONAL_FOREST;
    const TEF: &str = Self::TEMPERATE_DECIDUOUS_FOREST;
    const TRR: &str = Self::TROPICAL_RAINFOREST;
    const TER: &str = Self::TEMPERATE_RAINFOREST;
    const TAI: &str = Self::TAIGA;
    const TUN: &str = Self::TUNDRA;

    const DEFAULT_MATRIX: [[&str; 26]; 5] = [
        // hot ↔ cold [>19°C; <-4°C]; dry ↕ wet
        [Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::TUN],
        [Self::SAV, Self::SAV, Self::SAV, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRR, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN]
    ];

    fn get_default_biomes() -> Vec<(&'static str, i32, BiomeCriteria)> {
        let mut matrix_criteria = HashMap::new();
        for (moisture,row) in Self::DEFAULT_MATRIX.iter().enumerate() {
            for (temperature,id) in row.iter().enumerate() {
                match matrix_criteria.entry(id) {
                    Vacant(entry) => {
                        entry.insert(vec![(temperature,moisture)]);
                    },
                    Occupied(mut entry) => entry.get_mut().push((temperature,moisture)),
                }
            }

        }

        let mut result = Vec::new();
        for (name,habitability,criteria) in Self::DEFAULT_BIOMES.iter() {
            let criteria = if let BiomeCriteria::Matrix(_) = criteria {
                BiomeCriteria::Matrix(matrix_criteria.get(name).unwrap().clone())
            } else {
                criteria.clone()
            };
            result.push((*name,*habitability,criteria))


        }

        result

    }

    fn build_matrix_from_biomes(biomes: Vec<(&str, i32, BiomeCriteria)>) -> Result<BiomeMatrix,CommandError> {
        let mut matrix: [[&str; 26]; 5] = Default::default();
        let mut wetland = None;
        let mut glacier = None;
        for biome in biomes {
            match biome.2 {
                BiomeCriteria::Matrix(list) => {
                    for (temp,moist) in list {
                        matrix[moist][temp] = biome.0
                    }
                },
                BiomeCriteria::Wetland => wetland = Some(biome.0),
                BiomeCriteria::Glacier => glacier = Some(biome.0),
            }

        }
        let wetland = wetland.ok_or_else(|| CommandError::MissingWetlandBiome)?;
        let glacier = glacier.ok_or_else(|| CommandError::MissingGlacierBiome)?;
        for moisture in 0..matrix.len() {
            for temperature in 0..matrix[moisture].len() {
                if matrix[moisture][temperature] == "" {
                    return Err(CommandError::MissingBiomeMatrixSlot(moisture,temperature))
                }
            }
        }
        Ok(BiomeMatrix {
            matrix,
            glacier,
            wetland,
        })
    }

}

pub(crate) trait BiomeEntity: Sized {

    fn try_from_feature(feature: BiomeFeature) -> Result<Self,CommandError>;

}

pub(crate) struct BiomeEntityIterator<'lifetime, Data: BiomeEntity> {
    features: TypedFeatureIterator<'lifetime,BiomeFeature<'lifetime>>,
    data: std::marker::PhantomData<Data>
}

// This actually returns a pair with the id and the data, in case the entity doesn't store the data itself.
impl<'lifetime,Data: BiomeEntity> Iterator for BiomeEntityIterator<'lifetime,Data> {
    type Item = Result<(u64,Data),CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(feature) = self.features.next() {
            match (feature.fid(),Data::try_from_feature(feature)) {
                (Some(fid), Ok(entity)) => Some(Ok((fid,entity))),
                (None, Ok(_)) => Some(Err(CommandError::MissingField("fid"))),
                (_, Err(e)) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

impl<'lifetime,Data: BiomeEntity> From<TypedFeatureIterator<'lifetime,BiomeFeature<'lifetime>>> for BiomeEntityIterator<'lifetime,Data> {
    fn from(features: TypedFeatureIterator<'lifetime,BiomeFeature<'lifetime>>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}

entity!(BiomeData BiomeEntity {
    name: String,
    habitability: i32,
    criteria: BiomeCriteria
});

pub(crate) struct BiomeLayer<'lifetime> {
    biomes: WorldLayer<'lifetime>
}

impl<'lifetime> BiomeLayer<'lifetime> {

    pub(crate) const LAYER_NAME: &str = "biomes";

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let biomes = WorldLayer::open_from_dataset(dataset, Self::LAYER_NAME)?;
        Ok(Self {
            biomes
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let biomes = WorldLayer::create_from_dataset(dataset, Self::LAYER_NAME, OGRwkbGeometryType::wkbNone, Some(&BiomeFeature::FIELD_DEFS), overwrite)?;

        Ok(Self {
            biomes
        })
    }


    pub(crate) fn add_biome(&mut self, biome: BiomeData) -> Result<(),CommandError> {

        let (field_names,field_values) = BiomeFeature::to_field_names_values(
            &biome.name,biome.habitability,&biome.criteria);
        self.biomes.add_without_geometry(&field_names, &field_values)

    }


}

pub(crate) struct WorldMap {
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &str = "GPKG";

    fn new(dataset: Dataset) -> Self {
        Self {
            dataset
        }
    }

    #[allow(dead_code)] pub(crate) fn open<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        let dataset = Dataset::open(path)?;
        Ok(Self::new(dataset))
    }


    pub(crate) fn edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Dataset::open_ex(path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?))
    }

    pub(crate) fn create_or_edit<FilePath: AsRef<Path>>(path: FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(path)?;
            Ok(Self::new(dataset))
        }

    }

    pub(crate) fn with_transaction<Callback: FnOnce(&mut WorldMapTransaction) -> Result<(),CommandError>>(&mut self, callback: Callback) -> Result<(),CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        callback(&mut transaction)?;
        transaction.commit()?;
        Ok(())

    }

    pub(crate) fn save(&mut self) -> Result<(),CommandError> {
        self.dataset.flush_cache()?;
        Ok(())
    }

    pub(crate) fn points_layer(&self) -> Result<PointsLayer,CommandError> {
        PointsLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn triangles_layer(&self) -> Result<TrianglesLayer,CommandError> {
        TrianglesLayer::open_from_dataset(&self.dataset)
    }

    #[allow(dead_code)] pub(crate) fn tiles_layer(&self) -> Result<TilesLayer,CommandError> {
        TilesLayer::open_from_dataset(&self.dataset)
    }

    #[allow(dead_code)] pub(crate) fn biomes_layer(&self) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn load_points_layer<Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Option<&mut Progress>) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target_points = target.create_points_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing points.",generator.size_hint().1));
    
            for (i,point) in generator.enumerate() {
                target_points.add_point(point?)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Points written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving layer."); 
        
        self.save()?;
    
        progress.finish(|| "Layer saved.");
    
        Ok(())
    
    }

    pub(crate) fn load_triangles_layer<'lifetime, Generator: Iterator<Item=Result<Geometry,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target = target.create_triangles_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing triangles.",generator.size_hint().1));
    
            for (i,triangle) in generator.enumerate() {
                target.add_triangle(triangle?.to_owned())?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Triangles written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())

    }

    pub(crate) fn load_tile_layer<'lifetime, Generator: Iterator<Item=Result<NewTileEntity,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut target = target.create_tile_layer(overwrite_layer)?;
        
            // boundary points    
    
            progress.start(|| ("Writing tiles.",generator.size_hint().1));
    
            for (i,tile) in generator.enumerate() {
                target.add_tile(tile?)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Tiles written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())

    }

    pub(crate) fn calculate_tile_neighbors<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            calculate_neighbors(&mut tiles,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn sample_elevations_on_tiles<Progress: ProgressObserver>(&mut self, raster: &RasterMap, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            sample_elevations(&mut tiles,raster,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn sample_ocean_on_tiles<Progress: ProgressObserver>(&mut self, raster: &RasterMap, method: OceanSamplingMethod, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            sample_ocean(&mut tiles,raster,method,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn generate_temperatures<Progress: ProgressObserver>(&mut self, equator_temp: i8, polar_temp: i8, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            generate_temperatures(&mut tiles, equator_temp,polar_temp,progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }


    pub(crate) fn generate_winds<Progress: ProgressObserver>(&mut self, winds: [i32; 6], progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            generate_winds(&mut tiles, winds, progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }


    pub(crate) fn generate_precipitation<Progress: ProgressObserver>(&mut self, moisture: u16, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            generate_precipitation(&mut tiles, moisture, progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


    }

    pub(crate) fn generate_water_flow<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(HashMap<u64,TileEntityForWaterFill>,Vec<(u64,f64)>),CommandError> {

        let mut result = None;
        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            result = Some(generate_water_flow(&mut tiles, progress)?);

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(result.unwrap()) // the only way it wouldn't be Some is if there was an error.


    }

    pub(crate) fn get_tile_map_and_queue_for_water_fill<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(HashMap<u64,TileEntityForWaterFill>,Vec<(u64,f64)>),CommandError> {

        let mut tile_map = HashMap::new();
        let mut tile_queue = Vec::new();

        let mut tiles = self.tiles_layer()?;

        progress.start_known_endpoint(|| ("Indexing data.",tiles.feature_count() as usize));

        for (i,data) in tiles.read_entities::<TileEntityForWaterFill>().enumerate() {
            let (fid,entity) = data?;
            if entity.water_accumulation > 0.0 {
                tile_queue.push((fid,entity.water_accumulation));
            }
            tile_map.insert(fid, entity);
            progress.update(|| i);
    
        }
        progress.finish(|| "Data indexed.");

        Ok((tile_map,tile_queue))
        
    
    }


    pub(crate) fn generate_water_fill<Progress: ProgressObserver>(&mut self, tile_map: HashMap<u64,TileEntityForWaterFill>, tile_queue: Vec<(u64,f64)>, lake_bezier_scale: f64, lake_buffer_scale: f64, progress: &mut Progress) -> Result<Vec<NewLake>,CommandError> {

        let mut result = None;

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            result = Some(generate_water_fill(&mut tiles, tile_map, tile_queue, lake_bezier_scale, lake_buffer_scale, progress)?);

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(result.unwrap())

    }

    pub(crate) fn generate_water_rivers<Progress: ProgressObserver>(&mut self, bezier_scale: f64, progress: &mut Progress) -> Result<Vec<NewRiver>,CommandError> {

        let mut result = None;

        self.with_transaction(|target| {
            // FUTURE: I don't really need this to be in a transaction. The layer shouldn't need to be edited. However,
            // a feature iterator function requires the layer to be mutable. So, to avoid confusion, I'm marking this as
            // in a transaction as well.

            let mut tiles = target.edit_tile_layer()?;

            result = Some(generate_water_rivers(&mut tiles, bezier_scale, progress)?);

            Ok(())
    
        })?;
    
        Ok(result.unwrap())

    }

    pub(crate) fn load_rivers<Progress: ProgressObserver>(&mut self, segments: Vec<NewRiver>, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut segments_layer = target.create_rivers_layer(overwrite_layer)?;

        
            // boundary points    
    
            progress.start_known_endpoint(|| ("Writing rivers.",segments.len()));
    
            for (i,segment) in segments.iter().enumerate() {
                segments_layer.add_segment(segment)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Rivers written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving layer."); 
        
        self.save()?;
    
        progress.finish(|| "Layer saved.");
    
        Ok(())

    }

    pub(crate) fn load_lakes<Progress: ProgressObserver>(&mut self, lakes: Vec<NewLake>, overwrite_layer: bool, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut lakes_layer = target.create_lakes_layer(overwrite_layer)?;

        
            // boundary points    
    
            progress.start_known_endpoint(|| ("Writing lakes.",lakes.len()));
    
            for (i,lake) in lakes.into_iter().enumerate() {
                lakes_layer.add_lake(lake)?;
                progress.update(|| i);
            }
    
            progress.finish(|| "Lakes written.");
    
            Ok(())
        })?;
    
        progress.start_unknown_endpoint(|| "Saving layer."); 
        
        self.save()?;
    
        progress.finish(|| "Layer saved.");
    
        Ok(())

    }

    pub(crate) fn fill_biome_defaults(&mut self, overwrite_layer: bool, progress: &mut crate::progress::ConsoleProgressBar) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut biomes = target.create_biomes_layer(overwrite_layer)?;

            let default_biomes = BiomeFeature::get_default_biomes();
    
            progress.start_known_endpoint(|| ("Writing biomes.",default_biomes.len()));

            for (name,habitability,criteria) in &default_biomes {
                biomes.add_biome(BiomeData {
                    name: name.to_owned().to_owned(),
                    habitability: *habitability,
                    criteria: criteria.clone()
                })?
            }

            let matrix = BiomeFeature::build_matrix_from_biomes(default_biomes)?;

            println!("matrix {:?}",matrix.matrix);
            println!("glacier {}",matrix.glacier);
            println!("wetland {}",matrix.wetland);

    
            progress.finish(|| "Biomes written.");
    
            Ok(())
        })?;
    
        Ok(())
    }



}

pub(crate) struct WorldMapTransaction<'lifetime> {
    dataset: Transaction<'lifetime>
}

impl<'lifetime> WorldMapTransaction<'lifetime> {

    fn new(dataset: Transaction<'lifetime>) -> Self {
        Self {
            dataset
        }
    }

    pub(crate) fn create_points_layer(&mut self, overwrite: bool) -> Result<PointsLayer,CommandError> {
        Ok(PointsLayer::create_from_dataset(&mut self.dataset, overwrite)?)       

    }

    pub(crate) fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TrianglesLayer,CommandError> {
        Ok(TrianglesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn create_tile_layer(&mut self, overwrite: bool) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    pub(crate) fn create_rivers_layer(&mut self, overwrite: bool) -> Result<RiversLayer,CommandError> {
        Ok(RiversLayer::create_from_dataset(&mut self.dataset, overwrite)?)

    }

    fn create_lakes_layer(&mut self, overwrite_layer: bool) -> Result<LakesLayer,CommandError> {
        Ok(LakesLayer::create_from_dataset(&mut self.dataset, overwrite_layer)?)
    }



    pub(crate) fn edit_tile_layer(&mut self) -> Result<TilesLayer,CommandError> {
        Ok(TilesLayer::open_from_dataset(&mut self.dataset)?)

    }

    pub(crate) fn create_biomes_layer(&mut self, overwrite: bool) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }

    #[allow(dead_code)] pub(crate) fn edit_biomes_layer(&mut self) -> Result<BiomeLayer,CommandError> {
        Ok(BiomeLayer::open_from_dataset(&mut self.dataset)?)

    }

    fn commit(self) -> Result<(),CommandError> {
        // TODO: Am I doing this anywhere?
        self.dataset.commit()?;
        Ok(())
    }

}

