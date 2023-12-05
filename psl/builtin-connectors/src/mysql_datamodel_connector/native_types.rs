crate::native_type_definition! {
    /// The MySQL native type enum.
    MySqlType;
    Int -> Int,
    UnsignedInt -> Int,
    SmallInt -> Int,
    UnsignedSmallInt -> Int,
    TinyInt -> Boolean | Int,
    UnsignedTinyInt -> Boolean | Int,
    MediumInt -> Int,
    UnsignedMediumInt -> Int,
    BigInt -> BigInt,
    Decimal(Option<(u32, u32)>) -> Decimal,
    UnsignedBigInt -> BigInt,
    Float -> Float,
    Double -> Float,
    Bit(u32) -> Boolean | Bytes,
    Char(u32) -> String,
    VarChar(u32) -> String,
    Binary(u32) -> Bytes,
    VarBinary(u32) -> Bytes,
    TinyBlob -> Bytes,
    Blob -> Bytes,
    MediumBlob -> Bytes,
    LongBlob -> Bytes,
    TinyText -> String,
    Text -> String,
    MediumText -> String,
    LongText -> String,
    Date -> DateTime,
    Time(Option<u32>) -> DateTime,
    DateTime(Option<u32>) -> DateTime,
    Timestamp(Option<u32>) -> DateTime,
    Year -> Int,
    Json -> Json,
    Geometry(Option<u32>) -> Geometry | GeoJson,
    Point(Option<u32>) -> Geometry | GeoJson,
    LineString(Option<u32>) -> Geometry | GeoJson,
    Polygon(Option<u32>) -> Geometry | GeoJson,
    MultiPoint(Option<u32>) -> Geometry | GeoJson,
    MultiLineString(Option<u32>) -> Geometry | GeoJson,
    MultiPolygon(Option<u32>) -> Geometry | GeoJson,
    GeometryCollection(Option<u32>) -> Geometry | GeoJson,
}

impl MySqlType {
    /// The user-defined precision for timestamp columns, where applicable.
    pub fn timestamp_precision(&self) -> Option<u32> {
        match self {
            MySqlType::Time(n) | MySqlType::DateTime(n) | MySqlType::Timestamp(n) => *n,
            _ => None,
        }
    }
}
