use super::ResultSet;
use crate::{Value, ValueType};
use ser::SerializeSeq as _;
use serde::*;

#[derive(Serialize)]
struct SerializedResultSet<'a> {
    columns: &'a Vec<String>,
    types: SerializedTypes<'a>,
    rows: &'a Vec<Vec<Value<'a>>>,
}

struct SerializedTypes<'a>(&'a Vec<Value<'a>>);

impl<'a> Serialize for SerializedTypes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for value in self.0 {
            seq.serialize_element(get_value_type_name(value))?;
        }

        seq.end()
    }
}

impl serde::Serialize for ResultSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializedResultSet {
            columns: self.columns(),
            types: SerializedTypes(&self.rows[0]),
            rows: &self.rows,
        }
        .serialize(serializer)
    }
}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = &self;

        if val.is_null() {
            return serde_json::Value::Null.serialize(serializer);
        }

        match &val.typed {
            ValueType::Array(Some(values)) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for value in values {
                    seq.serialize_element(value)?;
                }

                seq.end()
            }
            ValueType::Int32(Some(value)) => value.serialize(serializer),
            ValueType::Int64(Some(value)) => value.to_string().serialize(serializer),
            ValueType::Numeric(Some(value)) => value.normalized().to_string().serialize(serializer),

            ValueType::Float(Some(value)) => value.serialize(serializer),
            ValueType::Double(Some(value)) => value.serialize(serializer),
            ValueType::Text(Some(value)) => value.serialize(serializer),
            ValueType::Enum(value, _) => value.as_ref().map(|val| val.inner()).serialize(serializer),
            ValueType::EnumArray(_, _) => {
                todo!()
            }
            ValueType::Bytes(Some(value)) => base64::encode(value).serialize(serializer),
            ValueType::Boolean(Some(value)) => value.serialize(serializer),
            ValueType::Char(Some(value)) => value.serialize(serializer),
            ValueType::Json(Some(value)) => value.serialize(serializer),
            ValueType::Xml(Some(value)) => value.serialize(serializer),
            ValueType::Uuid(Some(value)) => value.serialize(serializer),
            ValueType::DateTime(Some(value)) => value.to_rfc3339().serialize(serializer),
            ValueType::Date(Some(value)) => value.serialize(serializer),
            ValueType::Time(Some(value)) => value.serialize(serializer),
            _ => unreachable!(),
        }
    }
}

fn get_value_type_name<'a>(value: &'a Value<'_>) -> &'a str {
    if value.is_null() {
        return "null";
    }

    match &value.typed {
        crate::ValueType::Int32(_) => "int",
        crate::ValueType::Int64(_) => "bigint",
        crate::ValueType::Float(_) => "float",
        crate::ValueType::Double(_) => "double",
        crate::ValueType::Text(_) => "string",
        crate::ValueType::Enum(_, _) => "enum",
        crate::ValueType::Bytes(_) => "bytes",
        crate::ValueType::Boolean(_) => "bool",
        crate::ValueType::Char(_) => "char",
        crate::ValueType::Numeric(_) => "decimal",
        crate::ValueType::Json(_) => "json",
        crate::ValueType::Xml(_) => "xml",
        crate::ValueType::Uuid(_) => "uuid",
        crate::ValueType::DateTime(_) => "datetime",
        crate::ValueType::Date(_) => "date",
        crate::ValueType::Time(_) => "time",
        crate::ValueType::Array(_) | crate::ValueType::EnumArray(_, _) => "array",
    }
}
