use super::TypeParameter;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, io, str::FromStr};

static HEAP_ALLOCATED: Lazy<Vec<MsSqlType>> = Lazy::new(|| {
    vec![
        MsSqlType::Text,
        MsSqlType::NText,
        MsSqlType::Image,
        MsSqlType::Xml,
        MsSqlType::VarBinary(Some(TypeParameter::Max)),
        MsSqlType::VarChar(Some(TypeParameter::Max)),
        MsSqlType::NVarChar(Some(TypeParameter::Max)),
    ]
});

/// Parsing of a stringified type with no parameters.
///
/// Examples:
///
/// ```ignore
/// bigint
/// ```
static TYPE_NO_PARAMS: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*(?P<kind>\w+)\s*$").unwrap());

/// Parsing of a stringified type with one type parameter.
///
/// Examples:
///
/// ```ignore
/// nvarchar(max)
/// float(24)
/// ```
static TYPE_SINGLE_PARAM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?P<kind>\w+)\s*(\(\s*(?P<p0>\w+)\s*\))$").unwrap());

/// Parsing of a stringified type with two type parameters.
///
/// Examples:
///
/// ```ignore
/// decimal(32,16)
/// ```
static TYPE_DOUBLE_PARAM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(?P<kind>\w+)\s*(\(\s*(?P<p0>\w+)\s*,\s*(?P<p1>\w+)\s*\))$").unwrap());

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
    Decimal(Option<(u8, u8)>),
    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    Numeric(Option<(u8, u8)>),
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
    Float(Option<u8>),
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
    Char(Option<u16>),
    /// A fixed size UTF-16 string.
    NChar(Option<u16>),
    /// A variable size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    VarChar(Option<TypeParameter>),
    /// A heap-stored ASCII string. Deprecated in favour of `varchar(max)`.
    Text,
    /// A variable size string, supporting the full range of unicode character
    /// data. Stores in UTF-16.
    NVarChar(Option<TypeParameter>),
    /// A heap-stored Unicode (UTF-16) string. Deprecated in favour of
    /// `nvarchar(max)`.
    NText,
    /// A fixed size binary blob.
    Binary(Option<u16>),
    /// A variable size binary blob.
    VarBinary(Option<TypeParameter>),
    /// A heap-stored binary blob. Deprecated in favlour of `varbinary(max)`.
    Image,
    /// XML text.
    Xml,
}

