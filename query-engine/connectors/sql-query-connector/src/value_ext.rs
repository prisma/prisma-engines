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
            quaint::ValueInner::Int32(_) => "int",
            quaint::ValueInner::Int64(_) => "bigint",
            quaint::ValueInner::Float(_) => "float",
            quaint::ValueInner::Double(_) => "double",
            quaint::ValueInner::Text(_) => "string",
            quaint::ValueInner::Enum(_, _) => "enum",
            quaint::ValueInner::Bytes(_) => "bytes",
            quaint::ValueInner::Boolean(_) => "bool",
            quaint::ValueInner::Char(_) => "char",
            quaint::ValueInner::Numeric(_) => "decimal",
            quaint::ValueInner::Json(_) => "json",
            quaint::ValueInner::Xml(_) => "xml",
            quaint::ValueInner::Uuid(_) => "uuid",
            quaint::ValueInner::DateTime(_) => "datetime",
            quaint::ValueInner::Date(_) => "date",
            quaint::ValueInner::Time(_) => "time",
            quaint::ValueInner::Array(_) | quaint::ValueInner::EnumArray(_, _) => "array",
        };

        type_name.to_owned()
    }

    fn as_typed_json(self) -> serde_json::Value {
        let type_name = self.type_name();

        let json_value = match self.inner {
            quaint::ValueInner::Array(Some(values)) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.as_typed_json()).collect())
            }
            quaint::ValueInner::Int64(Some(value)) => serde_json::Value::String(value.to_string()),
            quaint::ValueInner::Numeric(Some(decimal)) => serde_json::Value::String(decimal.normalized().to_string()),
            x => serde_json::Value::from(x),
        };

        serde_json::json!({ "prisma__type": type_name, "prisma__value": json_value })
    }
}
