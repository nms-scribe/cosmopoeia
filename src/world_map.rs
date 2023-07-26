use std::path::Path;

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
use crate::algorithms::generate_flowage;

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

    pub(crate) fn site_x(&self) -> Result<Option<f64>,CommandError> {
        Ok(self.feature.field_as_double_by_name(TilesLayer::FIELD_SITE_X)?)
    }

    pub(crate) fn site_y(&self) -> Result<Option<f64>,CommandError> {
        Ok(self.feature.field_as_double_by_name(TilesLayer::FIELD_SITE_Y)?)
    }

    pub(crate) fn neighbors(&self) -> Result<Option<Vec<(u64,i32)>>,CommandError> {
        if let Some(neighbors) = self.feature.field_as_string_by_name(TilesLayer::FIELD_NEIGHBOR_TILES)? {
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
    }

    pub(crate) fn elevation(&self) -> Result<Option<f64>,CommandError> {
        Ok(self.feature.field_as_double_by_name(TilesLayer::FIELD_ELEVATION)?)
    }

    pub(crate) fn elevation_scaled(&self) -> Result<Option<i32>,CommandError> {
        Ok(self.feature.field_as_integer_by_name(TilesLayer::FIELD_ELEVATION_SCALED)?)
    }

    pub(crate) fn ocean(&self) -> Result<Option<bool>,CommandError> {
        Ok(self.feature.field_as_integer_by_name(TilesLayer::FIELD_SITE_Y)?.map(|n| n != 0))
    }

    pub(crate) fn temperature(&self) -> Result<Option<f64>,CommandError> {
        Ok(self.feature.field_as_double_by_name(TilesLayer::FIELD_TEMPERATURE)?)
    }

    pub(crate) fn wind(&self) -> Result<Option<i32>,CommandError> {
        Ok(self.feature.field_as_integer_by_name(TilesLayer::FIELD_WIND)?)
    }

    pub(crate) fn precipitation(&self) -> Result<Option<f64>,CommandError> {
        Ok(self.feature.field_as_double_by_name(TilesLayer::FIELD_PRECIPITATION)?)
    }

    #[allow(dead_code)] pub(crate) fn set_site_x(&mut self, value: f64) -> Result<(),CommandError> {
        Ok(self.feature.set_field_double(TilesLayer::FIELD_SITE_X, value)?)
    }

    #[allow(dead_code)] pub(crate) fn set_site_y(&mut self, value: f64) -> Result<(),CommandError> {
        Ok(self.feature.set_field_double(TilesLayer::FIELD_SITE_Y, value)?)
    }

    pub(crate) fn set_neighbors(&mut self, value: &Vec<(u64,i32)>) -> Result<(),CommandError> {
        let neighbors = value.iter().map(|(fid,dir)| format!("{}:{}",fid,dir)).collect::<Vec<String>>().join(",");
        Ok(self.feature.set_field_string(TilesLayer::FIELD_ELEVATION, &neighbors)?)
    }

    pub(crate) fn set_elevation(&mut self, value: f64) -> Result<(),CommandError> {
        Ok(self.feature.set_field_double(TilesLayer::FIELD_ELEVATION, value)?)
    }

    pub(crate) fn set_elevation_scaled(&mut self, value: i32) -> Result<(),CommandError> {
        Ok(self.feature.set_field_integer(TilesLayer::FIELD_ELEVATION_SCALED, value)?)
    }

    pub(crate) fn set_ocean(&mut self, value: bool) -> Result<(),CommandError> {
        Ok(self.feature.set_field_integer(TilesLayer::FIELD_OCEAN, if value { 1 } else { 0 })?)
    }

    pub(crate) fn set_temperature(&mut self, value: f64) -> Result<(),CommandError> {
        Ok(self.feature.set_field_double(TilesLayer::FIELD_TEMPERATURE, value)?)
    }

    pub(crate) fn set_wind(&mut self, value: i32) -> Result<(),CommandError> {
        Ok(self.feature.set_field_integer(TilesLayer::FIELD_WIND, value)?)
    }

    pub(crate) fn set_precipitation(&mut self, value: f64) -> Result<(),CommandError> {
        Ok(self.feature.set_field_double(TilesLayer::FIELD_PRECIPITATION, value)?)
    }


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

pub(crate) struct TileDataIterator<'lifetime, Data: TileData> {
    features: TileFeatureIterator<'lifetime>,
    data: std::marker::PhantomData<Data>
}

impl<'lifetime,Data: TileData> Iterator for TileDataIterator<'lifetime,Data> {
    type Item = Result<Data,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(data) = self.features.next() {
            if let Some(feature) = Data::from_data(data).transpose() {
                return Some(feature)
            }
        }
        None
    }
}

impl<'lifetime,Data: TileData> From<TileFeatureIterator<'lifetime>> for TileDataIterator<'lifetime,Data> {
    fn from(features: TileFeatureIterator<'lifetime>) -> Self {
        Self {
            features,
            data: std::marker::PhantomData
        }
    }
}

pub(crate) trait TileData: Sized {

    fn from_data(feature: TileFeature) -> Result<Option<Self>,CommandError>;

}