impl MsSqlType {
    /// A collection of types stored outside of the row to the heap, having
    /// certain properties such as not allowed in keys or normal indices.
    pub fn heap_allocated() -> &'static [MsSqlType] {
        &*HEAP_ALLOCATED
    }

    /// The type takes either 0 or the returned number of parameters.
    pub fn maximum_parameters(&self) -> usize {
        match self {
            MsSqlType::TinyInt => 0,
            MsSqlType::SmallInt => 0,
            MsSqlType::Int => 0,
            MsSqlType::BigInt => 0,
            MsSqlType::Decimal(_) => 2,
            MsSqlType::Numeric(_) => 2,
            MsSqlType::Money => 0,
            MsSqlType::SmallMoney => 0,
            MsSqlType::Bit => 0,
            MsSqlType::Float(_) => 1,
            MsSqlType::Real => 0,
            MsSqlType::Date => 0,
            MsSqlType::Time => 0,
            MsSqlType::DateTime => 0,
            MsSqlType::DateTime2 => 0,
            MsSqlType::DateTimeOffset => 0,
            MsSqlType::SmallDateTime => 0,
            MsSqlType::Char(_) => 1,
            MsSqlType::NChar(_) => 1,
            MsSqlType::VarChar(_) => 1,
            MsSqlType::Text => 0,
            MsSqlType::NVarChar(_) => 1,
            MsSqlType::NText => 0,
            MsSqlType::Binary(_) => 1,
            MsSqlType::VarBinary(_) => 1,
            MsSqlType::Image => 0,
            MsSqlType::Xml => 0,
        }
    }

    /// The name of the type without parameters.
    pub fn kind(&self) -> &'static str {
        match self {
            MsSqlType::TinyInt => "TinyInt",
            MsSqlType::SmallInt => "SmallInt",
            MsSqlType::Int => "Int",
            MsSqlType::BigInt => "BigInt",
            MsSqlType::Decimal(_) => "Decimal",
            MsSqlType::Numeric(_) => "Numeric",
            MsSqlType::Money => "Money",
            MsSqlType::SmallMoney => "SmallMoney",
            MsSqlType::Bit => "Bit",
            MsSqlType::Float(_) => "Float",
            MsSqlType::Real => "Real",
            MsSqlType::Date => "Date",
            MsSqlType::Time => "Time",
            MsSqlType::DateTime => "DateTime",
            MsSqlType::DateTime2 => "DateTime2",
            MsSqlType::DateTimeOffset => "DateTimeOffset",
            MsSqlType::SmallDateTime => "SmallDateTime",
            MsSqlType::Char(_) => "Char",
            MsSqlType::NChar(_) => "NChar",
            MsSqlType::VarChar(_) => "VarChar",
            MsSqlType::Text => "Text",
            MsSqlType::NVarChar(_) => "NVarChar",
            MsSqlType::NText => "NText",
            MsSqlType::Binary(_) => "Binary",
            MsSqlType::VarBinary(_) => "VarBinary",
            MsSqlType::Image => "Image",
            MsSqlType::Xml => "Xml",
        }
    }

    /// The type parameters, if any.
    pub fn parameters(self) -> Vec<TypeParameter> {
        match self {
            MsSqlType::Decimal(Some((p, s))) => vec![TypeParameter::from(p), TypeParameter::from(s)],
            MsSqlType::Numeric(Some((p, s))) => vec![TypeParameter::from(p), TypeParameter::from(s)],
            MsSqlType::Float(Some(l)) => vec![TypeParameter::from(l)],
            MsSqlType::Char(Some(l)) => vec![TypeParameter::from(l)],
            MsSqlType::NChar(Some(l)) => vec![TypeParameter::from(l)],
            MsSqlType::VarChar(Some(l)) => vec![l],
            MsSqlType::NVarChar(Some(l)) => vec![l],
            MsSqlType::VarBinary(Some(l)) => vec![l],
            MsSqlType::Binary(Some(l)) => vec![TypeParameter::from(l)],
            _ => vec![],
        }
    }
}

