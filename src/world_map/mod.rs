use std::path::Path;
use std::path::PathBuf;

use gdal::Dataset;
use gdal::DatasetOptions;
use gdal::DriverManager;
use gdal::GdalOpenFlags;
use gdal::vector::Transaction;

use crate::commands::OverwriteBiomesArg;
use crate::commands::OverwriteCoastlineArg;
use crate::commands::OverwriteCulturesArg;
use crate::commands::OverwriteLakesArg;
use crate::commands::OverwriteNationsArg;
use crate::commands::OverwriteOceanArg;
use crate::commands::OverwriteRiversArg;
use crate::commands::OverwriteSubnationsArg;
use crate::commands::OverwriteTilesArg;
use crate::commands::OverwriteTownsArg;
use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::world_map::auxiliary_layers::PointLayer;
use crate::world_map::auxiliary_layers::TriangleLayer;
use crate::world_map::biome_layer::BiomeLayer;
use crate::world_map::culture_layer::CultureLayer;
use crate::world_map::nation_layers::NationLayer;
use crate::world_map::nation_layers::SubnationLayer;
use crate::world_map::property_layer::PropertyLayer;
use crate::world_map::tile_layer::TileLayer;
use crate::world_map::town_layer::TownLayer;
use crate::world_map::water_layers::CoastlineLayer;
use crate::world_map::water_layers::LakeLayer;
use crate::world_map::water_layers::OceanLayer;
use crate::world_map::water_layers::RiverLayer;



// FUTURE: It would be really nice if the Gdal stuff were more type-safe. Right now, I could try to add a Point to a Polygon layer, or a Line to a Multipoint geometry, or a LineString instead of a LinearRing to a polygon, and I wouldn't know what the problem is until run-time. 
// The solution to this would probably require rewriting the gdal crate, so I'm not going to bother with this at this time, I'll just have to be more careful. 
// A fairly easy solution is to present a struct Geometry<Type>, where Type is an empty struct or a const numeric type parameter. Then, impl Geometry<Polygon> or Geometry<Point>, etc. This is actually an improvement over the geo_types crate as well. When creating new values of the type, the geometry_type of the inner pointer would have to be validated, possibly causing an error. But it would happen early in the program, and wouldn't have to be checked again.

// FUTURE: Another problem with the gdal crate is the lifetimes. Feature, for example, only requires the lifetimes because it holds a reference to 
// a field definition pointer, which is never used except in the constructor. Once the feature is created, this reference could easily be forgotten. Layer is
// a little more complex, it holds a phantom value of the type of a reference to its dataset. On the one hand, it also doesn't do anything with it at all,
// on the other this reference might keep it from outliving it's dataset reference. Which, I guess, is the same with Feature, so maybe that's what they're 
// doing. I just wish there was another way, as it would make the TypedFeature stuff I'm trying to do below work better. However, if that were built into
// the gdal crate, maybe it would be better.

pub(crate) mod fields;

pub(crate) mod auxiliary_layers;
pub(crate) mod tile_layer;
pub(crate) mod water_layers;
pub(crate) mod biome_layer;
pub(crate) mod culture_layer;
pub(crate) mod town_layer;
pub(crate) mod nation_layers;
pub(crate) mod property_layer;


/*
// Uncomment this stuff if you need to add a line layer for playing around with something.
feature!(Line["lines"]: LineString {
});

pub(crate) type LineLayer<'layer,'feature> = MapLayer<'layer,'feature,LineSchema,LineFeature<'feature>>;

impl LineLayer<'_,'_> {

     pub(crate) fn add_line(&mut self, line: &Vec<Point>) -> Result<u64,CommandError> {
        let geometry = crate::utils::create_line(line)?;
        self.add_feature(geometry, &[], &[])
    }
}
*/



pub(crate) struct WorldMap {
    //path: PathBuf, Removed after reedit bug was fixed
    dataset: Dataset
}

impl WorldMap {

    const GDAL_DRIVER: &'static str = "GPKG";

    fn new(dataset: Dataset/* , path: PathBuf*/) -> Self {
        Self { 
            //path, 
            dataset 
        }
    }

    fn open_dataset<FilePath: AsRef<Path>>(path: &FilePath) -> Result<Dataset, CommandError> {
        Ok(Dataset::open_ex(path, DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        })?)
    }

    pub(crate) fn edit<FilePath: AsRef<Path> + Into<PathBuf>>(path: &FilePath) -> Result<Self,CommandError> {
        Ok(Self::new(Self::open_dataset(path)?/*,path.into()*/))
    }

    pub(crate) fn create_or_edit<FilePath: AsRef<Path> + Into<PathBuf>>(path: &FilePath) -> Result<Self,CommandError> {
        if path.as_ref().exists() {
            Self::edit(path)
        } else {
            let driver = DriverManager::get_driver_by_name(Self::GDAL_DRIVER)?;
            let dataset = driver.create_vector_only(path)?;
            Ok(Self::new(dataset/*,path.into()*/))
        }

    }

    // This had been created to work around a bug which caused a database locked error in specific situations. That bug is fixed.
    //pub(crate) fn reedit(self) -> Result<Self,CommandError> {
    //    // This function is necessary to work around a bug in big-bang that reminds me of days long before rust and I don't want to investigate further.
    //    self.dataset.close()?;
    //    Self::edit(self.path)
    //}

    pub(crate) fn with_transaction<ResultType, Callback: FnOnce(&mut WorldMapTransaction) -> Result<ResultType,CommandError>>(&mut self, callback: Callback) -> Result<ResultType,CommandError> {
        let transaction = self.dataset.start_transaction()?;
        let mut transaction = WorldMapTransaction::new(transaction);
        match callback(&mut transaction) {
            Ok(result) => {
                transaction.dataset.commit()?;
                Ok(result)
            },
            Err(err) => {
                transaction.dataset.rollback()?;
                Err(err)
            },
        }

    }

    pub(crate) fn save<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<(),CommandError> {
        progress.start_unknown_endpoint(|| "Saving map."); 
        self.dataset.flush_cache()?;
        progress.finish(|| "Map saved."); 
        Ok(())
    }

    pub(crate) fn points_layer(&self) -> Result<PointLayer,CommandError> {
        PointLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn tiles_layer(&self) -> Result<TileLayer,CommandError> {
        TileLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn biomes_layer(&self) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn cultures_layer(&self) -> Result<CultureLayer, CommandError> {
        CultureLayer::open_from_dataset(&self.dataset)
    }



 

}

pub(crate) struct WorldMapTransaction<'data_life> {
    dataset: Transaction<'data_life>
}

