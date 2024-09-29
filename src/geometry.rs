use gdal::vector::Geometry as GDALGeometry;
use gdal::vector::OGRwkbGeometryType;

use crate::errors::CommandError;
use geo::ChamberlainDuquetteArea;
use crate::algorithms::beziers::bezierify_points;
use crate::utils::coordinates::Coordinates;
use crate::utils::extent::Extent;
use crate::utils::world_shape::WorldShape;
use gdal::cpl::CslStringList;
use geo_types::Error as GeoTypesError;
use core::marker::PhantomData;
use gdal::vector::Envelope;

impl From<Envelope> for Extent {
    fn from(value: Envelope) -> Self {
        Self::from_bounds(value.MinX, value.MinY, value.MaxX, value.MaxY)
    }
}

// special wrapper for the geo type.
pub(crate) trait ChamberlainDuquetteAreaInDegrees {

    fn chamberlain_duquette_area_in_degrees(&self) -> f64;
    
}

impl<CDA: ChamberlainDuquetteArea<f64>> ChamberlainDuquetteAreaInDegrees for CDA {
    fn chamberlain_duquette_area_in_degrees(&self) -> f64 {
        let result = self.chamberlain_duquette_unsigned_area();
            
        // result of above is multiplied twice by 6378137.0 to get square meters. This number is the equatorial earth radius in meters.
        // I need it in square degrees. 
    
        // If the earth radius 6378137.0, then it's circumference is 40075017 at the equator. And a degree along the equator is 111319.49 meters.
        // This means that my square degree unit is 111319.49^2 m2, or 12392029000 m2.
        // So, to convert from square meters to square degrees, I need to divide the result by 12392029000
        result/12392029000.0
    }
}

pub(crate) trait GDALGeometryWrapper: TryFrom<GDALGeometry,Error=CommandError> + Into<GDALGeometry> {

    const INTERNAL_TYPE: OGRwkbGeometryType::Type;

    fn get_envelope(&self) -> Extent;

    fn is_valid(&self) -> bool;

}

macro_rules! non_collection_geometry {
    ($struct: ident, $geo_type: ident) => {

        #[derive(Clone,Debug)]
        pub(crate) struct $struct {
            inner: GDALGeometry,
        }

        impl $struct {
            // these methods can't be implemented as a trait because they need access to the 'inner' member.

            // private method, use TryFrom to actually try to convert
            fn try_from_gdal(value: GDALGeometry) -> Result<Self,CommandError> {
                let found = value.geometry_type();
                if found == OGRwkbGeometryType::$geo_type {
                    Ok(Self {
                        inner: value
                    })
                } else {
                    Err(CommandError::IncorrectGdalGeometryType{ 
                        expected: OGRwkbGeometryType::$geo_type, 
                        found
                    })
                }

            }

            // internal function for constructing a blank, but empty and therefore incorrect, value which then gets filled in by constructor
            fn blank() -> Result<Self,CommandError> {
                let inner = GDALGeometry::empty(OGRwkbGeometryType::$geo_type)?;
                Ok(Self {
                    inner
                })
            }

        }


        impl GDALGeometryWrapper for $struct {


            const INTERNAL_TYPE: gdal::vector::OGRwkbGeometryType::Type = OGRwkbGeometryType::$geo_type;

            fn get_envelope(&self) -> Extent {
                let envelope = self.inner.envelope();
                envelope.into()
            }

            fn is_valid(&self) -> bool {
                // FUTURE: This writes text to stdout if it isn't valid. Is there a way to fix that?
                self.inner.is_valid()
            }


        }
        
        impl TryFrom<GDALGeometry> for $struct {

            type Error = CommandError;

            fn try_from(value: GDALGeometry) -> Result<Self,Self::Error> {
                Self::try_from_gdal(value)
            }
        
        }

        impl From<$struct> for GDALGeometry {

            fn from(value: $struct) -> GDALGeometry {
                value.inner
            }
        }
        
    };
}



non_collection_geometry!(Point,wkbPoint);

impl Point {

