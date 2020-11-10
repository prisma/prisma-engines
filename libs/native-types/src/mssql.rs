mod kind;
mod type_parameter;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{
    de::{Error, MapAccess, Unexpected, Visitor},
    ser::SerializeMap,
    Deserialize, Serialize, Serializer,
};
use serde_json::Value;
use std::{fmt, io, str::FromStr};

pub use kind::*;
pub use type_parameter::*;

static HEAP_ALLOCATED: Lazy<Vec<MsSqlType>> = Lazy::new(|| {
    vec![
        MsSqlType::text(),
        MsSqlType::ntext(),
        MsSqlType::image(),
        MsSqlType::xml(),
        MsSqlType::varbinary(Some(TypeParameter::Max)),
        MsSqlType::varchar(Some(TypeParameter::Max)),
        MsSqlType::nvarchar(Some(TypeParameter::Max)),
    ]
});

/// Parsing of a stringified type.
///
/// Examples:
///
/// ```ignore
/// bigint
/// ```
///
/// ```ignore
/// nvarchar(max)
/// ```
///
/// ```ignore
/// decimal(32, 16)
/// ```
static TYPE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?P<kind>\w+)(\((?P<params>.+)\))?$").unwrap());

/// Representing a type in SQL Server database.
#[derive(Debug, Clone, PartialEq)]
pub struct MsSqlType {
    kind: MsSqlKind,
    parameters: Vec<TypeParameter>,
}

