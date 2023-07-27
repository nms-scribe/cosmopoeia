use std::path::Path;
use std::collections::HashMap;

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

pub(crate) const POINTS_LAYER_NAME: &str = "points";
pub(crate) const TRIANGLES_LAYER_NAME: &str = "triangles";
pub(crate) const TILES_LAYER_NAME: &str = "tiles";
pub(crate) const BIOME_LAYER_NAME: &str = "biomes";

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

        self.layer.create_feature_fields(geometry,field_names,field_values)?;
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

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let points = WorldLayer::open_from_dataset(dataset, POINTS_LAYER_NAME)?;
        Ok(Self {
            points
        })
    }

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let points = WorldLayer::create_from_dataset(dataset, POINTS_LAYER_NAME, OGRwkbGeometryType::wkbPoint, None, overwrite)?;

        Ok(Self {
            points
        })
    }

    pub(crate) fn add_point(&mut self, point: Geometry) -> Result<(),CommandError> {

        self.points.add(point,&[],&[])

    }

    pub(crate) fn read_points(&mut self) -> LayerGeometryIterator {
        self.points.read_geometries()

    }


}



pub(crate) struct TrianglesLayer<'lifetime> {
    tiles: WorldLayer<'lifetime>
}

impl<'lifetime> TrianglesLayer<'lifetime> {

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, TRIANGLES_LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, TRIANGLES_LAYER_NAME, OGRwkbGeometryType::wkbPolygon, None, overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_triangle(&mut self, geo: Geometry) -> Result<(),CommandError> {

        self.tiles.add(geo,&[],&[])

    }


    pub(crate) fn read_triangles(&mut self) -> LayerGeometryIterator {
        self.tiles.read_geometries()

    }



}

pub(crate) struct VoronoiSite {
    geometry: Geometry,
    site: Point
}

impl VoronoiSite {

    pub(crate) fn new(geometry: Geometry, site: Point) -> Self {
        Self {
            geometry,
            site
        }
    }
}


pub(crate) struct TileFeature<'lifetime> {

    feature: Feature<'lifetime>
}


