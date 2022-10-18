use std::fmt;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum MsSqlTypeParameter {
    Number(u16),
    Max,
}

impl psl_core::datamodel_connector::NativeTypeArguments for MsSqlTypeParameter {
    const DESCRIPTION: &'static str = "an integer or `Max`";
    const OPTIONAL_ARGUMENTS_COUNT: usize = 0;
    const REQUIRED_ARGUMENTS_COUNT: usize = 1;

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [p] if p.eq_ignore_ascii_case("max") => Some(MsSqlTypeParameter::Max),
            [p] => p.parse().ok().map(MsSqlTypeParameter::Number),
            _ => None,
        }
    }

    fn to_parts(&self) -> Vec<String> {
        vec![self.to_string()]
    }
}

impl fmt::Display for MsSqlTypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(length) => fmt::Display::fmt(length, f),
            Self::Max => f.write_str("Max"),
        }
    }
}

crate::native_type_definition! {
    /// Representing a type in SQL Server database.
    MsSqlType;
    /// Maps to `i8` in Rust.
    TinyInt -> Int,
    /// Maps to `i16` in Rust.
    SmallInt -> Int,
    /// Maps to `i32` in Rust.
    Int -> Int,
    /// Maps to `i64` in Rust.
    BigInt -> BigInt,
    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    Decimal(Option<(u32, u32)>) -> Decimal,
    /// 8-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    Money -> Float,
    /// 4-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    SmallMoney -> Float,
    /// One or zero. Used mostly for booleans.
    Bit -> Boolean | Int,
    /// A floating point value. Has two sizes: numbers 1 to 24 represent `f32`,
    /// 25 to 53 represent `f64`.
    Float(Option<u32>) -> Float,
    /// A synonym for `float(24)`/`f32`.
    Real -> Float,
    /// Defines a date.
    Date -> DateTime,
    /// Defines a time.
    Time -> DateTime,
    /// Defines date and time. A legacy type with a weird accuracy of 1/300th of
    /// a second. Every new project should use the `datetime2` type.
    DateTime -> DateTime,
    /// Defines date and time. Accurate until a 100th of a nanosecond.
    DateTime2 -> DateTime,
    /// A `datetime2` with the time zone information.
    DateTimeOffset -> DateTime,
    /// A datetime between 1900-01-01 through 2079-06-06. Accurate to one
    /// minute. A legacy type, any new project should use the
    /// `time`/`date`/`datetime2` or `datetimeoffset` types instead.
    SmallDateTime -> DateTime,
    /// A fixed size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    Char(Option<u32>) -> String,
    /// A fixed size UTF-16 string.
    NChar(Option<u32>) -> String,
    /// A variable size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    VarChar(Option<MsSqlTypeParameter>) -> String,
    /// A heap-stored ASCII string. Deprecated in favour of `varchar(max)`.
    Text -> String,
    /// A variable size string, supporting the full range of unicode character
    /// data. Stores in UTF-16.
    NVarChar(Option<MsSqlTypeParameter>) -> String,
    /// A heap-stored Unicode (UTF-16) string. Deprecated in favour of
    /// `nvarchar(max)`.
    NText -> String,
    /// A fixed size binary blob.
    Binary(Option<u32>) -> Bytes,
    /// A variable size binary blob.
    VarBinary(Option<MsSqlTypeParameter>) -> Bytes,
    /// A heap-stored binary blob. Deprecated in favlour of `varbinary(max)`.
    Image -> Bytes,
    /// XML text.
    Xml -> String,
    /// GUID, which is UUID but Microsoft invented them so they have their own
    /// term for it.
    UniqueIdentifier -> String,
}