impl MsSqlType {
    /// A collection of types stored outside of the row to the heap, having
    /// certain properties such as not allowed in keys or normal indices.
    pub fn heap_allocated() -> &'static [MsSqlType] {
        &*HEAP_ALLOCATED
    }

    /// Break the type into kind and parameters.
    pub fn into_parts(self) -> (MsSqlKind, Vec<TypeParameter>) {
        (self.kind, self.parameters)
    }

    /// The type kind without parameters.
    pub fn kind(&self) -> MsSqlKind {
        self.kind
    }

    /// The type parameters.
    pub fn parameters(&self) -> &[TypeParameter] {
        &self.parameters
    }

    /// Maps to `i8` in Rust.
    pub fn tinyint() -> Self {
        Self {
            kind: MsSqlKind::TinyInt,
            parameters: Vec::new(),
        }
    }

    /// Maps to `i16` in Rust.
    pub fn smallint() -> Self {
        Self {
            kind: MsSqlKind::SmallInt,
            parameters: Vec::new(),
        }
    }

    /// Maps to `i32` in Rust.
    pub fn int() -> Self {
        Self {
            kind: MsSqlKind::Int,
            parameters: Vec::new(),
        }
    }

    /// Maps to `i64` in Rust.
    pub fn bigint() -> Self {
        Self {
            kind: MsSqlKind::BigInt,
            parameters: Vec::new(),
        }
    }

    /// 8-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    pub fn money() -> Self {
        Self {
            kind: MsSqlKind::Money,
            parameters: Vec::new(),
        }
    }

    /// 4-byte numeric money value, accurate to a ten-thousandth of the monetary
    /// units.
    pub fn smallmoney() -> Self {
        Self {
            kind: MsSqlKind::SmallMoney,
            parameters: Vec::new(),
        }
    }

    /// One or zero. Used mostly for booleans.
    pub fn bit() -> Self {
        Self {
            kind: MsSqlKind::Bit,
            parameters: Vec::new(),
        }
    }

    /// A synonym for `float(24)`/`f32`.
    pub fn real() -> Self {
        Self {
            kind: MsSqlKind::Real,
            parameters: Vec::new(),
        }
    }

    /// Defines a date.
    pub fn date() -> Self {
        Self {
            kind: MsSqlKind::Date,
            parameters: Vec::new(),
        }
    }

    /// Defines a time.
    pub fn time() -> Self {
        Self {
            kind: MsSqlKind::Time,
            parameters: Vec::new(),
        }
    }

    /// Defines date and time. A legacy type with a weird accuracy of 1/300th of
    /// a second. Every new project should use the `datetime2` type.
    pub fn datetime() -> Self {
        Self {
            kind: MsSqlKind::DateTime,
            parameters: Vec::new(),
        }
    }

    /// Defines date and time. Accurate until a 100th of a nanosecond.
    pub fn datetime2() -> Self {
        Self {
            kind: MsSqlKind::DateTime2,
            parameters: Vec::new(),
        }
    }

    /// A `datetime2` with the time zone information.
    pub fn datetimeoffset() -> Self {
        Self {
            kind: MsSqlKind::DateTimeOffset,
            parameters: Vec::new(),
        }
    }

    /// A datetime between 1900-01-01 through 2079-06-06. Accurate to one
    /// minute. A legacy type, any new project should use the
    /// `time`/`date`/`datetime2` or `datetimeoffset` types instead.
    pub fn smalldatetime() -> Self {
        Self {
            kind: MsSqlKind::SmallDateTime,
            parameters: Vec::new(),
        }
    }

    /// A heap-stored ASCII string. Deprecated in favour of `varchar(max)`.
    pub fn text() -> Self {
        Self {
            kind: MsSqlKind::Text,
            parameters: Vec::new(),
        }
    }

    /// A heap-stored Unicode (UTF-16) string. Deprecated in favour of
    /// `nvarchar(max)`.
    pub fn ntext() -> Self {
        Self {
            kind: MsSqlKind::NText,
            parameters: Vec::new(),
        }
    }

    /// A heap-stored binary blob. Deprecated in favlour of `varbinary(max)`.
    pub fn image() -> Self {
        Self {
            kind: MsSqlKind::Image,
            parameters: Vec::new(),
        }
    }

    /// XML text.
    pub fn xml() -> Self {
        Self {
            kind: MsSqlKind::Xml,
            parameters: Vec::new(),
        }
    }

    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    pub fn decimal(params: Option<(u8, u8)>) -> Self {
        let parameters = params
            .map(|params| {
                vec![
                    TypeParameter::Number(params.0.into()),
                    TypeParameter::Number(params.1.into()),
                ]
            })
            .unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::Decimal,
            parameters,
        }
    }

    /// Numeric data types that have fixed precision and scale. Decimal and
    /// numeric are synonyms and can be used interchangeably.
    pub fn numeric(params: Option<(u8, u8)>) -> Self {
        let parameters = params
            .map(|params| {
                vec![
                    TypeParameter::Number(params.0.into()),
                    TypeParameter::Number(params.1.into()),
                ]
            })
            .unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::Numeric,
            parameters,
        }
    }

    /// A floating point value. Has two sizes: numbers 1 to 24 represent `f32`,
    /// 25 to 53 represent `f64`.
    pub fn float(size: Option<u8>) -> Self {
        let parameters = size
            .map(|size| vec![TypeParameter::Number(size.into())])
            .unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::Float,
            parameters,
        }
    }

    /// A fixed size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    pub fn r#char(length: Option<u64>) -> Self {
        let parameters = length.map(TypeParameter::Number).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::Char,
            parameters,
        }
    }

    /// A fixed size UTF-16 string.
    pub fn nchar(length: Option<u64>) -> Self {
        let parameters = length.map(TypeParameter::Number).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::NChar,
            parameters,
        }
    }

    /// A fixed size binary blob.
    pub fn binary(length: Option<u64>) -> Self {
        let parameters = length.map(TypeParameter::Number).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::Binary,
            parameters,
        }
    }

    /// A variable size string. Before SQL Server 2019 supported only ASCII
    /// characters, and from that version on supports also UTF-8 when using the
    /// right collation.
    pub fn varchar<T>(length: Option<T>) -> Self
    where
        T: Into<TypeParameter>,
    {
        let parameters = length.map(Into::into).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::VarChar,
            parameters,
        }
    }

    /// A variable size string, supporting the full range of unicode character
    /// data. Stores in UTF-16.
    pub fn nvarchar<T>(length: Option<T>) -> Self
    where
        T: Into<TypeParameter>,
    {
        let parameters = length.map(Into::into).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::NVarChar,
            parameters,
        }
    }

    /// A variable size binary blob.
    pub fn varbinary<T>(length: Option<T>) -> Self
    where
        T: Into<TypeParameter>,
    {
        let parameters = length.map(Into::into).map(|l| vec![l]).unwrap_or(Vec::new());

        Self {
            kind: MsSqlKind::VarBinary,
            parameters,
        }
    }
}

impl fmt::Display for MsSqlType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.parameters.len() {
            0 => write!(f, "{}", self.kind),
            _ => {
                let params: Vec<_> = self.parameters.iter().map(|s| format!("{}", s)).collect();
                write!(f, "{}({})", self.kind, params.join(","))
            }
        }
    }
}

