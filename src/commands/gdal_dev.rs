use std::path::PathBuf;


use clap::Args;
use clap::Subcommand;
use gdal::Dataset;
use gdal::DriverManager;
use gdal::version::VersionInfo;
use gdal::Metadata;
use gdal::MetadataEntry;

use super::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::command_def;
use crate::progress::ProgressObserver;

subcommand_def!{
    /// Opens a GDAL file and gets some information.
    pub struct DatasetInfo {
        /// Name of user to greet.
        pub source: PathBuf
    }
}

impl Task for DatasetInfo {

    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
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
    pub struct Version {
    }
}

impl Task for Version {

    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
        println!("{}",VersionInfo::version_report());
        Ok(())
    }
}

subcommand_def!{
    /// Opens a GDAL file and gets some information.
    pub struct Drivers {
    }
}

impl Task for Drivers {

    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
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

command_def!(
    #[command(disable_help_subcommand(true))]
    pub GdalCommand {
        DatasetInfo,
        Version,
        Drivers 
    }
);


subcommand_def!{
    /// Retrieves information about local gdal library
    #[command(hide=true)]
    pub struct Gdal {
        #[command(subcommand)]
        pub command: GdalCommand

    }
}

impl Task for Gdal {
    fn run<Progress: ProgressObserver>(self, progress: &mut Progress) -> Result<(),CommandError> {
        self.command.run(progress)        
    }
}