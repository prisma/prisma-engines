/// `JSArgType` is a 1:1 mapping of [`quaint::ValueType`] that:
/// - only includes the type tag (e.g. `Int32`, `Text`, `Enum`, etc.)
/// - doesn't care for the optionality of the actual value (e.g., `quaint::Value::Int32(None)` -> `JSArgType::Int32`)
/// - is used to guide the JS side on how to serialize the query argument value before sending it to the JS driver.
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

pub fn value_to_js_arg_type(value: &quaint::Value) -> JSArgType {
    match &value.typed {
        quaint::ValueType::Int32(_) => JSArgType::Int32,
        quaint::ValueType::Int64(_) => JSArgType::Int64,
        quaint::ValueType::Float(_) => JSArgType::Float,
        quaint::ValueType::Double(_) => JSArgType::Double,
        quaint::ValueType::Text(_) => JSArgType::Text,
        quaint::ValueType::Enum(_, _) => JSArgType::Enum,
        quaint::ValueType::EnumArray(_, _) => JSArgType::EnumArray,
        quaint::ValueType::Bytes(_) => JSArgType::Bytes,
        quaint::ValueType::Boolean(_) => JSArgType::Boolean,
        quaint::ValueType::Char(_) => JSArgType::Char,
        quaint::ValueType::Array(_) => JSArgType::Array,
        quaint::ValueType::Numeric(_) => JSArgType::Numeric,
        quaint::ValueType::Json(_) => JSArgType::Json,
        quaint::ValueType::Xml(_) => JSArgType::Xml,
        quaint::ValueType::Uuid(_) => JSArgType::Uuid,
        quaint::ValueType::DateTime(_) => JSArgType::DateTime,
        quaint::ValueType::Date(_) => JSArgType::Date,
        quaint::ValueType::Time(_) => JSArgType::Time,
        quaint::ValueType::Var(_, _) => unreachable!(),
    }
}
