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

        let type_name = match self {
            quaint::Value::Int32(_) => "int",
            quaint::Value::Int64(_) => "bigint",
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
            quaint::Value::DateTime(_) => "datetime",
            quaint::Value::Date(_) => "date",
            quaint::Value::Time(_) => "time",
            quaint::Value::Array(_) => "array",
            quaint::Value::Geometry(_) => "geometry",
            quaint::Value::Geography(_) => "geography",
        };

        type_name.to_owned()
    }

    fn as_typed_json(self) -> serde_json::Value {
        let type_name = self.type_name();

        let json_value = match self {
            quaint::Value::Array(Some(values)) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.as_typed_json()).collect())
            }
            quaint::Value::Int64(Some(value)) => serde_json::Value::String(value.to_string()),
            quaint::Value::Numeric(Some(decimal)) => serde_json::Value::String(decimal.normalized().to_string()),
            x => serde_json::Value::from(x),
        };

        serde_json::json!({ "prisma__type": type_name, "prisma__value": json_value })
    }
}
