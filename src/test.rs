#[test]
fn close_gdal_layer() {
    // NOTE: This test will fail until they release the code with the fix: https://github.com/georust/gdal/pull/420, which will make the close function take ownership so no call to drop.
    use std::path::PathBuf;
    use gdal::Dataset;

    let ds = Dataset::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample_data").join("wisconsin.tif")).expect("Dataset should have opened.");
    ds.close().expect("Should have closed"); // Get a segmentation fault if this is called, I'm guessing they do it again on drop.


}

#[test]
fn test_bezier() {
    use crate::utils::Point;
    use crate::utils::PolyBezier;

    let pos = vec![
        Point::from_f64(0.5, 0.5).unwrap(),
        Point::from_f64(1.0, -0.5).unwrap(),
        Point::from_f64(1.5, 1.0).unwrap(),
        Point::from_f64(2.25, 1.1).unwrap(),
        Point::from_f64(2.6, -0.5).unwrap(),
        Point::from_f64(3.0, 0.5).unwrap(),
    ];

    let curves = PolyBezier::from_poly_line(&pos);

    let expected = vec![
        (
            Point::from_f64(0.5, 0.5).unwrap(),
            Point::from_f64(0.6666666666666666, 0.16666666666666669).unwrap(),
            Point::from_f64(0.6666666666666667, -0.66666666666666666).unwrap(),
            Point::from_f64(1.0, -0.5).unwrap(),
        ),
        (
            Point::from_f64(1.0, -0.5).unwrap(),
            Point::from_f64(1.4714045207910318, -0.26429773960448416).unwrap(), 
            Point::from_f64(1.1755270999091973, 0.5846746878837725).unwrap(), 
            Point::from_f64(1.5, 1.0).unwrap(),
        ),
        (
            Point::from_f64(1.5, 1.0).unwrap(),
            Point::from_f64(1.655273081384295, 1.1987495441718978).unwrap(), 
            Point::from_f64(2.100850731900237, 1.3033853655905858).unwrap(), 
            Point::from_f64(2.25, 1.1).unwrap(),
        ),
        (
            Point::from_f64(2.25, 1.1).unwrap(),
            Point::from_f64(2.572851825487011, 0.6597475106995304).unwrap(), 
            Point::from_f64(2.1736888549287925, -0.15895108394303398).unwrap(), 
            Point::from_f64(2.6, -0.5).unwrap(),
        ),
        (
            Point::from_f64(2.6, -0.5).unwrap(),
            Point::from_f64(2.8803404821067753, -0.7242723856854201).unwrap(), 
            Point::from_f64(2.8666666666666667, 0.16666666666666669).unwrap(), 
            Point::from_f64(3.0, 0.5).unwrap(),
        )
    ];

    let mut i = 0;
    while let Some(curve) = curves.segment_at(i) {
        let expected_curve = &expected[i];
        assert_eq!(curve.0,&expected_curve.0,"At curve {}, point 0",i);
        assert_eq!(curve.1,&expected_curve.1,"At curve {}, point 1",i);
        assert_eq!(curve.2,&expected_curve.2,"At curve {}, point 2",i);
        assert_eq!(curve.3,&expected_curve.3,"At curve {}, point 3",i);
        i += 1;
    }

/* python output:
[
[
(0.500000000000000, 0.500000000000000), 
(0.666666666666667, 0.166666666666667), 
(0.666666666666667, -0.666666666666667), 
(1.00000000000000, -0.500000000000000)
], [
(1.47140452079103, -0.264297739604484), 
(1.17552709990920, 0.584674687883773), 
(1.50000000000000, 1.00000000000000)
], [
(1.65527308138430, 1.19874954417190), 
(2.10085073190024, 1.30338536559059), 
(2.25000000000000, 1.10000000000000)
], [
(2.57285182548701, 0.659747510699530), 
(2.17368885492879, -0.158951083943034), 
(2.60000000000000, -0.500000000000000)
], [
(2.88034048210678, -0.724272385685420), 
(2.86666666666667, 0.166666666666667), 
(3.00000000000000, 0.500000000000000)
]
]
*/

}