impl fmt::Display for MsSqlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())?;

        let parameters = self.parameters();
        let length = parameters.len();

        if parameters.len() > 0 {
            write!(f, "(")?;
            for (i, p) in parameters.into_iter().enumerate() {
                write!(f, "{}", p)?;

                if i < length - 1 {
                    write!(f, ",")?;
                }
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl FromStr for MsSqlType {
    type Err = crate::Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let captures = TYPE_NO_PARAMS
            .captures(s)
            .or_else(|| TYPE_SINGLE_PARAM.captures(s))
            .or_else(|| TYPE_DOUBLE_PARAM.captures(s));

        match captures {
            Some(captures) => {
                let kind = captures
                    .name("kind")
                    .map(|cap| cap.as_str())
                    .map(|kind| kind.trim().to_lowercase())
                    .ok_or_else(|| {
                        let ek = io::ErrorKind::InvalidInput;
                        io::Error::new(ek, format!("Could not parse `{}` as a MsSqlType.", s))
                    })?;

                let p0 = captures.name("p0").map(|cap| cap.as_str());
                let p1 = captures.name("p1").map(|cap| cap.as_str());

                match kind.as_str() {
                    "decimal" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::Decimal(None)),
                        (Some(p0), Some(p1)) => Ok(MsSqlType::Decimal(Some((p0.parse()?, p1.parse()?)))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `decimal`."))?
                        }
                    },
                    "numeric" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::Numeric(None)),
                        (Some(p0), Some(p1)) => Ok(MsSqlType::Numeric(Some((p0.parse()?, p1.parse()?)))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `numeric`."))?
                        }
                    },
                    "float" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::Float(None)),
                        (Some(p0), None) => Ok(MsSqlType::Float(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `float`."))?
                        }
                    },
                    "char" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::Char(None)),
                        (Some(p0), None) => Ok(MsSqlType::Char(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `char`."))?
                        }
                    },
                    "nchar" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::NChar(None)),
                        (Some(p0), None) => Ok(MsSqlType::NChar(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `nchar`."))?
                        }
                    },
                    "varchar" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::VarChar(None)),
                        (Some(p0), None) => Ok(MsSqlType::VarChar(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `varchar`."))?
                        }
                    },
                    "nvarchar" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::NVarChar(None)),
                        (Some(p0), None) => Ok(MsSqlType::NVarChar(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `nvarchar`."))?
                        }
                    },
                    "binary" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::Binary(None)),
                        (Some(p0), None) => Ok(MsSqlType::Binary(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `binary`."))?
                        }
                    },
                    "varbinary" => match (p0, p1) {
                        (None, None) => Ok(MsSqlType::VarBinary(None)),
                        (Some(p0), None) => Ok(MsSqlType::VarBinary(Some(p0.parse()?))),
                        _ => {
                            let kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(kind, "Invalid number of types for `varbinary`."))?
                        }
                    },
                    kind => {
                        if p0.is_some() || p1.is_some() {
                            let err_kind = io::ErrorKind::InvalidInput;
                            Err(io::Error::new(
                                err_kind,
                                format!("Invalid number of types for `{}`.", kind),
                            ))?
                        }

                        match kind {
                            "tinyint" => Ok(MsSqlType::TinyInt),
                            "smallint" => Ok(MsSqlType::SmallInt),
                            "int" => Ok(MsSqlType::Int),
                            "bigint" => Ok(MsSqlType::BigInt),
                            "money" => Ok(MsSqlType::Money),
                            "smallmoney" => Ok(MsSqlType::SmallMoney),
                            "bit" => Ok(MsSqlType::Bit),
                            "real" => Ok(MsSqlType::Real),
                            "date" => Ok(MsSqlType::Date),
                            "time" => Ok(MsSqlType::Time),
                            "datetime" => Ok(MsSqlType::DateTime),
                            "datetime2" => Ok(MsSqlType::DateTime2),
                            "datetimeoffset" => Ok(MsSqlType::DateTimeOffset),
                            "smalldatetime" => Ok(MsSqlType::SmallDateTime),
                            "text" => Ok(MsSqlType::Text),
                            "ntext" => Ok(MsSqlType::NText),
                            "image" => Ok(MsSqlType::Image),
                            "xml" => Ok(MsSqlType::Xml),
                            k => {
                                let kind = io::ErrorKind::InvalidInput;
                                Err(io::Error::new(kind, format!("Invalid SQL Server type: `{}`", k)))?
                            }
                        }
                    }
                }
            }
            None => {
                let ek = io::ErrorKind::InvalidInput;

                Err(io::Error::new(ek, format!("Could not parse `{}` as a MsSqlType", s)))?
            }
        }
    }
}

