use std::borrow::Cow;

use parser_database::{GeometrySpec, GeometrySubtype, PostgisSpatialKind};

use crate::datamodel_connector::{NativeTypeArguments, NativeTypeParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum PostgresType {
    Known(KnownPostgresType),
    Unknown(String, Vec<String>),
    /// PostGIS spatial types (`geometry(Subtype, SRID)` / `geography(Subtype, SRID)`) that the
    /// generic `native_type_definition!` macro cannot express because their arguments mix an
    /// enum subtype with an optional SRID. The matching PSL scalars are the unit
    /// `ScalarType::Geometry` / `ScalarType::Geography` variants — subtype and SRID are carried
    /// here on the native attribute (same convention as `String @db.VarChar(300)`).
    Postgis(PostgisNativeType),
}

impl PostgresType {
    pub fn to_parts(&self) -> (&str, Cow<'_, [String]>) {
        match self {
            Self::Known(known) => known.to_parts(),
            Self::Unknown(name, args) => (name.as_str(), Cow::Borrowed(args)),
            Self::Postgis(postgis) => postgis.rendered_parts(),
        }
    }

    pub fn as_known(&self) -> Option<&KnownPostgresType> {
        match self {
            Self::Known(known) => Some(known),
            Self::Unknown(_, _) | Self::Postgis(_) => None,
        }
    }

    pub fn as_postgis(&self) -> Option<&PostgisNativeType> {
        match self {
            Self::Postgis(postgis) => Some(postgis),
            _ => None,
        }
    }
}

/// PostGIS native attribute: either `@db.Geometry(subtype, srid?)` for the planar `geometry`
/// type or `@db.Geography(subtype, srid?)` for the geodetic `geography` type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostgisNativeType {
    Geometry(GeometryNativeArgs),
    Geography(GeometryNativeArgs),
}

impl PostgisNativeType {
    pub fn args(&self) -> &GeometryNativeArgs {
        match self {
            Self::Geometry(args) | Self::Geography(args) => args,
        }
    }

    pub fn spatial(&self) -> PostgisSpatialKind {
        match self {
            Self::Geometry(_) => PostgisSpatialKind::Geometry,
            Self::Geography(_) => PostgisSpatialKind::Geography,
        }
    }

    pub fn to_geometry_spec(&self) -> GeometrySpec {
        let args = self.args();
        GeometrySpec {
            subtype: args.subtype,
            srid: args.srid,
            spatial: self.spatial(),
        }
    }

    fn rendered_parts(self) -> (&'static str, Cow<'static, [String]>) {
        let (name, args) = match self {
            Self::Geometry(args) => ("Geometry", args),
            Self::Geography(args) => ("Geography", args),
        };
        (name, Cow::Owned(args.to_parts()))
    }
}

/// Arguments accepted by `@db.Geometry` / `@db.Geography`: a required OGC subtype optionally
/// followed by a non-negative SRID literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryNativeArgs {
    pub subtype: GeometrySubtype,
    pub srid: Option<i32>,
}

impl GeometryNativeArgs {
    pub fn to_parts(&self) -> Vec<String> {
        let mut out = vec![self.subtype.as_str().to_owned()];
        if let Some(srid) = self.srid {
            out.push(srid.to_string());
        }
        out
    }
}

impl NativeTypeArguments for GeometryNativeArgs {
    const DESCRIPTION: &'static str =
        "an OGC geometry subtype (e.g. Point, LineString, Polygon) optionally followed by an SRID between 0 and 999999";
    const REQUIRED_ARGUMENTS_COUNT: usize = 1;
    const OPTIONAL_ARGUMENTS_COUNT: usize = 1;

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [subtype] => parse_subtype(subtype).map(|subtype| Self { subtype, srid: None }),
            [subtype, srid] => {
                let subtype = parse_subtype(subtype)?;
                let srid = srid.parse::<i32>().ok().filter(|v| (0..=999_999).contains(v))?;
                Some(Self {
                    subtype,
                    srid: Some(srid),
                })
            }
            _ => None,
        }
    }

    fn to_parts(&self) -> Vec<String> {
        GeometryNativeArgs::to_parts(self)
    }
}

fn parse_subtype(name: &str) -> Option<GeometrySubtype> {
    match name {
        "Point" => Some(GeometrySubtype::Point),
        "LineString" => Some(GeometrySubtype::LineString),
        "Polygon" => Some(GeometrySubtype::Polygon),
        "MultiPoint" => Some(GeometrySubtype::MultiPoint),
        "MultiLineString" => Some(GeometrySubtype::MultiLineString),
        "MultiPolygon" => Some(GeometrySubtype::MultiPolygon),
        "GeometryCollection" => Some(GeometrySubtype::GeometryCollection),
        "Geometry" => Some(GeometrySubtype::Geometry),
        _ => None,
    }
}

/// Parse a `@db.Geometry(...)` / `@db.Geography(...)` invocation. Returns `Some` only when the
/// name and argument shape match; the caller falls back to the generic
/// `KnownPostgresType::from_parts` path for every other native type.
pub(crate) fn try_parse_postgis<'a>(
    name: &'a str,
    arguments: &[String],
) -> Option<Result<PostgisNativeType, NativeTypeParseError<'a>>> {
    let ctor = match name {
        "Geometry" => PostgisNativeType::Geometry,
        "Geography" => PostgisNativeType::Geography,
        _ => return None,
    };

    let Some(args) = GeometryNativeArgs::from_parts(arguments) else {
        let rendered_args = format!("({})", arguments.join(", "));
        return Some(Err(NativeTypeParseError::InvalidArgs {
            expected: GeometryNativeArgs::DESCRIPTION,
            found: rendered_args,
        }));
    };

    Some(Ok(ctor(args)))
}

crate::native_type_definition! {
    KnownPostgresType;
    SmallInt -> Int,
    Integer -> Int,
    BigInt -> BigInt,
    Decimal(Option<(u32, u32)>) -> Decimal,
    Money -> Decimal,
    Inet -> String,
    Oid -> Int,
    Citext -> String,
    Real -> Float,
    DoublePrecision -> Float,
    VarChar(Option<u32>) -> String,
    Char(Option<u32>) -> String,
    Text -> String,
    ByteA -> Bytes,
    Timestamp(Option<u32>) -> DateTime,
    Timestamptz(Option<u32>) -> DateTime,
    Date -> DateTime,
    Time(Option<u32>) -> DateTime,
    Timetz(Option<u32>) -> DateTime,
    Boolean -> Boolean,
    Bit(Option<u32>) -> String,
    VarBit(Option<u32>) -> String,
    Uuid -> String,
    Xml -> String,
    Json -> Json,
    JsonB -> Json,
}
