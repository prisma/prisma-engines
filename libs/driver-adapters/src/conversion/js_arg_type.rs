#[derive(Debug, PartialEq)]
pub struct JSArgType {
    pub scalar_type: JSArgScalarType,
    pub db_type: Option<String>,
    pub arity: JSArgArity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JSArgScalarType {
    String,
    Int,
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum,
    Uuid,
    Json,
    DateTime,
    Bytes,
    Unknown,
}

impl From<JSArgScalarType> for &'static str {
    fn from(arg: JSArgScalarType) -> Self {
        match arg {
            JSArgScalarType::String => "string",
            JSArgScalarType::Int => "int",
            JSArgScalarType::BigInt => "bigint",
            JSArgScalarType::Float => "float",
            JSArgScalarType::Decimal => "decimal",
            JSArgScalarType::Boolean => "boolean",
            JSArgScalarType::Enum => "enum",
            JSArgScalarType::Uuid => "uuid",
            JSArgScalarType::Json => "json",
            JSArgScalarType::DateTime => "datetime",
            JSArgScalarType::Bytes => "bytes",
            JSArgScalarType::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JSArgArity {
    Scalar,
    List,
}

impl From<JSArgArity> for &'static str {
    fn from(arg: JSArgArity) -> Self {
        match arg {
            JSArgArity::Scalar => "scalar",
            JSArgArity::List => "list",
        }
    }
}

pub fn value_to_js_arg_type(value: &quaint::Value) -> JSArgType {
    JSArgType {
        scalar_type: value_to_js_arg_scalar_type(value),
        db_type: value.native_column_type_name().map(|nt| nt.to_string()),
        arity: if matches!(
            value.typed,
            quaint::ValueType::Array(_) | quaint::ValueType::EnumArray(_, _)
        ) {
            JSArgArity::List
        } else {
            JSArgArity::Scalar
        },
    }
}

fn value_to_js_arg_scalar_type(value: &quaint::Value) -> JSArgScalarType {
    match &value.typed {
        quaint::ValueType::Int32(_) => JSArgScalarType::Int,
        quaint::ValueType::Int64(_) => JSArgScalarType::BigInt,
        quaint::ValueType::Float(_) => JSArgScalarType::Float,
        quaint::ValueType::Double(_) => JSArgScalarType::Float,
        quaint::ValueType::Text(_) | quaint::ValueType::Char(_) | quaint::ValueType::Xml(_) => JSArgScalarType::String,
        quaint::ValueType::Enum(_, _) | quaint::ValueType::EnumArray(_, _) => JSArgScalarType::Enum,
        quaint::ValueType::Bytes(_) => JSArgScalarType::Bytes,
        quaint::ValueType::Boolean(_) => JSArgScalarType::Boolean,
        quaint::ValueType::Array(vals) => vals
            .as_deref()
            .unwrap_or_default()
            .first()
            .map_or(JSArgScalarType::Unknown, value_to_js_arg_scalar_type),
        quaint::ValueType::Numeric(_) => JSArgScalarType::Decimal,
        quaint::ValueType::Json(_) => JSArgScalarType::Json,
        quaint::ValueType::Uuid(_) => JSArgScalarType::Uuid,
        quaint::ValueType::DateTime(_) | quaint::ValueType::Date(_) | quaint::ValueType::Time(_) => {
            JSArgScalarType::DateTime
        }
        quaint::ValueType::Opaque(_) => unreachable!("Opaque values are not supposed to be converted to JSON"),
    }
}