    pub(crate) fn new(x: f64, y: f64) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        this.inner.add_point_2d((x,y));
        Ok(this)
    }

}

non_collection_geometry!(LineString,wkbLineString);

impl LineString {

    pub(crate) fn from_vertices<Items: IntoIterator<Item=(f64,f64)>>(vertices: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for point in vertices {
            this.push_point(point)
        }
        Ok(this)
    
    }

    pub(crate) fn get_point(&self, index: usize) -> (f64,f64) {
        let (x,y,_) = self.inner.get_point(index as i32);
        (x,y)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.point_count()
    }

    #[allow(dead_code)] pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn push_point(&mut self, point: (f64,f64)) {
        self.inner.add_point_2d(point)
    }



}

pub(crate) struct LineStringIter {
    inner: LineString,
    position: usize
}

impl Iterator for LineStringIter {

    type Item = (f64,f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.inner.len() {
            let result = self.inner.get_point(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This is how size_hint is supposed to work.
        let count = self.inner.len();
        let remaining = count - self.position;
        (remaining,Some(remaining))
    }
}

impl IntoIterator for LineString {
    type Item = (f64,f64);

    type IntoIter = LineStringIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self,
            position: 0
        }
    }
}

non_collection_geometry!(MultiLineString,wkbMultiLineString);

impl MultiLineString {


    pub(crate) fn from_lines<Items: IntoIterator<Item = Result<LineString,CommandError>>>(lines: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for line in lines {
            this.push_line(line?)?
        }
        Ok(this)
    }

    #[allow(dead_code)] pub(crate) fn len(&self) -> usize {
        self.inner.geometry_count()
    }

    #[allow(dead_code)] pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)] pub(crate) fn get_line(&self, index: usize) -> Result<LineString,CommandError> {
        let line = self.inner.get_geometry(index);
        LineString::try_from(line.clone()) // FUTURE: Unfortunately, GeometryRef is inaccessible, which might mess with performance a little.
    }

    pub(crate) fn push_line(&mut self, line: LineString) -> Result<(),CommandError> {
        Ok(self.inner.add_geometry(line.into())?)
    }

}

non_collection_geometry!(LinearRing,wkbLinearRing);

impl LinearRing {

    pub(crate) fn from_vertices<Items: IntoIterator<Item=(f64,f64)>>(vertices: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for point in vertices {
            this.push_point(point);
        }
        if this.is_empty() {
            // FUTURE: I allow for other empty structures, mostly for cases where a Geometry would be "null", so perhaps
            // I should allow this. But then, I would need some way of automatically validating.
            Err(CommandError::EmptyLinearRing)
        } else if this.get_point(this.len() - 1) != this.get_point(0) {
            Err(CommandError::UnclosedLinearRing)
        } else {
            Ok(this)
        }
    
    }

    pub(crate) fn get_point(&self, index: usize) -> (f64,f64) {
        let (x,y,_) = self.inner.get_point(index as i32);
        (x,y)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.point_count()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn push_point(&mut self, point: (f64,f64)) {
        self.inner.add_point_2d(point)
    }

}


pub(crate) struct LinearRingIter {
    inner: LinearRing,
    position: usize
}

impl Iterator for LinearRingIter {

