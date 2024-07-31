#[derive(Debug, PartialEq)]
pub enum JSArgType {
    Int32,
    Int64,
    Float,
    Double,
    Boolean,
}

impl core::fmt::Display for JSArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            JSArgType::Int32 => "Int32",
            JSArgType::Int64 => "Int64",
            JSArgType::Float => "Float",
            JSArgType::Double => "Double",
            JSArgType::Boolean => "Boolean",
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
        quaint::ValueType::Boolean(Some(_)) => Some(JSArgType::Boolean),
        _ => None,
    }
}
