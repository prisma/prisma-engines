use std::fmt;

/// Arguments accepted by the Oracle-compatible `NUMBER` type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KingbaseOracleNumberArguments {
    Unspecified,
    Precision(u32),
    PrecisionAndScale(u32, u32),
}

impl crate::datamodel_connector::NativeTypeArguments for KingbaseOracleNumberArguments {
    const DESCRIPTION: &'static str = "zero, one, or two nonnegative integers";
    const OPTIONAL_ARGUMENTS_COUNT: usize = 2;
    const REQUIRED_ARGUMENTS_COUNT: usize = 0;

    fn from_parts(parts: &[String]) -> Option<Self> {
        match parts {
            [] => Some(Self::Unspecified),
            [precision] => precision.parse().ok().map(Self::Precision),
            [precision, scale] => precision
                .parse()
                .ok()
                .zip(scale.parse().ok())
                .map(|(precision, scale)| Self::PrecisionAndScale(precision, scale)),
            _ => None,
        }
    }

    fn to_parts(&self) -> Vec<String> {
        match self {
            Self::Unspecified => Vec::new(),
            Self::Precision(precision) => vec![precision.to_string()],
            Self::PrecisionAndScale(precision, scale) => vec![precision.to_string(), scale.to_string()],
        }
    }
}

impl fmt::Display for KingbaseOracleNumberArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => Ok(()),
            Self::Precision(precision) => fmt::Display::fmt(precision, f),
            Self::PrecisionAndScale(precision, scale) => write!(f, "{precision},{scale}"),
        }
    }
}

crate::native_type_definition! {
    /// Native types exposed by KingbaseES in Oracle compatibility mode.
    KingbaseOracleType;
    Number(KingbaseOracleNumberArguments) -> Int | BigInt | Float | Decimal,
    TinyInt -> Int,
    Float(Option<u32>) -> Float,
    BinaryFloat -> Float,
    BinaryDouble -> Float,
    Char(Option<u32>) -> String,
    VarChar2(Option<u32>) -> String,
    NChar(Option<u32>) -> String,
    NVarChar2(Option<u32>) -> String,
    Clob -> String,
    NClob -> String,
    Blob -> Bytes,
    Date -> DateTime,
    Timestamp(Option<u32>) -> DateTime,
    TimestampTz(Option<u32>) -> DateTime,
    TimestampLocalTz(Option<u32>) -> DateTime,
    Boolean -> Boolean,
    Json -> Json,
    Uuid -> String,
    Xml -> String,
}