    type Item = (f64,f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.inner.len() {
            let result = self.inner.get_point(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This is how size_hint is supposed to work.
        let count = self.inner.len();
        let remaining = count - self.position;
        (remaining,Some(remaining))
    }
}

impl IntoIterator for LinearRing {
    type Item = (f64,f64);

    type IntoIter = LinearRingIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self,
            position: 0
        }
    }
}

non_collection_geometry!(Polygon,wkbPolygon);

fn validate_options_structure() -> Result<CslStringList, CommandError> {
    let mut validate_options = CslStringList::new();
    validate_options.add_string("METHOD=STRUCTURE")?;
    Ok(validate_options)
}

// While I could put these on some sort of trait, I'd also need to provide some sort of access to inner in that trait,
// which wouldn't be private, and I don't really want 'inner' to leak. It doesn't belong on wrapper, because it shouldn't be
// possible to run area on a 2D or less feature. (Union might be different)
macro_rules! areal_fns {
    () => {

        pub(crate) fn area(&self) -> f64 {
            self.inner.area()
        }

        pub(crate) fn union(&self, rhs: &Self) -> Result<VariantArealGeometry,CommandError> {
            if let Some(united) = self.inner.union(&rhs.inner) {
                let united = if !united.is_valid() {
                    // I'm writing to stdout here because the is_valid also writes to stdout
                    // FUTURE: I can't use progress.warning here because it is borrowed for mutable, is there another way?
                    eprintln!("fixing invalid union");
                    united.make_valid(&validate_options_structure()?)?
                } else {
                    united
                };
    
                Ok(united.try_into()?)
            } else {
                Err(CommandError::GdalUnionFailed)
            }
    
        }

        pub(crate) fn buffer(&self, distance: f64, n_quad_segs: u32) -> Result<VariantArealGeometry,CommandError> {
            self.inner.buffer(distance, n_quad_segs)?.try_into()
        }
    
        pub(crate) fn simplify(&self, tolerance: f64) -> Result<VariantArealGeometry,CommandError> {
            self.inner.simplify(tolerance)?.try_into()
        }

        #[allow(dead_code)] // not all implementations use this
        pub(crate) fn intersection(&self, rhs: &Self) -> Result<VariantArealGeometry,CommandError> {
            if let Some(intersected) = self.inner.intersection(&rhs.inner) {
                // I haven't seen any broken intersections yet, so I'm not checking validity.
                Ok(intersected.try_into()?)
            } else {
                Err(CommandError::GdalIntersectionFailed)
            }
    
        }

        pub(crate) fn difference(&self, rhs: &Self) -> Result<VariantArealGeometry,CommandError> {
            if let Some(different) = self.inner.difference(&rhs.inner) {
                let different = if !different.is_valid() {
                    // I'm writing to stdout here because the is_valid also writes to stdout
                    // FUTURE: I can't use progress.warning because I've got progress borrowed as mutable. Is there another way?
                    eprintln!("fixing invalid difference");
                    different.make_valid(&validate_options_structure()?)?
                } else {
                    different
                };

                Ok(different.try_into()?)
            } else {
                Err(CommandError::GdalDifferenceFailed)
            }

        }

    
    };
}


impl Polygon {

