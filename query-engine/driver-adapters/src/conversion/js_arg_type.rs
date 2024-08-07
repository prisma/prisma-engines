#[derive(Debug, PartialEq)]
pub enum JSArgType {
    /// 32-bit signed integer.
    Int32,
    /// 64-bit signed integer.
    Int64,
    /// 32-bit floating point.
    Float,
    /// 64-bit floating point.
    Double,
    /// String value.
    Text,
    /// Database enum value.
    Enum,
    /// Database enum array (PostgreSQL specific).
    EnumArray,
    /// Bytes value.
    Bytes,
    /// Boolean value.
    Boolean,
    /// A single character.
    Char,
    /// An array value (PostgreSQL).
    Array,
    /// A numeric value.
    Numeric,
    /// A JSON value.
    Json,
    /// A XML value.
    Xml,
    /// An UUID value.
    Uuid,
    /// A datetime value.
    DateTime,
    /// A date value.
    Date,
    /// A time value.
    Time,
}

impl core::fmt::Display for JSArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JSArgType::Int32 => "Int32",
            JSArgType::Int64 => "Int64",
            JSArgType::Float => "Float",
            JSArgType::Double => "Double",
            JSArgType::Text => "Text",
            JSArgType::Enum => "Enum",
            JSArgType::EnumArray => "EnumArray",
            JSArgType::Bytes => "Bytes",
            JSArgType::Boolean => "Boolean",
            JSArgType::Char => "Char",
            JSArgType::Array => "Array",
            JSArgType::Numeric => "Numeric",
            JSArgType::Json => "Json",
            JSArgType::Xml => "Xml",
            JSArgType::Uuid => "Uuid",
            JSArgType::DateTime => "DateTime",
            JSArgType::Date => "Date",
            JSArgType::Time => "Time",
        };

        write!(f, "{}", s)
    }
}

pub fn value_to_js_arg_type(value: &quaint::Value) -> Option<JSArgType> {
    match &value.typed {
        quaint::ValueType::Int32(Some(_)) => Some(JSArgType::Int32),
        quaint::ValueType::Int64(Some(_)) => Some(JSArgType::Int64),
        quaint::ValueType::Float(Some(_)) => Some(JSArgType::Float),
        quaint::ValueType::Double(Some(_)) => Some(JSArgType::Double),
        quaint::ValueType::Text(Some(_)) => Some(JSArgType::Text),
        quaint::ValueType::Enum(Some(_), _) => Some(JSArgType::Enum),
        quaint::ValueType::EnumArray(Some(_), _) => Some(JSArgType::EnumArray),
        quaint::ValueType::Bytes(Some(_)) => Some(JSArgType::Bytes),
        quaint::ValueType::Boolean(Some(_)) => Some(JSArgType::Boolean),
        quaint::ValueType::Char(Some(_)) => Some(JSArgType::Char),
        quaint::ValueType::Array(Some(_)) => Some(JSArgType::Array),
        quaint::ValueType::Numeric(Some(_)) => Some(JSArgType::Numeric),
        quaint::ValueType::Json(Some(_)) => Some(JSArgType::Json),
        quaint::ValueType::Xml(Some(_)) => Some(JSArgType::Xml),
        quaint::ValueType::Uuid(Some(_)) => Some(JSArgType::Uuid),
        quaint::ValueType::DateTime(Some(_)) => Some(JSArgType::DateTime),
        quaint::ValueType::Date(Some(_)) => Some(JSArgType::Date),
        quaint::ValueType::Time(Some(_)) => Some(JSArgType::Time),

        _ => None,
    }
}