impl FromStr for MsSqlType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match TYPE_REGEX.captures(s.trim()) {
            Some(captures) => {
                let kind = captures
                    .name("kind")
                    .and_then(|kind| kind.as_str().parse::<MsSqlKind>().ok())
                    .ok_or_else(|| {
                        let ek = io::ErrorKind::InvalidInput;
                        io::Error::new(ek, format!("Could not parse `{}` as a MsSqlType.", s))
                    })?;

                match captures.name("params") {
                    // The `max` variant.
                    Some(params) if TypeParameter::is_max(params.as_str()) => {
                        if kind.allows_max_variant() {
                            Ok(Self {
                                kind,
                                parameters: vec![TypeParameter::Max],
                            })
                        } else {
                            let ek = io::ErrorKind::InvalidInput;

                            Err(io::Error::new(
                                ek,
                                format!("The variant `max` is not allowed in kind `{}`", kind),
                            ))
                        }
                    }
                    // Number parameters given.
                    Some(params) => {
                        let splitted = params.as_str().split(",");
                        let mut parameters: Vec<TypeParameter> = Vec::new();

                        for n in splitted {
                            parameters.push(n.trim().parse()?);
                        }

                        match (parameters.len(), kind.maximum_parameters()) {
                            (n, m) if n == m => Ok(Self { kind, parameters }),
                            (n, m) => {
                                let ek = io::ErrorKind::InvalidInput;

                                Err(io::Error::new(
                                    ek,
                                    format!("Expected either 0 or {} parameters for kind `{}`, got {}.", m, kind, n),
                                ))
                            }
                        }
                    }
                    // No parameters.
                    None => Ok(Self {
                        kind,
                        parameters: Vec::new(),
                    }),
                }
            }
            None => {
                let ek = io::ErrorKind::InvalidInput;

                Err(io::Error::new(ek, format!("Could not parse `{}` as a MsSqlType", s)))
            }
        }
    }
}

impl Serialize for MsSqlType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.kind, &self.parameters)?;
        map.end()
    }
}

struct MsSqlTypeVisitor;

impl<'de> Visitor<'de> for MsSqlTypeVisitor {
    type Value = MsSqlType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "An object in the form of `\"TypeName\": [p1, p2]`")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        match map.next_entry()? {
            Some((k, v)) => {
                let kind: MsSqlKind = k;
                let parameters: Vec<TypeParameter> = v;
                let uses_max = parameters.iter().any(|v| *v == TypeParameter::Max);

                if uses_max && !kind.allows_max_variant() {
                    return Err(A::Error::invalid_value(Unexpected::Str("Max"), &self));
                }

                match (kind.maximum_parameters(), parameters.len()) {
                    (x, y) if x == y || y == 0 => Ok(Self::Value { kind, parameters }),
                    (_, y) => Err(A::Error::invalid_length(y, &self)),
                }
            }
            _ => Err(A::Error::invalid_value(Unexpected::StructVariant, &self)),
        }
    }
}