    pub(crate) fn from_rings<Items: IntoIterator<Item = LinearRing>>(rings: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for ring in rings {
            this.push_ring(ring)?
        }
        Ok(this)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.geometry_count()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn get_ring(&self, index: usize) -> Result<LineString,CommandError> {
        let ring = self.inner.get_geometry(index);
        LineString::try_from(ring.clone()) // FUTURE: Unfortunately, GeometryRef is inaccessible, which might mess with performance a little.
    }

    // For some reason, polygons return LineStrings, but require a LinearRing when building. At least that's what appears to be happening.
    pub(crate) fn push_ring(&mut self, ring: LinearRing) -> Result<(),CommandError> {
        Ok(self.inner.add_geometry(ring.into())?)
    }

    pub(crate) fn to_geo_type(&self) -> Result<geo::Polygon,CommandError> {
        let result: Result<geo::Polygon, geo_types::Error> = self.inner.to_geo()?.try_into();
        match result {
            Ok(polygon) => Ok(polygon),
            Err(err) => match err {
                GeoTypesError::MismatchedGeometry { expected, found } => Err(CommandError::CantConvert{expected,found}),
            },
        }
    }

    // Yes, this should be defined for MultiPolygon, but it's not used for that anywhere. If I ever need to define it, it will be exactly like this.
    pub(crate) fn spherical_area(&self) -> Result<f64,CommandError> {
        Ok(self.to_geo_type()?.chamberlain_duquette_area_in_degrees())
    }

    // Yes, this should be defined for MultiPolygon, but it's not used for that anywhere. If I ever need to define it, it will be exactly like this.
    pub(crate) fn shaped_area(&self, world_shape: &WorldShape) -> Result<f64,CommandError> {
        match world_shape {
            WorldShape::Cylinder => Ok(self.area()),
            WorldShape::Sphere => self.spherical_area()
        }
    }
        

    areal_fns!();

    // NOTE: This returns a variant areal feature, since sometimes it returns MultiPolygons
    pub(crate) fn make_valid_default(&self) -> Result<VariantArealGeometry,CommandError> {
        // Primary cause of invalid geometry that I've noticed: the original dissolved tiles meet at the same point, or a point that is very close.
        // Much preferred would be to snip away the polygon created by the intersection if it's small. FUTURE: Maybe revisit that theory.
        self.inner.make_valid(&CslStringList::new())?.try_into()
    }

    pub(crate) fn make_valid_structure(&self) -> Result<VariantArealGeometry,CommandError> {
        // primarily caused by invalid unions and things.
        self.inner.make_valid(&validate_options_structure()?)?.try_into()
    }

    // NOTE: Theres a small chance that bezierifying will create invalid geometries. These are automatically
    // made valid, which could turn them into a multi-polygon.
    pub(crate) fn bezierify(self, scale: f64) -> Result<VariantArealGeometry,CommandError> {
        let mut rings = Vec::new();
        for ring in self {
            let mut points = Vec::new();
            for point in ring? {
                points.push(point.try_into()?);
            }

            let line = bezierify_points(&points, scale)?;
            rings.push(LinearRing::from_vertices(line.iter().map(Coordinates::to_tuple))?);
        }
        let polygon = Self::from_rings(rings)?;
        // Primary cause of invalid geometry that I've noticed: the original dissolved tiles meet at the same point, or a point that is very close.
        // Much preferred would be to snip away the polygon created by the intersection if it's small. FUTURE: Maybe revisit that theory.
        polygon.make_valid_default()
    }

}


pub(crate) struct PolygonIter {
    inner: Polygon,
    position: usize
}

impl Iterator for PolygonIter {

    // For some reason, polygons return LineStrings, but require a LinearRing when building. At least that's what appears to be happening.
    type Item = Result<LineString,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.inner.len() {
            let result = self.inner.get_ring(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This is how size_hint is supposed to work.
        let count = self.inner.len();
        let remaining = count - self.position;
        (remaining,Some(remaining))
    }
}

impl IntoIterator for Polygon {
    type Item = Result<LineString,CommandError>;

    type IntoIter = PolygonIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self,
            position: 0
        }
    }
}

impl TryFrom<MultiPolygon> for Polygon {

    type Error = CommandError;

    fn try_from(value: MultiPolygon) -> Result<Self,Self::Error> {
        if value.len() == 1 {
            value.get_polygon(0)
        } else {
            Err(CommandError::CantConvertMultiPolygonToPolygon)
        }
    }
}




non_collection_geometry!(MultiPolygon,wkbMultiPolygon);

impl MultiPolygon {

    pub(crate) fn from_polygons<Items: IntoIterator<Item = Polygon>>(polygons: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for polygon in polygons {
            this.push_polygon(polygon)?
        }
        Ok(this)
    }

    pub(crate) fn from_variants<Items: IntoIterator<Item = VariantArealGeometry>>(polygons: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for variant in polygons {
            for polygon in variant {
                this.push_polygon(polygon?)?
            }
        }
        Ok(this)
    }

    pub(crate) fn from_polygon_results<Items: IntoIterator<Item = Result<Polygon,CommandError>>>(polygons: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for polygon in polygons {
            this.push_polygon(polygon?)?
        }
        Ok(this)
    }

