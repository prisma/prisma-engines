pub trait IntoTypedJsonExtension {
    // Returns the type-name
    fn type_name(&self) -> String;
    /// Decorate all values with type-hints
    fn as_typed_json(self) -> serde_json::Value;
}

impl<'a> IntoTypedJsonExtension for quaint::Value<'a> {
    fn type_name(&self) -> String {
        if self.is_null() {
            return "null".to_owned();
        }

        let type_name = match self.inner {
            quaint::ValueType::Int32(_) => "int",
            quaint::ValueType::Int64(_) => "bigint",
            quaint::ValueType::Float(_) => "float",
            quaint::ValueType::Double(_) => "double",
            quaint::ValueType::Text(_) => "string",
            quaint::ValueType::Enum(_, _) => "enum",
            quaint::ValueType::Bytes(_) => "bytes",
            quaint::ValueType::Boolean(_) => "bool",
            quaint::ValueType::Char(_) => "char",
            quaint::ValueType::Numeric(_) => "decimal",
            quaint::ValueType::Json(_) => "json",
            quaint::ValueType::Xml(_) => "xml",
            quaint::ValueType::Uuid(_) => "uuid",
            quaint::ValueType::DateTime(_) => "datetime",
            quaint::ValueType::Date(_) => "date",
            quaint::ValueType::Time(_) => "time",
            quaint::ValueType::Array(_) | quaint::ValueType::EnumArray(_, _) => "array",
        };

        type_name.to_owned()
    }

    fn as_typed_json(self) -> serde_json::Value {
        let type_name = self.type_name();

        let json_value = match self.inner {
            quaint::ValueType::Array(Some(values)) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.as_typed_json()).collect())
            }
            quaint::ValueType::Int64(Some(value)) => serde_json::Value::String(value.to_string()),
            quaint::ValueType::Numeric(Some(decimal)) => serde_json::Value::String(decimal.normalized().to_string()),
            x => serde_json::Value::from(x),
        };

        serde_json::json!({ "prisma__type": type_name, "prisma__value": json_value })
    }
}
