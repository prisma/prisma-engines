pub trait IntoTypedJsonExtension {
    fn type_name(&self) -> String;
    fn as_typed_json(self) -> serde_json::Value;
}

impl<'a> IntoTypedJsonExtension for quaint::Value<'a> {
    fn type_name(&self) -> String {
        if self.is_null() {
            return "null".to_owned();
        }

        let type_name = match self {
            quaint::Value::Integer(Some(int)) => match i32::try_from(*int) {
                Ok(_) => "integer",
                Err(_) => "bigint",
            },
            quaint::Value::Integer(_) => unreachable!(),
            quaint::Value::Float(_) => "float",
            quaint::Value::Double(_) => "double",
            quaint::Value::Text(_) => "string",
            quaint::Value::Enum(_) => "enum",
            quaint::Value::Bytes(_) => "bytes",
            quaint::Value::Boolean(_) => "bool",
            quaint::Value::Char(_) => "char",
            quaint::Value::Numeric(_) => "decimal",
            quaint::Value::Json(_) => "json",
            quaint::Value::Xml(_) => "xml",
            quaint::Value::Uuid(_) => "uuid",
            quaint::Value::DateTime(_) | quaint::Value::Date(_) | quaint::Value::Time(_) => "date",
            quaint::Value::Array(_) => "array",
        };

        type_name.to_owned()
    }

    fn as_typed_json(self) -> serde_json::Value {
        let type_name = self.type_name();
        let json_value = match self {
            quaint::Value::Array(Some(values)) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.as_typed_json()).collect())
            }
            x => serde_json::Value::from(x),
        };

        serde_json::json!({ "prisma__type": type_name, "prisma__value": json_value })
    }
}
