pub trait IntoTypedJsonExtension {
    // Returns the type-name
    fn type_name(&self, backward_compatible: bool) -> String;
    /// Decorate all values with type-hints
    fn as_typed_json(self, backward_compatible: bool) -> serde_json::Value;
}

impl<'a> IntoTypedJsonExtension for quaint::Value<'a> {
    fn type_name(&self, backward_compatible: bool) -> String {
        if self.is_null() {
            return "null".to_owned();
        }

        let type_name = match self {
            quaint::Value::Int32(_) => "integer",
            quaint::Value::Int64(_) => match backward_compatible {
                // TODO: Remove backward_compatible checks for Prisma4
                true => "integer",
                false => "bigint",
            },
            quaint::Value::Float(_) => "float",
            quaint::Value::Double(_) => "double",
            quaint::Value::Text(_) => "string",
            quaint::Value::Enum(_) => "enum",
            quaint::Value::Bytes(_) => match backward_compatible {
                // TODO: Remove backward_compatible checks for Prisma4
                true => "string",
                false => "bytes",
            },
            quaint::Value::Boolean(_) => "bool",
            quaint::Value::Char(_) => "char",
            quaint::Value::Numeric(_) => match backward_compatible {
                true => "float",
                false => "decimal",
            },
            quaint::Value::Json(_) => "json",
            quaint::Value::Xml(_) => "xml",
            quaint::Value::Uuid(_) => "uuid",
            quaint::Value::DateTime(_) | quaint::Value::Date(_) | quaint::Value::Time(_) => match backward_compatible {
                // TODO: Remove backward_compatible checks for Prisma4
                true => "string",
                false => "date",
            },
            quaint::Value::Array(_) => "array",
        };

        type_name.to_owned()
    }

    fn as_typed_json(self, backward_compatible: bool) -> serde_json::Value {
        let type_name = self.type_name(backward_compatible);

        let json_value = match self {
            quaint::Value::Array(Some(values)) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|value| value.as_typed_json(backward_compatible))
                    .collect(),
            ),
            // TODO: Remove backward_compatible checks for Prisma4
            quaint::Value::Int64(Some(value)) if !backward_compatible => serde_json::Value::String(value.to_string()),
            // TODO: Remove backward_compatible checks for Prisma4
            quaint::Value::Numeric(Some(decimal)) if !backward_compatible => {
                serde_json::Value::String(decimal.normalized().to_string())
            }
            x => serde_json::Value::from(x),
        };

        serde_json::json!({ "prisma__type": type_name, "prisma__value": json_value })
    }
}