impl<'de> Deserialize<'de> for MsSqlType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(MsSqlTypeVisitor)
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
    use std::collections::BTreeMap;

    use super::*;
    use crate::NativeType;
    use serde_json::json;

    #[test]
    fn serde_no_params() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push((json!({"TinyInt": []}), MsSqlType::tinyint()));
        types.push((json!({"SmallInt": []}), MsSqlType::smallint()));
        types.push((json!({"Int": []}), MsSqlType::int()));
        types.push((json!({"BigInt": []}), MsSqlType::bigint()));
        types.push((json!({"Decimal": []}), MsSqlType::decimal(None)));
        types.push((json!({"Numeric": []}), MsSqlType::numeric(None)));
        types.push((json!({"Money": []}), MsSqlType::money()));
        types.push((json!({"SmallMoney": []}), MsSqlType::smallmoney()));
        types.push((json!({"Bit": []}), MsSqlType::bit()));
        types.push((json!({"Float": []}), MsSqlType::float(None)));
        types.push((json!({"Real": []}), MsSqlType::real()));
        types.push((json!({"Date": []}), MsSqlType::date()));
        types.push((json!({"Time": []}), MsSqlType::time()));
        types.push((json!({"DateTime": []}), MsSqlType::datetime()));
        types.push((json!({"DateTime2": []}), MsSqlType::datetime2()));
        types.push((json!({"DateTimeOffset": []}), MsSqlType::datetimeoffset()));
        types.push((json!({"SmallDateTime": []}), MsSqlType::smalldatetime()));
        types.push((json!({"Char": []}), MsSqlType::r#char(None)));
        types.push((json!({"NChar": []}), MsSqlType::nchar(None)));
        types.push((json!({"VarChar": []}), MsSqlType::varchar(Option::<u64>::None)));
        types.push((json!({"Text": []}), MsSqlType::text()));
        types.push((json!({"NVarChar": []}), MsSqlType::nvarchar(Option::<u64>::None)));
        types.push((json!({"NText": []}), MsSqlType::ntext()));
        types.push((json!({"Binary": []}), MsSqlType::binary(None)));
        types.push((json!({"VarBinary": []}), MsSqlType::varbinary(Option::<u64>::None)));
        types.push((json!({"Image": []}), MsSqlType::image()));
        types.push((json!({"Xml": []}), MsSqlType::xml()));

        for (expected, typ) in types.into_iter() {
            assert_eq!(expected, typ.to_json());
            assert_eq!(typ, serde_json::from_value(expected)?);
        }

        Ok(())
    }

    #[test]
    fn serde_with_int_params() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push((json!({"Decimal": [32, 16]}), MsSqlType::decimal(Some((32, 16)))));
        types.push((json!({"Numeric": [16, 8]}), MsSqlType::numeric(Some((16, 8)))));
        types.push((json!({"Float": [24]}), MsSqlType::float(Some(24))));
        types.push((json!({"Char": [123]}), MsSqlType::r#char(Some(123))));
        types.push((json!({"NChar": [456]}), MsSqlType::nchar(Some(456))));
        types.push((json!({"VarChar": [4000]}), MsSqlType::varchar(Some(4000u64))));
        types.push((json!({"NVarChar": [2000]}), MsSqlType::nvarchar(Some(2000u64))));
        types.push((json!({"Binary": [1024]}), MsSqlType::binary(Some(1024))));
        types.push((json!({"VarBinary": [2048]}), MsSqlType::varbinary(Some(2048u64))));

        for (expected, typ) in types.into_iter() {
            assert_eq!(expected, typ.to_json());
            assert_eq!(typ, serde_json::from_value(expected)?);
        }

        Ok(())
    }

    #[test]
    fn serde_with_max_param() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push((
            json!({"VarChar": ["Max"]}),
            MsSqlType::varchar(Some(TypeParameter::Max)),
        ));
        types.push((
            json!({"NVarChar": ["Max"]}),
            MsSqlType::nvarchar(Some(TypeParameter::Max)),
        ));
        types.push((
            json!({"VarBinary": ["Max"]}),
            MsSqlType::varbinary(Some(TypeParameter::Max)),
        ));

        for (expected, typ) in types.into_iter() {
            assert_eq!(expected, typ.to_json());
            assert_eq!(typ, serde_json::from_value(expected)?);
        }

        Ok(())
    }

    #[test]
    fn serde_having_parameters_for_a_type_that_has_none() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push(json!({"TinyInt": [1]}));
        types.push(json!({"SmallInt": [1, 2]}));
        types.push(json!({"Int": [1, 2]}));
        types.push(json!({"BigInt": [1, 2]}));
        types.push(json!({"Money": [1, 2]}));
        types.push(json!({"SmallMoney": [1, 2]}));
        types.push(json!({"Bit": [1]}));
        types.push(json!({"Real": [1]}));
        types.push(json!({"Date": [1]}));
        types.push(json!({"Time": [1]}));
        types.push(json!({"DateTime": [1]}));
        types.push(json!({"DateTime2": [1]}));
        types.push(json!({"DateTimeOffset": [1]}));
        types.push(json!({"SmallDateTime": [1]}));
        types.push(json!({"Text": [1]}));
        types.push(json!({"NText": [1]}));
        types.push(json!({"Image": [1]}));
        types.push(json!({"Xml": [1]}));

        for js in types.into_iter() {
            let res: Result<MsSqlType, _> = serde_json::from_value(js);
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn serde_incorrect_number_of_parameters() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push(json!({"Decimal": [1]}));
        types.push(json!({"Numeric": [1, 2, 3]}));
        types.push(json!({"Float": [1, 2, 3]}));
        types.push(json!({"Char": [1, 2, 3]}));
        types.push(json!({"NChar": [1, 2, 3]}));
        types.push(json!({"VarChar": [1, 2, 3]}));
        types.push(json!({"NVarChar": [1, 2, 3]}));
        types.push(json!({"Binary": [1, 2, 3]}));
        types.push(json!({"VarBinary": [1, 2, 3]}));

        for js in types.into_iter() {
            let res: Result<MsSqlType, _> = serde_json::from_value(js);
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn serde_incorrectly_given_max_parameter() -> anyhow::Result<()> {
        let mut types = Vec::new();

        types.push(json!({"Decimal": ["Max", "Max"]}));
        types.push(json!({"Numeric": ["Max", "Max"]}));
        types.push(json!({"Float": ["Max"]}));
        types.push(json!({"NChar": ["Max"]}));
        types.push(json!({"Char": ["Max"]}));
        types.push(json!({"Binary": ["Max"]}));

        for js in types.into_iter() {
            let res: Result<MsSqlType, _> = serde_json::from_value(js);
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_no_params() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("tinyint", MsSqlType::tinyint());
        types.insert("smallint", MsSqlType::smallint());
        types.insert("int", MsSqlType::int());
        types.insert("bigint", MsSqlType::bigint());
        types.insert("decimal", MsSqlType::decimal(None));
        types.insert("numeric", MsSqlType::numeric(None));
        types.insert("money", MsSqlType::money());
        types.insert("smallmoney", MsSqlType::smallmoney());
        types.insert("bit", MsSqlType::bit());
        types.insert("float", MsSqlType::float(None));
        types.insert("real", MsSqlType::real());
        types.insert("date", MsSqlType::date());
        types.insert("time", MsSqlType::time());
        types.insert("datetime", MsSqlType::datetime());
        types.insert("datetime2", MsSqlType::datetime2());
        types.insert("datetimeoffset", MsSqlType::datetimeoffset());
        types.insert("smalldatetime", MsSqlType::smalldatetime());
        types.insert("char", MsSqlType::r#char(None));
        types.insert("nchar", MsSqlType::nchar(None));
        types.insert("varchar", MsSqlType::varchar(Option::<u64>::None));
        types.insert("text", MsSqlType::text());
        types.insert("nvarchar", MsSqlType::nvarchar(Option::<u64>::None));
        types.insert("ntext", MsSqlType::ntext());
        types.insert("binary", MsSqlType::binary(None));
        types.insert("varbinary", MsSqlType::varbinary(Option::<u64>::None));
        types.insert("image", MsSqlType::image());
        types.insert("xml", MsSqlType::xml());

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
            let res: Result<MsSqlType, io::Error> = s.parse();
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
            let res: Result<MsSqlType, io::Error> = s.parse();
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_max_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("varchar(max)", MsSqlType::varchar(Some(TypeParameter::Max)));
        types.insert("nvarchar(max)", MsSqlType::nvarchar(Some(TypeParameter::Max)));

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_single_int_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("float(24)", MsSqlType::float(Some(24)));
        types.insert("char(123)", MsSqlType::r#char(Some(123)));
        types.insert("nchar(456)", MsSqlType::nchar(Some(456)));
        types.insert("varchar(1000)", MsSqlType::varchar(Some(1000u64)));
        types.insert("nvarchar(2000)", MsSqlType::nvarchar(Some(2000u64)));
        types.insert("binary(1024)", MsSqlType::binary(Some(1024)));
        types.insert("varbinary(2048)", MsSqlType::varbinary(Some(2048u64)));

        for (s, expected) in types.into_iter() {
            let typ: MsSqlType = s.parse()?;
            assert_eq!(expected, typ);
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_double_int_param() -> anyhow::Result<()> {
        let mut types = BTreeMap::new();

        types.insert("decimal(32, 16)", MsSqlType::decimal(Some((32, 16))));
        types.insert("numeric(16,8)", MsSqlType::numeric(Some((16, 8))));

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
            let res: Result<MsSqlType, io::Error> = s.parse();
            assert!(res.is_err());
        }

        Ok(())
    }

    #[test]
    fn parse_correct_type_with_garbage() -> anyhow::Result<()> {
        let typ: MsSqlType = " nVarchaR(MaX)".parse()?;
        let expected = MsSqlType::nvarchar(Some(TypeParameter::Max));

        assert_eq!(expected, typ);

        Ok(())
    }
}
