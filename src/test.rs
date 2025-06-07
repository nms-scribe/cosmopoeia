

#[test]
fn close_gdal_layer() {
    // NOTE: This test will fail until they release the code with the fix: https://github.com/georust/gdal/pull/420, which will make the close function take ownership so no call to drop.
    use std::path::PathBuf;
    use gdal::Dataset;
    use gdal::GdalOpenFlags;
    use gdal::DriverManager;

    let test_file: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("tmp").join("test_close_gdal_layer.gpkg");

    let ds = if (&test_file).exists() {
        Dataset::open_ex(&test_file, gdal::DatasetOptions { 
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
            ..Default::default()
        }).expect("open dataset")
    } else {
        let driver = DriverManager::get_driver_by_name("GPKG").expect("get driver");
        driver.create_vector_only(&test_file).expect("create dataset")
    };


    ds.close().expect("Should have closed"); // Get a segmentation fault if this is called, I'm guessing they do it again on drop.


}

#[test]
fn test_run_command() {
    use std::path::PathBuf;
    use std::ffi::OsString;

    let cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_file = cargo_dir.join("share").join("qgis").join("World.gpkg");
    
    crate::run(&[
        OsString::from(""),
        "big-bang".into(),
        test_file.into(),
        "--world-shape".into(),
        "sphere".into(),
        "--overwrite-all".into(),
        "--cultures".into(),
        "share/culture_sets/afmg_culture_antique.json".into(),
        "--namers".into(),
        "share/namers/afmg_namers.json".into(),
        "--default-namer".into(),
        "English".into(),
        "--seed".into(),
        "9543572450198918714".into(),
        "blank".into(),
        "180".into(),
        "360".into(),
        "-90".into(),
        "-180".into(),
        "recipe-set".into(),
        "--source".into(),
        "share/terrain_recipes/afmg_recipes.json".into(),
        "--recipe".into(),
        "shattered".into(),
    ]).expect("Command should have run.");
    

}


#[test]
// This appears to be fixed.
//#[should_panic(expected="create should not return an an error here, but it does for now: OgrError { err: 6, method_name: \"OGR_L_CreateFeature\" }")]
fn test_database_lock_issue() {
    use std::path::PathBuf;
    use gdal::Dataset;
    use gdal::GdalOpenFlags;
    use gdal::DriverManager;
    use gdal::vector::LayerOptions;
    use gdal::vector::OGRwkbGeometryType;
    use gdal::vector::LayerAccess;
    use gdal::vector::Feature;
    use gdal::spatial_ref::SpatialRef;

    fn edit_dataset(test_file: PathBuf, finish_loop: bool) {
        let mut dataset = if (&test_file).exists() {
            Dataset::open_ex(&test_file, gdal::DatasetOptions { 
                open_flags: GdalOpenFlags::GDAL_OF_UPDATE, 
                ..Default::default()
            }).expect("open dataset")
        } else {
            let driver = DriverManager::get_driver_by_name("GPKG").expect("get driver");
            driver.create_vector_only(&test_file).expect("create dataset")
        };
    
        let mut transaction = dataset.start_transaction().expect("start transaction");
    
        const RIVERS: &str = "rivers";
        let mut rivers = transaction.create_layer(LayerOptions {
                    name: RIVERS,
                    ty: OGRwkbGeometryType::wkbNone,
                    srs: None,
                    options: Some(&["OVERWRITE=YES"])
                }).expect("create layer");
        rivers.create_defn_fields(&[]).expect("define fields");
    
        { // put in a block so we can borrow rivers as mutable again.
            let feature = Feature::new(&rivers.defn()).expect("new feature");
            feature.create(&rivers).expect("create feature");
        }
    
        // I think this is where the problem is...
        for _ in rivers.features() {
            // break early...
            if !finish_loop {
                break;
            }
        };
        //gdal::vector::LayerAccess::reset_feature_reading(&mut rivers); // FIX IS HERE!
    
    
        const COASTLINES: &str = "coastlines";
        let coastlines = transaction.create_layer(LayerOptions {
            name: COASTLINES,
            ty: OGRwkbGeometryType::wkbPolygon,
            srs: Some(&SpatialRef::from_epsg(4326).expect("srs")),
            options: Some(&["OVERWRITE=YES"])
        }).expect("create layer");
    
        coastlines.create_defn_fields(&[]).expect("defn_fields");
    
        { // in a block so transaction can be borrowed again.
            let feature = Feature::new(coastlines.defn()).expect("new feature");
            // ERROR HAPPENS HERE.
            feature.create(&coastlines).expect("create should not return an an error here, but it does for now");
        }
    
        transaction.commit().expect("commit");
    
        dataset.flush_cache().expect("flush");
    }
    

    let test_file: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("tmp").join("test_database_locked.gpkg");
    
    // delete the file if it exists so I can rerun the test.
    _ = std::fs::remove_file(test_file.clone()); // ignore error
    edit_dataset(test_file.clone(),false);
    // this one doesn't cause the error, so it's not because the database already exists...
    edit_dataset(test_file.clone(),true);
    // this one will error
    edit_dataset(test_file,false)


}

#[test]
fn split_and_remove_chars_should_be_the_same() {
    use crate::utils::remove_n_chars_from_end;
    use crate::utils::split_string_from_end;

    let str1 = "This is a test";
    let mut str2 = str1.to_owned();
    let test_count = 3;
    remove_n_chars_from_end(&mut str2, test_count);
    let str1 = split_string_from_end(str1, test_count).0;
    assert_eq!(str2,str1);
    let test_count = 1;
    remove_n_chars_from_end(&mut str2, test_count);
    let str1 = split_string_from_end(str1, test_count).0;
    assert_eq!(str2,str1);

}