    pub(crate) fn from_combined<Items: IntoIterator<Item = Self>>(multis: Items) -> Result<Self,CommandError> {
        let mut this = Self::blank()?;
        for multi in multis {
            for polygon in multi {
                this.push_polygon(polygon?)?
            }
        }
        Ok(this)
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.geometry_count()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn get_polygon(&self, index: usize) -> Result<Polygon,CommandError> {
        let ring = self.inner.get_geometry(index);
        Polygon::try_from(ring.clone()) // FUTURE: Unfortunately, GeometryRef is inaccessible, which might mess with performance a little.
    }

    pub(crate) fn push_polygon(&mut self, polygon: Polygon) -> Result<(),CommandError> {
        Ok(self.inner.add_geometry(polygon.into())?)
    }

    #[allow(dead_code)] // If I ever need to support spherical_area, I will need this.
    pub(crate) fn to_geo_type(&self) -> Result<geo::MultiPolygon,CommandError> {
        let result: Result<geo::MultiPolygon, geo_types::Error> = self.inner.to_geo()?.try_into();
        match result {
            Ok(shape) => Ok(shape),
            Err(err) => match err {
                GeoTypesError::MismatchedGeometry { expected, found } => Err(CommandError::CantConvert{expected,found}),
            },
        }
    }

    areal_fns!();

    // this is different from Polygon::make_valid_default, in that it always returns MultiPolygon, whereas that one could be variant.
    pub(crate) fn make_valid_default(&self) -> Result<Self,CommandError> {
        // Primary cause of invalid geometry that I've noticed: the original dissolved tiles meet at the same point, or a point that is very close.
        // Much preferred would be to snip away the polygon created by the intersection if it's small. FUTURE: Maybe revisit that theory.
        self.inner.make_valid(&CslStringList::new())?.try_into()
    }


    pub(crate) fn bezierify(self, scale: f64) -> Result<Self,CommandError> {
        let mut polygons = Vec::new();
        for polygon in self {
            let mut rings = Vec::new();
            for ring in polygon? {
                let mut points = Vec::new();
                for point in ring? {
                    points.push(point.try_into()?);
                }
                
                let line = bezierify_points(&points, scale)?;
                rings.push(LinearRing::from_vertices(line.iter().map(Coordinates::to_tuple))?);
            }
            polygons.push(Polygon::from_rings(rings)?);
        }
        let result = Self::from_polygons(polygons)?;
        // Primary cause of invalid geometry that I've noticed: the original dissolved tiles meet at the same point, or a point that is very close.
        // Much preferred would be to snip away the polygon created by the intersection if it's small. FUTURE: Maybe revisit that theory.
        result.make_valid_default()
    }
    
    
}


pub(crate) struct MultiPolygonIter {
    inner: MultiPolygon,
    position: usize
}

impl Iterator for MultiPolygonIter {

    type Item = Result<Polygon,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.inner.len() {
            let result = self.inner.get_polygon(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This is how size_hint is supposed to work.
        let count = self.inner.len();
        let remaining = count - self.position;
        (remaining,Some(remaining))
    }
}

impl IntoIterator for MultiPolygon {
    type Item = Result<Polygon,CommandError>;

    type IntoIter = MultiPolygonIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self,
            position: 0
        }
    }
}

impl TryFrom<Polygon> for MultiPolygon {

    type Error = CommandError;

    fn try_from(value: Polygon) -> Result<Self,Self::Error> {
        Self::from_polygons([value])
    }
}


// At this point I'm not implementing GDALGeometryWrapper for all Collections, just Variant collections.
// The other ones are more difficult, as I'd have to validate every single geometry in the collection
// in TryFrom.
pub(crate) struct Collection<ItemType: GDALGeometryWrapper> {
    inner: GDALGeometry,
    _marker: PhantomData<ItemType>
}

impl<ItemType: GDALGeometryWrapper> From<Collection<ItemType>> for GDALGeometry {

    fn from(value: Collection<ItemType>) -> Self {
        value.inner
    }
}

impl GDALGeometryWrapper for Collection<VariantGeometry> {

    const INTERNAL_TYPE: OGRwkbGeometryType::Type = OGRwkbGeometryType::wkbGeometryCollection;

    fn get_envelope(&self) -> Extent {
        let envelope = self.inner.envelope();
        envelope.into()
    }

    fn is_valid(&self) -> bool {
        // FUTURE: This writes text to stdout. Is there a way to fix that?
        self.inner.is_valid()
    }


}

impl TryFrom<GDALGeometry> for Collection<VariantGeometry> {

    type Error = CommandError;

    fn try_from(value: GDALGeometry) -> Result<Self,Self::Error> {
        Self::try_from_gdal(value)
    }

}

impl Collection<VariantGeometry> {

