use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryParams {
    pub type_: GeometryType,
    pub srid: i32,
}

impl GeometryParams {
    pub const fn default() -> Self {
        Self {
            type_: GeometryType::Geometry,
            srid: 4326,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum GeometryType {
    Geometry = 0,
    Point = 1,
    LineString = 2,
    Polygon = 3,
    MultiPoint = 4,
    MultiLineString = 5,
    MultiPolygon = 6,
    GeometryCollection = 7,
    GeometryZ = 1000,
    PointZ = 1001,
    LineStringZ = 1002,
    PolygonZ = 1003,
    MultiPointZ = 1004,
    MultiLineStringZ = 1005,
    MultiPolygonZ = 1006,
    GeometryCollectionZ = 1007,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub enum GeometryDimension {
    #[default]
    XY,
    XYZ,
}

impl fmt::Display for GeometryDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XY => write!(f, "XY"),
            Self::XYZ => write!(f, "XYZ"),
        }
    }
}

impl GeometryType {
    pub fn as_2d(&self) -> Self {
        Self::try_from(*self as u32 % 1000).unwrap()
    }

    pub fn dimension(&self) -> &'static GeometryDimension {
        match *self as u32 / 1000 {
            0 => &GeometryDimension::XY,
            1 => &GeometryDimension::XYZ,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u32> for GeometryType {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, String> {
        match value {
            0 => Ok(Self::Geometry),
            1 => Ok(Self::Point),
            2 => Ok(Self::LineString),
            3 => Ok(Self::Polygon),
            4 => Ok(Self::MultiPoint),
            5 => Ok(Self::MultiLineString),
            6 => Ok(Self::MultiPolygon),
            7 => Ok(Self::GeometryCollection),
            1000 => Ok(Self::GeometryZ),
            1001 => Ok(Self::PointZ),
            1002 => Ok(Self::LineStringZ),
            1003 => Ok(Self::PolygonZ),
            1004 => Ok(Self::MultiPointZ),
            1005 => Ok(Self::MultiLineStringZ),
            1006 => Ok(Self::MultiPolygonZ),
            1007 => Ok(Self::GeometryCollectionZ),
            i => Err(format!("Unsupported geometry type code: {i}")),
        }
    }
}

impl FromStr for GeometryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "geometry" => Ok(GeometryType::Geometry),
            "geometryz" => Ok(GeometryType::GeometryZ),
            "point" => Ok(GeometryType::Point),
            "pointz" => Ok(GeometryType::PointZ),
            "linestring" => Ok(GeometryType::LineString),
            "linestringz" => Ok(GeometryType::LineStringZ),
            "polygon" => Ok(GeometryType::Polygon),
            "polygonz" => Ok(GeometryType::PolygonZ),
            "multipoint" => Ok(GeometryType::MultiPoint),
            "multipointz" => Ok(GeometryType::MultiPointZ),
            "multilinestring" => Ok(GeometryType::MultiLineString),
            "multilinestringz" => Ok(GeometryType::MultiLineStringZ),
            "multipolygon" => Ok(GeometryType::MultiPolygon),
            "multipolygonz" => Ok(GeometryType::MultiPolygonZ),
            "geometrycollection" => Ok(GeometryType::GeometryCollection),
            "geometrycollectionz" => Ok(GeometryType::GeometryCollectionZ),
            _ => Err(format!("Unsupported geometry type: {}.", s)),
        }
    }
}

impl fmt::Display for GeometryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryType::Geometry => write!(f, "Geometry"),
            GeometryType::GeometryZ => write!(f, "GeometryZ"),
            GeometryType::Point => write!(f, "Point"),
            GeometryType::PointZ => write!(f, "PointZ"),
            GeometryType::LineString => write!(f, "LineString"),
            GeometryType::LineStringZ => write!(f, "LineStringZ"),
            GeometryType::Polygon => write!(f, "Polygon"),
            GeometryType::PolygonZ => write!(f, "PolygonZ"),
            GeometryType::MultiPoint => write!(f, "MultiPoint"),
            GeometryType::MultiPointZ => write!(f, "MultiPointZ"),
            GeometryType::MultiLineString => write!(f, "MultiLineString"),
            GeometryType::MultiLineStringZ => write!(f, "MultiLineStringZ"),
            GeometryType::MultiPolygon => write!(f, "MultiPolygon"),
            GeometryType::MultiPolygonZ => write!(f, "MultiPolygonZ"),
            GeometryType::GeometryCollection => write!(f, "GeometryCollection"),
            GeometryType::GeometryCollectionZ => write!(f, "GeometryCollectionZ"),
        }
    }
}

impl crate::datamodel_connector::NativeTypeArguments for GeometryParams {
    const DESCRIPTION: &'static str = "a geometry type and an srid";
    const OPTIONAL_ARGUMENTS_COUNT: usize = 0;
    const REQUIRED_ARGUMENTS_COUNT: usize = 2;

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [geom, srid] => Some(Self {
                type_: GeometryType::from_str(geom).ok()?,
                srid: srid.parse().ok()?,
            }),
            _ => None,
        }
    }

    fn to_parts(&self) -> Vec<String> {
        vec![self.type_.to_string(), self.srid.to_string()]
    }
}
