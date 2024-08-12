#[cfg(not(target_arch = "wasm32"))]
use super::TypeIdentifier;

use crate::{Value, ValueType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Int32,
    Int64,
    Float,
    Double,
    Text,
    Bytes,
    Boolean,
    Char,
    Numeric,
    Json,
    Xml,
    Uuid,
    DateTime,
    Date,
    Time,
    Enum,

    Int32Array,
    Int64Array,
    FloatArray,
    DoubleArray,
    TextArray,
    CharArray,
    BytesArray,
    BooleanArray,
    NumericArray,
    JsonArray,
    XmlArray,
    UuidArray,
    DateTimeArray,
    DateArray,
    TimeArray,

    Null,

    Unknown,
}

impl ColumnType {
    pub fn is_unknown(&self) -> bool {
        matches!(self, ColumnType::Unknown)
    }
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnType::Int32 => write!(f, "int"),
            ColumnType::Int64 => write!(f, "bigint"),
            ColumnType::Float => write!(f, "float"),
            ColumnType::Double => write!(f, "double"),
            ColumnType::Text => write!(f, "string"),
            ColumnType::Enum => write!(f, "enum"),
            ColumnType::Bytes => write!(f, "bytes"),
            ColumnType::Boolean => write!(f, "bool"),
            ColumnType::Char => write!(f, "char"),
            ColumnType::Numeric => write!(f, "decimal"),
            ColumnType::Json => write!(f, "json"),
            ColumnType::Xml => write!(f, "xml"),
            ColumnType::Uuid => write!(f, "uuid"),
            ColumnType::DateTime => write!(f, "datetime"),
            ColumnType::Date => write!(f, "date"),
            ColumnType::Time => write!(f, "time"),
            ColumnType::Int32Array => write!(f, "int-array"),
            ColumnType::Int64Array => write!(f, "bigint-array"),
            ColumnType::FloatArray => write!(f, "float-array"),
            ColumnType::DoubleArray => write!(f, "double-array"),
            ColumnType::TextArray => write!(f, "string-array"),
            ColumnType::BytesArray => write!(f, "bytes-array"),
            ColumnType::BooleanArray => write!(f, "bool-array"),
            ColumnType::CharArray => write!(f, "char-array"),
            ColumnType::NumericArray => write!(f, "decimal-array"),
            ColumnType::JsonArray => write!(f, "json-array"),
            ColumnType::XmlArray => write!(f, "xml-array"),
            ColumnType::UuidArray => write!(f, "uuid-array"),
            ColumnType::DateTimeArray => write!(f, "datetime-array"),
            ColumnType::DateArray => write!(f, "date-array"),
            ColumnType::TimeArray => write!(f, "time-array"),

            ColumnType::Null => write!(f, "null"),
            ColumnType::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&Value<'_>> for ColumnType {
    fn from(value: &Value<'_>) -> Self {
        Self::from(&value.typed)
    }
}

impl From<&ValueType<'_>> for ColumnType {
    fn from(value: &ValueType) -> Self {
        match value {
            ValueType::Int32(_) => ColumnType::Int32,
            ValueType::Int64(_) => ColumnType::Int64,
            ValueType::Float(_) => ColumnType::Float,
            ValueType::Double(_) => ColumnType::Double,
            ValueType::Text(_) => ColumnType::Text,
            ValueType::Enum(_, _) => ColumnType::Enum,
            ValueType::EnumArray(_, _) => ColumnType::TextArray,
            ValueType::Bytes(_) => ColumnType::Bytes,
            ValueType::Boolean(_) => ColumnType::Boolean,
            ValueType::Char(_) => ColumnType::Char,
            ValueType::Numeric(_) => ColumnType::Numeric,
            ValueType::Json(_) => ColumnType::Json,
            ValueType::Xml(_) => ColumnType::Xml,
            ValueType::Uuid(_) => ColumnType::Uuid,
            ValueType::DateTime(_) => ColumnType::DateTime,
            ValueType::Date(_) => ColumnType::Date,
            ValueType::Time(_) => ColumnType::Time,
            ValueType::Array(Some(vals)) if !vals.is_empty() => match &vals[0].typed {
                ValueType::Int32(_) => ColumnType::Int32Array,
                ValueType::Int64(_) => ColumnType::Int64Array,
                ValueType::Float(_) => ColumnType::FloatArray,
                ValueType::Double(_) => ColumnType::DoubleArray,
                ValueType::Text(_) => ColumnType::TextArray,
                ValueType::Enum(_, _) => ColumnType::TextArray,
                ValueType::Bytes(_) => ColumnType::BytesArray,
                ValueType::Boolean(_) => ColumnType::BooleanArray,
                ValueType::Char(_) => ColumnType::CharArray,
                ValueType::Numeric(_) => ColumnType::NumericArray,
                ValueType::Json(_) => ColumnType::JsonArray,
                ValueType::Xml(_) => ColumnType::TextArray,
                ValueType::Uuid(_) => ColumnType::UuidArray,
                ValueType::DateTime(_) => ColumnType::DateTimeArray,
                ValueType::Date(_) => ColumnType::DateArray,
                ValueType::Time(_) => ColumnType::TimeArray,
                ValueType::Array(_) => ColumnType::Unknown,
                ValueType::EnumArray(_, _) => ColumnType::Unknown,
            },
            ValueType::Array(_) => ColumnType::Unknown,
        }
    }
}

impl ColumnType {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_type_identifier<T>(value: T) -> Self
    where
        T: TypeIdentifier,
    {
        if value.is_bool() {
            ColumnType::Boolean
        } else if value.is_bytes() {
            ColumnType::Bytes
        } else if value.is_date() {
            ColumnType::Date
        } else if value.is_datetime() {
            ColumnType::DateTime
        } else if value.is_time() {
            ColumnType::Time
        } else if value.is_double() {
            ColumnType::Double
        } else if value.is_float() {
            ColumnType::Float
        } else if value.is_int32() {
            ColumnType::Int32
        } else if value.is_int64() {
            ColumnType::Int64
        } else if value.is_enum() {
            ColumnType::Enum
        } else if value.is_json() {
            ColumnType::Json
        } else if value.is_real() {
            ColumnType::Numeric
        } else if value.is_text() {
            ColumnType::Text
        } else {
            ColumnType::Unknown
        }
    }
}