    // private method, use TryFrom to actually try to convert
    fn try_from_gdal(value: GDALGeometry) -> Result<Self,CommandError> {
        let found = value.geometry_type();
        if found == OGRwkbGeometryType::wkbGeometryCollection {
            Ok(Self {
                inner: value,
                _marker: PhantomData
            })
        } else {
            Err(CommandError::IncorrectGdalGeometryType{ 
                expected: OGRwkbGeometryType::wkbGeometryCollection, 
                found
            })
        }

    }

}



impl Collection<Point> {

    pub(crate) fn delaunay_triangulation(&self, tolerance: Option<f64>) -> Result<Collection<Polygon>, CommandError> {
        let inner = self.inner.delaunay_triangulation(tolerance)?;
        Collection::reluctantly_try_from_gdal(inner)
    }

}

impl<ItemType: GDALGeometryWrapper> Collection<ItemType> {

    // private method, don't use this anywhere, it doesn't check the contents. Use Collection<VariantGeometry>::try_from instead.
    // I only use it in cases where I know what the geometry is going to be.
    fn reluctantly_try_from_gdal(value: GDALGeometry) -> Result<Self,CommandError> {
        let found = value.geometry_type();
        if found == OGRwkbGeometryType::wkbGeometryCollection {
            Ok(Self {
                inner: value,
                _marker: PhantomData
            })
        } else {
            Err(CommandError::IncorrectGdalGeometryType{ 
                expected: OGRwkbGeometryType::wkbGeometryCollection, 
                found
            })
        }

    }

    pub(crate) fn new() -> Result<Self,CommandError> {
        let inner = GDALGeometry::empty(OGRwkbGeometryType::wkbGeometryCollection)?;
        Ok(Self {
            inner,
            _marker: PhantomData
        })

    }