macro_rules! tile_data {
    (variables@ $feature: ident, $($field: ident),*) => {
        $(
            let $field = $feature.$field()?;
        )*
    };
    (constructor@ $($field: ident),*) => {
        if let ($(Some($field)),*) = ($($field),*) {
            Ok(Some(Self {
                $(
                    $field
                ),*
            }))
        } else {
            Ok(None)
        }

    };
    (from_data@ $feature: ident, fid, geometry, $($field: ident),*) => {{
        let fid = $feature.fid();
        let geometry = $feature.geometry().cloned();
        tile_data!(variables@ $feature, $($field),*);
        tile_data!(constructor@ fid, geometry, $($field),*)
    }};
    (from_data@ $feature: ident, fid, $($field: ident),*) => {{
        let fid = $feature.fid();
        tile_data!(variables@ $feature, $($field),*);
        tile_data!(constructor@ fid, $($field),*)
    }};
    ($name: ident, fid, $($field: ident: $type: ty),*) => {
        pub(crate) struct $name {
            pub(crate) fid: u64,
            $(
                pub(crate) $field: $type
            ),*
        }

        impl TileData for $name {

            fn from_data(feature: TileFeature) -> Result<Option<Self>,CommandError> {
                tile_data!(from_data@ feature,fid,$($field),*)
            }
        }
        
    };
    ($name: ident, fid, geometry, $($field: ident: $type: ty),*) => {
        pub(crate) struct $name {
            pub(crate) fid: u64,
            pub(crate) geometry: Geometry,
            $(
                pub(crate) $field: $type
            ),*
        }

        impl TileData for $name {

            fn from_data(feature: TileFeature) -> Result<Option<Self>,CommandError> {
                tile_data!(from_data@ feature,fid,geometry,$($field),*)
            }
        }
        
    };
}

tile_data!(TileDataSite, fid, site_x: f64,site_y: f64);
tile_data!(TileDataSiteGeo, fid, geometry, site_x: f64,site_y: f64);
tile_data!(TileDataLatElevOcean, fid, site_y: f64, elevation: f64, ocean: bool);
tile_data!(TileDataLat, fid, site_y: f64);
tile_data!(TileDataForPrecipitation, fid, site_y: f64, elevation_scaled: i32, wind: i32, ocean: bool, neighbors: Vec<(u64,i32)>, temperature: f64);


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
    pub(crate) const FIELD_OCEAN: &str = "is_ocean";
    pub(crate) const FIELD_TEMPERATURE: &str = "temperature";
    pub(crate) const FIELD_WIND: &str = "wind_dir";
    pub(crate) const FIELD_PRECIPITATION: &str = "precipitation";

    const FIELD_DEFS: [(&str,OGRFieldType::Type); 9] = [
        (Self::FIELD_SITE_X,OGRFieldType::OFTReal),
        (Self::FIELD_SITE_Y,OGRFieldType::OFTReal),
        (Self::FIELD_NEIGHBOR_TILES,OGRFieldType::OFTString),
        (Self::FIELD_ELEVATION,OGRFieldType::OFTReal),
        (Self::FIELD_ELEVATION_SCALED,OGRFieldType::OFTInteger),
        (Self::FIELD_OCEAN,OGRFieldType::OFTInteger),
        (Self::FIELD_TEMPERATURE,OGRFieldType::OFTReal),
        (Self::FIELD_WIND,OGRFieldType::OFTInteger),
        (Self::FIELD_PRECIPITATION,OGRFieldType::OFTReal)
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

    pub(crate) fn read_features<Progress: ProgressObserver, Data: TileData>(&mut self, progress: &mut Progress) -> Result<Vec<Data>,CommandError> {
        progress.start_known_endpoint(|| ("Reading tiles.",self.tiles.layer.feature_count() as usize));
        let mut result = Vec::new();
        for (i,feature) in self.tiles.layer.features().enumerate() {
            if let Some(data) = Data::from_data(TileFeature::from_data(feature))? {
                result.push(data)
            }
            progress.update(|| i);
        }
        progress.finish(|| "Tiles read.");
        Ok(result)
    }

    pub(crate) fn feature_by_id(&self, fid: u64) -> Option<TileFeature> {
        self.tiles.layer.feature(fid).map(TileFeature::from_data)
    }

    pub(crate) fn update_feature(&self, feature: TileFeature) -> Result<(),CommandError> {
        Ok(self.tiles.layer.set_feature(feature.feature)?)
    }

    // FUTURE: It would be nice if we could set the filter and retrieve the features all at once. But then I have to implement drop.
    pub(crate) fn set_spatial_filter_rect(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.tiles.layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y)
    }

    pub(crate) fn features(&mut self) -> TileFeatureIterator {
        self.tiles.layer.features().into()
    }

    pub(crate) fn data<Data: TileData>(&mut self) -> TileDataIterator<Data> {
        self.features().into()
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

    pub(crate) fn generate_flowage<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {

        self.with_transaction(|target| {
            let mut tiles = target.edit_tile_layer()?;


            generate_flowage(&mut tiles, progress)?;

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