impl<'impl_life> WorldMapTransaction<'impl_life> {

    fn new(dataset: Transaction<'impl_life>) -> Self {
        Self {
            dataset
        }
    }

    pub(crate) fn create_points_layer(&mut self, overwrite: bool) -> Result<PointLayer,CommandError> {
        PointLayer::create_from_dataset(&mut self.dataset, overwrite)       

    }

    pub(crate) fn create_triangles_layer(&mut self, overwrite: bool) -> Result<TriangleLayer,CommandError> {
        TriangleLayer::create_from_dataset(&mut self.dataset, overwrite)

    }

    pub(crate) fn edit_triangles_layer(&self) -> Result<TriangleLayer, CommandError> {
        TriangleLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_tile_layer(&mut self, overwrite: &OverwriteTilesArg) -> Result<TileLayer,CommandError> {
        TileLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_tiles)

    }

    pub(crate) fn create_rivers_layer(&mut self, overwrite: &OverwriteRiversArg) -> Result<RiverLayer,CommandError> {
        RiverLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_rivers)

    }

    pub (crate) fn create_lakes_layer(&mut self, overwrite_layer: &OverwriteLakesArg) -> Result<LakeLayer,CommandError> {
        LakeLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_lakes)
    }

    pub (crate) fn edit_lakes_layer(&mut self) -> Result<LakeLayer,CommandError> {
        LakeLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn edit_tile_layer(&mut self) -> Result<TileLayer,CommandError> {
        TileLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_biomes_layer(&mut self, overwrite: &OverwriteBiomesArg) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_biomes)
    }

    pub(crate) fn edit_biomes_layer(&mut self) -> Result<BiomeLayer,CommandError> {
        BiomeLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_cultures_layer(&mut self, overwrite: &OverwriteCulturesArg) -> Result<CultureLayer,CommandError> {
        CultureLayer::create_from_dataset(&mut self.dataset, overwrite.overwrite_cultures)
    }

    pub(crate) fn edit_cultures_layer(&mut self) -> Result<CultureLayer,CommandError> {
        CultureLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_towns_layer(&mut self, overwrite_layer: &OverwriteTownsArg) -> Result<TownLayer,CommandError> {
        TownLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_towns)
    }

    pub(crate) fn edit_towns_layer(&mut self) -> Result<TownLayer,CommandError> {
        TownLayer::open_from_dataset(&self.dataset)

    }

    pub(crate) fn create_nations_layer(&mut self, overwrite_layer: &OverwriteNationsArg) -> Result<NationLayer,CommandError> {
        NationLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_nations)
    }

    pub(crate) fn edit_nations_layer(&mut self) -> Result<NationLayer,CommandError> {
        NationLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_subnations_layer(&mut self, overwrite_layer: &OverwriteSubnationsArg) -> Result<SubnationLayer,CommandError> {
        SubnationLayer::create_from_dataset(&mut self.dataset, overwrite_layer.overwrite_subnations)
    }

    pub(crate) fn edit_subnations_layer(&mut self) -> Result<SubnationLayer,CommandError> {
        SubnationLayer::open_from_dataset(&self.dataset)
    }

    pub(crate) fn create_coastline_layer(&mut self, overwrite_coastline: &OverwriteCoastlineArg) -> Result<CoastlineLayer,CommandError> {
        CoastlineLayer::create_from_dataset(&mut self.dataset, overwrite_coastline.overwrite_coastline)
    }

    pub(crate) fn create_ocean_layer(&mut self, overwrite_ocean: &OverwriteOceanArg) -> Result<OceanLayer,CommandError> {
        OceanLayer::create_from_dataset(&mut self.dataset, overwrite_ocean.overwrite_ocean)
    }

    /* Uncomment this to add a line layer for playing around with ideas.
     pub(crate) fn create_lines_layer(&mut self, overwrite: bool) -> Result<LineLayer,CommandError> {
        Ok(LineLayer::create_from_dataset(&mut self.dataset, overwrite)?)
    }
    */

    pub(crate) fn create_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        PropertyLayer::create_from_dataset(&mut self.dataset,true)
    }

    pub(crate) fn edit_properties_layer(&mut self) -> Result<PropertyLayer,CommandError> {
        PropertyLayer::open_from_dataset(&self.dataset)
    }

}