    #[allow(dead_code)]
    pub(crate) fn from_geometries<Items: IntoIterator<Item = ItemType>>(geometries: Items) -> Result<Self,CommandError> {
        let mut inner = GDALGeometry::empty(OGRwkbGeometryType::wkbGeometryCollection)?;
        for geometry in geometries {
            inner.add_geometry(geometry.into())?
        }
        Ok(Self {
            inner,
            _marker: PhantomData
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.geometry_count()
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn get_item(&self, index: usize) -> Result<ItemType,CommandError> {
        let geometry = self.inner.get_geometry(index);
        ItemType::try_from(geometry.clone()) // FUTURE: Unfortunately, GeometryRef is inaccessible, which might mess with performance a little.
    }

    pub(crate) fn push_item(&mut self, geometry: ItemType) -> Result<(),CommandError> {
        Ok(self.inner.add_geometry(geometry.into())?)
    }

    // implementations of GDALGeometryWrapper for use in Variant, because it is impossible to implement it directly due to nested types
    #[allow(clippy::same_name_method)]
    fn get_envelope(&self) -> Extent {
        let envelope = self.inner.envelope();
        envelope.into()
    }

    #[allow(clippy::same_name_method)]
    fn is_valid(&self) -> bool {
        // FUTURE: This writes text to stdout if it isn't valid. Is there a way to fix that?
        self.inner.is_valid()
    }

}




pub(crate) struct CollectionIter<ItemType: GDALGeometryWrapper> {
    inner: Collection<ItemType>,
    position: usize
}

impl<ItemType: GDALGeometryWrapper> Iterator for CollectionIter<ItemType> {

    type Item = Result<ItemType,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.inner.len() {
            let result = self.inner.get_item(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // This is how size_hint is supposed to work.
        let count = self.inner.len();
        let remaining = count - self.position;
        (remaining,Some(remaining))
    }
}

impl<ItemType: GDALGeometryWrapper> IntoIterator for Collection<ItemType> {
    type Item = Result<ItemType,CommandError>;

    type IntoIter = CollectionIter<ItemType>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self,
            position: 0
        }
    }
}

macro_rules! impl_variants {
    ($enum: ty, $variant: ident, $struct: ty) => {
        impl From<$struct> for $enum {

            fn from(value: $struct) -> Self {
                Self::$variant(value)
            }


        }

    };
}

macro_rules! impl_variant_geometry {
    ($enum: ident { $( $variant: ident( $struct: ty) $([$type: ident])?),*$(,)? }) => {
                
        pub(crate) enum $enum {
            $( $variant($struct)),*
        }

        impl GDALGeometryWrapper for $enum {

            // If you've got a Variant type, don't use this INTERNAL_TYPE implementation, it probably won't do what you think.
            const INTERNAL_TYPE: gdal::vector::OGRwkbGeometryType::Type = OGRwkbGeometryType::wkbUnknown;

            fn get_envelope(&self) -> Extent {
                match self {
                    $( $enum::$variant(a) => a.get_envelope(), )*
                }
            }

            fn is_valid(&self) -> bool {
                match self {
                    $( $enum::$variant(a) => a.is_valid(), )*
                }
            }


        }

        impl TryFrom<GDALGeometry> for $enum {
            type Error = CommandError;

            fn try_from(value: GDALGeometry) -> Result<Self, Self::Error> {
                match value.geometry_type() {
                    $( $( OGRwkbGeometryType::$type => Ok(Self::$variant(<$struct>::try_from(value)?)),)? )*
                    unknown => Err(CommandError::UnsupportedGdalGeometryType(unknown))
                }
            }
        }

        impl From<$enum> for GDALGeometry {
            fn from(value: $enum) -> GDALGeometry {
                match value {
                    $( $enum::$variant(a) => a.into(), )*
                }
            }
        }

        $(
            impl_variants!($enum,$variant,$struct);
        )*

    };
}

impl_variant_geometry!(VariantGeometry {
    Point(Point) [wkbPoint],
    LineString(LineString) [wkbLineString],
    LinearRing(LinearRing) [wkbLinearRing],
    Polygon(Polygon) [wkbPolygon],
    MultiPolygon(MultiPolygon) [wkbMultiPolygon],
    VariantCollection(Collection<VariantGeometry>) [wkbGeometryCollection],
    PointCollection(Collection<Point>),
    LineStringCollection(Collection<LineString>),
    LinearRingCollection(Collection<LinearRing>),
    PolygonCollection(Collection<Polygon>),
    MultiPolygonCollection(Collection<MultiPolygon>),
    // I can't start to do Collection<Collection> as that would just lead to infinite types.
});

impl_variant_geometry!(VariantArealGeometry {
    Polygon(Polygon) [wkbPolygon],
    MultiPolygon(MultiPolygon) [wkbMultiPolygon],
});

impl VariantArealGeometry {

    #[allow(dead_code)] // not used, but I feel I should have it.
    pub(crate) fn area(&self) -> f64 {
        match self {
            Self::Polygon(inner) => inner.area(),
            Self::MultiPolygon(inner) => inner.area(),
        }
    }

    pub(crate) fn union(&self, rhs: &Self) -> Result<Self,CommandError> {
        match (self,rhs) {
            (Self::Polygon(lhs), Self::Polygon(rhs)) => lhs.union(rhs),
            (Self::MultiPolygon(lhs), Self::MultiPolygon(rhs)) => lhs.union(rhs),
            (Self::MultiPolygon(lhs), Self::Polygon(rhs)) => lhs.union(&rhs.clone().try_into()?),
            (Self::Polygon(lhs), Self::MultiPolygon(rhs)) => rhs.union(&lhs.clone().try_into()?),
        }
    }

    pub(crate) fn difference(&self, rhs: &Self) -> Result<Self,CommandError> {
        match (self,rhs) {
            (Self::Polygon(lhs), Self::Polygon(rhs)) => lhs.difference(rhs),
            (Self::MultiPolygon(lhs), Self::MultiPolygon(rhs)) => lhs.difference(rhs),
            (Self::MultiPolygon(lhs), Self::Polygon(rhs)) => lhs.difference(&rhs.clone().try_into()?),
            (Self::Polygon(lhs), Self::MultiPolygon(rhs)) => rhs.difference(&lhs.clone().try_into()?),
        }
    }

    pub(crate) fn intersection(&self, rhs: &Self) -> Result<Self,CommandError> {
        match (self,rhs) {
            (Self::Polygon(lhs), Self::Polygon(rhs)) => lhs.intersection(rhs),
            (Self::MultiPolygon(lhs), Self::MultiPolygon(rhs)) => lhs.intersection(rhs),
            (Self::MultiPolygon(lhs), Self::Polygon(rhs)) => lhs.intersection(&rhs.clone().try_into()?),
            (Self::Polygon(lhs), Self::MultiPolygon(rhs)) => rhs.intersection(&lhs.clone().try_into()?),
        }
    }


    pub(crate) fn buffer(&self, distance: f64, n_quad_segs: u32) -> Result<Self,CommandError> {
        match self {
            Self::Polygon(lhs) => lhs.buffer(distance,n_quad_segs),
            Self::MultiPolygon(lhs) => lhs.buffer(distance,n_quad_segs),
        }
    }

    pub(crate) fn simplify(&self, tolerance: f64) -> Result<Self,CommandError> {
        match self {
            Self::Polygon(lhs) => lhs.simplify(tolerance),
            Self::MultiPolygon(lhs) => lhs.simplify(tolerance),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Polygon(lhs) => lhs.is_empty(),
            Self::MultiPolygon(lhs) => lhs.is_empty(),
        }
    }


}


pub(crate) enum VariantArealGeometryIter {
    Polygon(Option<Polygon>),
    MultiPolygon(MultiPolygonIter)
}

impl Iterator for VariantArealGeometryIter {

    type Item = Result<Polygon,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Polygon(polygon) => {
                polygon.take().map(Ok)
            },
            Self::MultiPolygon(multi) => multi.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Polygon(Some(_)) => (1,Some(1)),
            Self::Polygon(None) => (0,Some(0)),
            Self::MultiPolygon(multi) => multi.size_hint()
        }
    }
}

impl IntoIterator for VariantArealGeometry {
    type Item = Result<Polygon,CommandError>;