impl super::NativeType for MsSqlType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn parse_correct_type_no_params() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("tinyint", MsSqlType::TinyInt);
        types.insert("smallint", MsSqlType::SmallInt);
        types.insert("int", MsSqlType::Int);
        types.insert("bigint", MsSqlType::BigInt);
        types.insert("decimal", MsSqlType::Decimal(None));
        types.insert("numeric", MsSqlType::Numeric(None));
        types.insert("money", MsSqlType::Money);
        types.insert("smallmoney", MsSqlType::SmallMoney);
        types.insert("bit", MsSqlType::Bit);
        types.insert("float", MsSqlType::Float(None));
        types.insert("real", MsSqlType::Real);
        types.insert("date", MsSqlType::Date);
        types.insert("time", MsSqlType::Time);
        types.insert("datetime", MsSqlType::DateTime);
        types.insert("datetime2", MsSqlType::DateTime2);
        types.insert("datetimeoffset", MsSqlType::DateTimeOffset);
        types.insert("smalldatetime", MsSqlType::SmallDateTime);
        types.insert("char", MsSqlType::Char(None));
        types.insert("nchar", MsSqlType::NChar(None));
        types.insert("varchar", MsSqlType::VarChar(None));
        types.insert("text", MsSqlType::Text);
        types.insert("nvarchar", MsSqlType::NVarChar(None));
        types.insert("ntext", MsSqlType::NText);
        types.insert("binary", MsSqlType::Binary(None));
        types.insert("varbinary", MsSqlType::VarBinary(None));
        types.insert("image", MsSqlType::Image);
        types.insert("xml", MsSqlType::Xml);

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_having_parameters_for_a_type_that_has_none() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push("tinyint(1)");
        types.push("smallint(1)");
        types.push("int(1)");
        types.push("bigint(1)");
        types.push("money(1)");
        types.push("smallmoney(1)");
        types.push("bit(1)");
        types.push("real(1)");
        types.push("date(1)");
        types.push("time(1)");
        types.push("datetime(1)");
        types.push("datetime2(1)");
        types.push("datetimeoffset(1)");
        types.push("smalldatetime(1)");
        types.push("text(1)");
        types.push("ntext(1)");
        types.push("image(1)");
        types.push("xml(1)");

        for s in types.into_iter() {
            let res: crate::Result<MsSqlType> = s.parse();
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_incorrect_number_of_parameters() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push("decimal(1)");
        types.push("numeric(1, 2, 3)");
        types.push("float(1,2,3)");
        types.push("char(1,2)");
        types.push("nchar(1,2)");
        types.push("varchar(1,2)");
        types.push("nvarchar(1,2)");
        types.push("binary(1,2)");
        types.push("varbinary(1,2)");

        for s in types.into_iter() {
            let res: crate::Result<MsSqlType> = s.parse();
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_max_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("varchar(max)", MsSqlType::VarChar(Some(TypeParameter::Max)));
        types.insert("varbinary(max)", MsSqlType::VarBinary(Some(TypeParameter::Max)));
        types.insert("nvarchar(max)", MsSqlType::NVarChar(Some(TypeParameter::Max)));

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_single_int_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("float(24)", MsSqlType::Float(Some(24)));
        types.insert("char(123)", MsSqlType::Char(Some(123)));
        types.insert("nchar(456)", MsSqlType::NChar(Some(456)));
        types.insert("varchar(1000)", MsSqlType::VarChar(Some(TypeParameter::Number(1000))));
        types.insert("nvarchar(2000)", MsSqlType::NVarChar(Some(TypeParameter::Number(2000))));
        types.insert("binary(1024)", MsSqlType::Binary(Some(1024)));
        types.insert(
            "varbinary(2048)",
            MsSqlType::VarBinary(Some(TypeParameter::Number(2048))),
        );

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_double_int_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("decimal(32, 16)", MsSqlType::Decimal(Some((32, 16))));
        types.insert("numeric(16,8)", MsSqlType::Numeric(Some((16, 8))));

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_incorrectly_given_max_parameter() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push("decimal(max, max)");
        types.push("numeric(max, max)");
        types.push("float(max)");
        types.push("char(max)");
        types.push("nchar(max)");
        types.push("binary(max)");

        for s in types.into_iter() {
            let res: crate::Result<MsSqlType> = s.parse();
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_garbage() -> anyhow::Result<()> {
        let typ: MsSqlType = " nVarchaR(MaX)".parse()?;
        let expected = MsSqlType::NVarChar(Some(TypeParameter::Max));

        assert_eq!(expected, typ);

        Ok(())
    }
}
