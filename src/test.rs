#[test]
fn close_gdal_layer() {
    // NOTE: This test will fail until they release the code with the fix: https://github.com/georust/gdal/pull/420, which will make the close function take ownership so no call to drop.
    use std::path::PathBuf;
    use gdal::Dataset;

    let ds = Dataset::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample_data").join("wisconsin.tif")).expect("Dataset should have opened.");
    ds.close().expect("Should have closed"); // Get a segmentation fault if this is called, I'm guessing they do it again on drop.


}