    type IntoIter = VariantArealGeometryIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Polygon(polygon) => Self::IntoIter::Polygon(Some(polygon)),
            Self::MultiPolygon(multi) => Self::IntoIter::MultiPolygon(multi.into_iter()),
        }
    }
}

impl TryFrom<VariantArealGeometry> for MultiPolygon {

    type Error = CommandError;

    fn try_from(value: VariantArealGeometry) -> Result<Self,CommandError> {
        match value {
            VariantArealGeometry::Polygon(polygon) => polygon.try_into(),
            VariantArealGeometry::MultiPolygon(multi) => Ok(multi),
        }
    }
}

impl TryFrom<VariantArealGeometry> for Polygon {

    type Error = CommandError;

    fn try_from(value: VariantArealGeometry) -> Result<Self,CommandError> {
        match value {
            VariantArealGeometry::Polygon(polygon) => Ok(polygon),
            VariantArealGeometry::MultiPolygon(multi) => multi.try_into(),
        }
    }
}

#[derive(Clone)]
/// This can be used in generics that require a geometry to represent a lack of geometry. It is up to that structure to prevent creating the geometry object.
pub(crate) struct NoGeometry;


impl GDALGeometryWrapper for NoGeometry {
    // This marks it for use in generic objects so they know not to manipulate the geometry on this one.
    const INTERNAL_TYPE: OGRwkbGeometryType::Type = OGRwkbGeometryType::wkbNone;

    fn get_envelope(&self) -> Extent {
        unreachable!("This program should never be getting extent for 'None' geometry.")
    }

    fn is_valid(&self) -> bool {
        false
    }

    
}

impl TryFrom<GDALGeometry>for NoGeometry {
    type Error = CommandError;

    fn try_from(_: GDALGeometry) -> Result<Self,Self::Error>{
        unreachable!("This program should never be creating a 'None' geometry.")
    }

}

impl From<NoGeometry> for GDALGeometry {
    fn from(_: NoGeometry) -> Self {
        unreachable!("This program should never be getting a geometry off of 'None' geometry.")
    }

}

