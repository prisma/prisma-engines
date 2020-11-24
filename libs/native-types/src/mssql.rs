use serde::*;
use serde_json::Value;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum MsSqlTypeParameter {
    Number(u16),
    Max,
}

impl fmt::Display for MsSqlTypeParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(length) => write!(f, "{}", length),
            Self::Max => write!(f, "max"),
        }
    }
}

/// Representing a type in SQL Server database.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MsSqlType {
    /// Maps to `i8` in Rust.
    TinyInt,
    /// Maps to `i16` in Rust.
    SmallInt,
    /// Maps to `i32` in Rust.
    Int,
    /// Maps to `i64` in Rust.
    BigInt,
    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    Decimal(Option<(u32, u32)>),
    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    Numeric(Option<(u32, u32)>),
    /// 8-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    Money,
    /// 4-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    SmallMoney,
    /// One or zero. Used mostly for booleans.
    Bit,
    /// A floating point value. Has two sizes: numbers 1 to 24 represent `f32`,
    /// 25 to 53 represent `f64`.
    Float(Option<u32>),
    /// A synonym for `float(24)`/`f32`.
    Real,
    /// Defines a date.
    Date,
    /// Defines a time.
    Time,
    /// Defines date and time. A legacy type with a weird accuracy of 1/300th of
    /// a second. Every new project should use the `datetime2` type.
    DateTime,
    /// Defines date and time. Accurate until a 100th of a nanosecond.
    DateTime2,
    /// A `datetime2` with the time zone information.
    DateTimeOffset,
    /// A datetime between 1900-01-01 through 2079-06-06. Accurate to one
    /// minute. A legacy type, any new project should use the
    /// `time`/`date`/`datetime2` or `datetimeoffset` types instead.
    SmallDateTime,
    /// A fixed size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    Char(Option<u32>),
    /// A fixed size UTF-16 string.
    NChar(Option<u32>),
    /// A variable size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    VarChar(Option<MsSqlTypeParameter>),
    /// A heap-stored ASCII string. Deprecated in favour of `varchar(max)`.
    Text,
    /// A variable size string, supporting the full range of unicode character
    /// data. Stores in UTF-16.
    NVarChar(Option<MsSqlTypeParameter>),
    /// A heap-stored Unicode (UTF-16) string. Deprecated in favour of
    /// `nvarchar(max)`.
    NText,
    /// A fixed size binary blob.
    Binary(Option<u32>),
    /// A variable size binary blob.
    VarBinary(Option<MsSqlTypeParameter>),
    /// A heap-stored binary blob. Deprecated in favlour of `varbinary(max)`.
    Image,
    /// XML text.
    Xml,
    /// GUID, which is UUID but Microsoft invented them so they have their own
    /// term for it.
    UniqueIdentifier,
}

impl super::NativeType for MsSqlType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}