macro_rules! tile_field {
    (fieldtype@ f64) => {
        f64
    };
    (fieldtype@ i32) => {
        i32
    };
    (fieldtype@ bool) => {
        bool
    };
    (fieldtype@ vec_u64_i32) => {
        Vec<(u64,i32)>
    };
    (fieldtype@ vec_u64) => {
        Vec<u64>
    };
    (setfieldtype@ f64) => {
        f64
    };
    (setfieldtype@ i32) => {
        i32
    };
    (setfieldtype@ bool) => {
        bool
    };
    (setfieldtype@ vec_u64_i32) => {
        &Vec<(u64,i32)>
    };
    (setfieldtype@ vec_u64) => {
        &Vec<u64>
    };
    (getfield@ $self: ident $field: ident f64) => {
        Ok($self.feature.field_as_double_by_name(TilesLayer::$field)?)
    };
    (getfield@ $self: ident $field: ident i32) => {
        Ok($self.feature.field_as_integer_by_name(TilesLayer::$field)?)
    };
    (getfield@ $self: ident $field: ident bool) => {
        Ok($self.feature.field_as_integer_by_name(TilesLayer::$field)?.map(|n| n != 0))
    };
    (getfield@ $self: ident $field: ident vec_u64_i32) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name(TilesLayer::$field)? {
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
    (getfield@ $self: ident $field: ident vec_u64) => {
        if let Some(neighbors) = $self.feature.field_as_string_by_name(TilesLayer::$field)? {
            Ok(Some(neighbors.split(',').filter_map(|a| {
                a.parse().ok()
            }).collect()))
        } else {
            Ok(Some(Vec::new()))
        }

    };
    (setfield@ $self: ident $field: ident $value: ident f64) => {
        Ok($self.feature.set_field_double(TilesLayer::$field, $value)?)
    };
    (setfield@ $self: ident $field: ident $value: ident i32) => {
        Ok($self.feature.set_field_integer(TilesLayer::$field, $value)?)
    };
    (setfield@ $self: ident $field: ident $value: ident bool) => {
        Ok($self.feature.set_field_integer(TilesLayer::$field, if $value { 1 } else { 0 })?)
    };
    (setfield@ $self: ident $field: ident $value: ident vec_u64_i32) => {{
        let neighbors = $value.iter().map(|(fid,dir)| format!("{}:{}",fid,dir)).collect::<Vec<String>>().join(",");
        Ok($self.feature.set_field_string(TilesLayer::$field, &neighbors)?)
    }};
    (setfield@ $self: ident $field: ident $value: ident vec_u64) => {{
        let neighbors = $value.iter().map(|fid| format!("{}",fid)).collect::<Vec<String>>().join(",");
        Ok($self.feature.set_field_string(TilesLayer::$field, &neighbors)?)
    }};
    (get@ $(#[$attr: meta])* $prop: ident $field: ident $type: ident) => {
        $(#[$attr])* pub(crate) fn $prop(&self) -> Result<Option<tile_field!(fieldtype@ $type)>,CommandError> {
            tile_field!(getfield@ self $field $type)
        }
    };
    (set@ $(#[$attr: meta])* $set_prop: ident $field: ident $type: ident) => {
        $(#[$attr])* pub(crate) fn $set_prop(&mut self, value: tile_field!(setfieldtype@ $type)) -> Result<(),CommandError> {
            tile_field!(setfield@ self $field value $type)
        }            
    };
    ($(#[$get_attr: meta])* $prop: ident $(#[$set_attr: meta])* $set_prop: ident $field: ident $type: ident) => {
        tile_field!(get@ $(#[$get_attr])* $prop $field $type);

        tile_field!(set@ $(#[$set_attr])* $set_prop $field $type);
        
    };
}

impl<'lifetime> TileFeature<'lifetime> {

    fn from_data(feature: Feature<'lifetime>) -> Self {
        Self {
            feature
        }
    }

    pub(crate) fn fid(&self) -> Option<u64> {
        self.feature.fid()
    }

    pub(crate) fn geometry(&self) -> Option<&Geometry> {
        self.feature.geometry()
    }

    tile_field!(site_x #[allow(dead_code)] set_site_x FIELD_SITE_X f64);

    tile_field!(site_y #[allow(dead_code)] set_site_y FIELD_SITE_Y f64);

    tile_field!(elevation set_elevation FIELD_ELEVATION f64);

    tile_field!(elevation_scaled set_elevation_scaled FIELD_ELEVATION_SCALED i32);

    tile_field!(is_ocean set_is_ocean FIELD_IS_OCEAN bool);

    tile_field!(temperature set_temperature FIELD_TEMPERATURE f64);

    tile_field!(wind set_wind FIELD_WIND i32);

    tile_field!(precipitation set_precipitation FIELD_PRECIPITATION f64);

    tile_field!(#[allow(dead_code)] water_flow set_water_flow FIELD_WATER_FLOW f64);

    tile_field!(#[allow(dead_code)] water_accumulation set_water_accumulation FIELD_WATER_ACCUMULATION f64);

    tile_field!(neighbors set_neighbors FIELD_NEIGHBOR_TILES vec_u64_i32);

    tile_field!(#[allow(dead_code)] flow_to set_flow_to FIELD_FLOW_TO vec_u64);


}

pub(crate) struct TileFeatureIterator<'lifetime> {
    features: FeatureIterator<'lifetime>
}

impl<'lifetime> Iterator for TileFeatureIterator<'lifetime> {
    type Item = TileFeature<'lifetime>;

    fn next(&mut self) -> Option<Self::Item> {
        self.features.next().map(TileFeature::from_data)
    }
}

impl<'lifetime> From<FeatureIterator<'lifetime>> for TileFeatureIterator<'lifetime> {
    fn from(features: FeatureIterator<'lifetime>) -> Self {
        Self {
            features
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

#[macro_export]
macro_rules! tile_entity {
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
                $field: tile_entity!(fieldassign@ $feature $field $type $(= $function)?)
            ),*
        })

    };
    (from_data@ $name: ident $feature: ident, $($field: ident: $type: ty $(= $function: expr)?),*) => {{
        tile_entity!(constructor@ $name $feature $($field: $type $(= $function)? ),*)
    }};
    (fielddef@ $type: ty [$function: expr]) => {
        $type
    };
    (fielddef@ $type: ty) => {
        $type
    };
    ($name: ident $($field: ident: $type: ty $(= $function: expr)?),*) => {
        #[derive(Clone)]
        pub(crate) struct $name {
            $(
                pub(crate) $field: tile_entity!(fielddef@ $type $([$function])?)
            ),*
        }

        impl TileEntity for $name {

            fn try_from_feature(feature: crate::world_map::TileFeature) -> Result<Self,CommandError> {
                tile_entity!(from_data@ $name feature, $($field: $type $(= $function)?),*)
            }
        }        

    };
}

tile_entity!(TileEntitySite fid: u64, site_x: f64, site_y: f64);
tile_entity!(TileEntitySiteGeo fid: u64, geometry: Geometry, site_x: f64, site_y: f64);
tile_entity!(TileEntityLatElevOcean fid: u64, site_y: f64, elevation: f64, is_ocean: bool);
tile_entity!(TileEntityLat fid: u64, site_y: f64);
tile_entity!(TileEntityForWaterFlow
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    precipitation: f64,
    water_flow: f64 = |_| Ok::<_,CommandError>(0.0),
    water_accumulation: f64 = |_| Ok::<_,CommandError>(0.0),
    flow_to: Vec<u64> = |_| Ok::<_,CommandError>(Vec::new())
);

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
tile_entity!(TileEntityForWaterFill
    elevation: f64, 
    is_ocean: bool, 
    neighbors: Vec<(u64,i32)>,
    precipitation: f64,
    water_flow: f64,
    water_accumulation: f64,
    flow_to: Vec<u64>,
    lake_id: Option<usize> = |_| Ok::<_,CommandError>(None)
);

impl From<TileEntityForWaterFlow> for TileEntityForWaterFill {

    fn from(value: TileEntityForWaterFlow) -> Self {
        Self {
            elevation: value.elevation,
            is_ocean: value.is_ocean,
            neighbors: value.neighbors,
            precipitation: value.precipitation,
            water_flow: value.water_flow,
            water_accumulation: value.water_accumulation,
            flow_to: value.flow_to,
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
    features: TileFeatureIterator<'lifetime>,
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

impl<'lifetime,Data: TileEntity> From<TileFeatureIterator<'lifetime>> for TileEntityIterator<'lifetime,Data> {
    fn from(features: TileFeatureIterator<'lifetime>) -> Self {
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

    pub(crate) const FIELD_SITE_X: &str = "site_x";
    pub(crate) const FIELD_SITE_Y: &str = "site_y";
    pub(crate) const FIELD_NEIGHBOR_TILES: &str = "neighbor_tiles";
    pub(crate) const FIELD_ELEVATION: &str = "elevation";
    // NOTE: This field is used in various places which use algorithms ported from AFMG, which depend on a height from 0-100. 
    // If I ever get rid of those algorithms, this field can go away.
    pub(crate) const FIELD_ELEVATION_SCALED: &str = "elevation_scaled";
    pub(crate) const FIELD_IS_OCEAN: &str = "is_ocean";
    pub(crate) const FIELD_TEMPERATURE: &str = "temperature";
    pub(crate) const FIELD_WIND: &str = "wind_dir";
    pub(crate) const FIELD_PRECIPITATION: &str = "precipitation";
    pub(crate) const FIELD_WATER_FLOW: &str = "water_flow";
    pub(crate) const FIELD_WATER_ACCUMULATION: &str = "water_accum";
    pub(crate) const FIELD_FLOW_TO: &str = "flow_to";

    const FIELD_DEFS: [(&str,OGRFieldType::Type); 12] = [
        (Self::FIELD_SITE_X,OGRFieldType::OFTReal),
        (Self::FIELD_SITE_Y,OGRFieldType::OFTReal),
        (Self::FIELD_ELEVATION,OGRFieldType::OFTReal),
        (Self::FIELD_ELEVATION_SCALED,OGRFieldType::OFTInteger),
        (Self::FIELD_IS_OCEAN,OGRFieldType::OFTInteger),
        (Self::FIELD_TEMPERATURE,OGRFieldType::OFTReal),
        (Self::FIELD_WIND,OGRFieldType::OFTInteger),
        (Self::FIELD_PRECIPITATION,OGRFieldType::OFTReal),
        (Self::FIELD_WATER_FLOW,OGRFieldType::OFTReal),
        (Self::FIELD_WATER_ACCUMULATION,OGRFieldType::OFTReal),
        (Self::FIELD_FLOW_TO,OGRFieldType::OFTString),
        (Self::FIELD_NEIGHBOR_TILES,OGRFieldType::OFTString) // put this one last to make the tables easier to read in QGIS.
    ];

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let tiles = WorldLayer::open_from_dataset(dataset, TILES_LAYER_NAME)?;
        Ok(Self {
            tiles
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let tiles = WorldLayer::create_from_dataset(dataset, TILES_LAYER_NAME, OGRwkbGeometryType::wkbPolygon, Some(&Self::FIELD_DEFS), overwrite)?;

        Ok(Self {
            tiles
        })
    }

    pub(crate) fn add_tile(&mut self, tile: VoronoiSite) -> Result<(),CommandError> {

        let (x,y) = tile.site.to_tuple();
        self.tiles.add(tile.geometry,&[
                Self::FIELD_SITE_X,
                Self::FIELD_SITE_Y,
            ],&[
                FieldValue::RealValue(x),
                FieldValue::RealValue(y),
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
            result.insert(tile_entity!(fieldassign@ feature fid u64),Data::try_from_feature(feature)?);
            progress.update(|| i);
        }
        progress.finish(|| "Tiles indexed.");
        Ok(result)
    }

    pub(crate) fn feature_by_id(&self, fid: &u64) -> Option<TileFeature> {
        self.tiles.layer.feature(*fid).map(TileFeature::from_data)
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

    pub(crate) fn read_features(&mut self) -> TileFeatureIterator {
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


}

pub(crate) struct BiomeLayer<'lifetime> {
    biomes: WorldLayer<'lifetime>
}

impl<'lifetime> BiomeLayer<'lifetime> {

    pub(crate) const FIELD_NAME: &str = "name";

    const FIELD_DEFS: [(&str,OGRFieldType::Type); 1] = [
        (Self::FIELD_NAME,OGRFieldType::OFTString),
    ];

    fn open_from_dataset(dataset: &'lifetime Dataset) -> Result<Self,CommandError> {
        let biomes = WorldLayer::open_from_dataset(dataset, BIOME_LAYER_NAME)?;
        Ok(Self {
            biomes
        })
    }
    

    fn create_from_dataset(dataset: &'lifetime mut Dataset, overwrite: bool) -> Result<Self,CommandError> {
        let biomes = WorldLayer::create_from_dataset(dataset, BIOME_LAYER_NAME, OGRwkbGeometryType::wkbNone, Some(&Self::FIELD_DEFS), overwrite)?;

        Ok(Self {
            biomes
        })
    }


    pub(crate) fn add_biome(&mut self, name: String) -> Result<(),CommandError> {

        self.biomes.add_without_geometry(&[
            Self::FIELD_NAME
        ], &[
            FieldValue::StringValue(name)
        ])

    }

    pub(crate) fn list_biomes(&mut self) -> Result<Vec<String>,CommandError> {
        Ok(self.biomes.layer.features().filter_map(|feature| {
            feature.field_as_string_by_name(Self::FIELD_NAME).transpose()
        }).collect::<Result<Vec<String>,gdal::errors::GdalError>>()?)
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

    pub(crate) fn biomes_layer(&self) -> Result<BiomeLayer,CommandError> {
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

    pub(crate) fn load_tile_layer<'lifetime, Generator: Iterator<Item=Result<VoronoiSite,CommandError>>, Progress: ProgressObserver>(&mut self, overwrite_layer: bool, generator: Generator, progress: &mut Progress) -> Result<(),CommandError> {

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

    pub(crate) fn generate_water_flow<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            let result = generate_water_flow(&mut tiles, progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
        Ok(())


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


    pub(crate) fn generate_water_fill<Progress: ProgressObserver>(&mut self, tile_map: HashMap<u64,TileEntityForWaterFill>, tile_queue: Vec<(u64,f64)>, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            generate_water_fill(&mut tiles, tile_map, tile_queue, progress)?;

            Ok(())
    
        })?;
    
        progress.start_unknown_endpoint(|| "Saving Layer..."); 
        
        self.save()?;
    
        progress.finish(|| "Layer Saved.");
    
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
        self.dataset.commit()?;
        Ok(())
    }

}

