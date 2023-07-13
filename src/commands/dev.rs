use std::path::PathBuf;

// "Dev" commands are generally hidden, intended for testing during development. While they should be usable by users, they rarely are, and are hidden to keep the UI clean.

use clap::Args;
use gdal::Dataset;
use gdal::DriverManager;
use gdal::version::VersionInfo;
use gdal::Metadata;
use gdal::MetadataEntry;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;

subcommand_def!{
    /// Opens a GDAL file and gets some information.
    #[command(hide=true)]
    pub struct DevGdalInfo {
        /// Name of user to greet.
        source: PathBuf
    }
}

impl Task for DevGdalInfo {

    fn run(self) -> Result<(),CommandError> {
        let ds = Dataset::open(self.source)?;
        println!("projection: {}",ds.projection()); 
        //println!("spatial reference: {:?}",ds.spatial_ref()?); // TODO: This causes an error in gpkg
        //println!("geotransform: {:?}",ds.geo_transform()?); // TODO: This causes an error in gpkg
        println!("layer count: {}",ds.layer_count()); // If the file is a vector, this will be > 0
        println!("raster band count: {}",ds.raster_count()); // If the file is a raster, this will be > 0
        println!("raster size: {:?}",ds.raster_size()); // I see this as 512,512 for vector.
        println!("metadata:");
        for MetadataEntry { domain, key, value } in ds.metadata() {
            let domain = if domain == "" { "DEFAULT".to_string() } else { domain };
            println!("{domain}: {key}={value}");
        }
        println!("driver: {}",ds.driver().long_name());
        println!("driver metadata:");
        for MetadataEntry { domain, key, value } in ds.driver().metadata() {
            let domain = if domain == "" { "DEFAULT".to_string() } else { domain };
            println!("{domain}: {key}={value}");
        }
        Ok(())
    }
}

subcommand_def!{
    /// Opens a GDAL file and gets some information.
    #[command(hide=true)]
    pub struct DevGdalVersion {
    }
}

impl Task for DevGdalVersion {

    fn run(self) -> Result<(),CommandError> {
        println!("{}",VersionInfo::version_report());
        Ok(())
    }
}

subcommand_def!{
    /// Opens a GDAL file and gets some information.
    #[command(hide=true)]
    pub struct DevGdalDrivers {
    }
}

impl Task for DevGdalDrivers {

    fn run(self) -> Result<(),CommandError> {
        let mut drivers = Vec::new();
        for i in 0..DriverManager::count() {
            let driver = DriverManager::get_driver(i)?;
            drivers.push((driver.short_name(),driver.long_name(),driver.description()?))
        }
        drivers.sort_by(|(a,_,_),(b,_,_)| a.cmp(b));
        for (a,b,c) in drivers {
            println!("{}: ({}) {}",a,b,c);
        }
        Ok(())
    }
}


subcommand_def!{
    /// Creates a random points vector layer from a raster heightmap
    #[command(hide=true)]
    pub struct DevPointsFromHeightmap {
        source: PathBuf,

        target: PathBuf,

        #[arg(long)]
        target_driver: String,
        #[arg(long)]
        density: Option<usize>
    }
}

impl Task for DevPointsFromHeightmap {

    fn run(self) -> Result<(),CommandError> {
        let source = Dataset::open(self.source)?;
        let target_driver = DriverManager::get_driver_by_name(&self.target_driver)?;
        let target = target_driver.create_vector_only(self.target)?;



        println!("{}",VersionInfo::version_report());
        Ok(())
    }
}

