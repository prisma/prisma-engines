use super::ResultSet;
use crate::{Value, ValueType};
use ser::{SerializeSeq, SerializeTuple};
use serde::*;

pub struct SerializedResultSet(pub ResultSet);

#[derive(Serialize)]
struct InnerSerializedResultSet<'a> {
    columns: SerializedColumns<'a>,
    types: SerializedTypes<'a>,
    rows: &'a Vec<Vec<Value<'a>>>,
}

impl serde::Serialize for SerializedResultSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let this = &self.0;

        InnerSerializedResultSet {
            columns: SerializedColumns(this),
            types: SerializedTypes(&this.rows),
            rows: &this.rows,
        }
        .serialize(serializer)
    }
}

struct SerializedColumns<'a>(&'a ResultSet);

impl<'a> Serialize for SerializedColumns<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.rows.is_empty() {
            return self.0.columns().serialize(serializer);
        }

        let first_row = self.0.rows.first().unwrap();

        let mut seq = serializer.serialize_seq(Some(first_row.len()))?;

        for (idx, _) in first_row.iter().enumerate() {
            if let Some(column_name) = self.0.columns.get(idx) {
                seq.serialize_element(column_name)?;
            } else {
                seq.serialize_element(&format!("f{idx}"))?;
            }
        }

        seq.end()
    }
}

struct SerializedTypes<'a>(&'a Vec<Vec<Value<'a>>>);

impl<'a> Serialize for SerializedTypes<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_empty() {
            return serializer.serialize_seq(Some(0))?.end();
        }

        let first_row = &self.0[0];
        let mut seq = serializer.serialize_seq(Some(first_row.len()))?;

        for value in first_row {
            seq.serialize_element(get_value_type_name(value))?;
        }

        seq.end()
    }
}

struct SerializedArrayValue<'a>(&'a Value<'a>);

impl<'a> Serialize for SerializedArrayValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;

        tuple.serialize_element(get_value_type_name(self.0))?;
        tuple.serialize_element(self.0)?;

        tuple.end()
    }
}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = &self;

        match &val.typed {
            ValueType::Array(Some(values)) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for value in values {
                    seq.serialize_element(&SerializedArrayValue(value))?;
                }

                seq.end()
            }
            ValueType::Array(None) => serializer.serialize_none(),
            ValueType::Int32(value) => value.serialize(serializer),
            ValueType::Int64(value) => value.map(|val| val.to_string()).serialize(serializer),
            ValueType::Numeric(value) => value
                .as_ref()
                .map(|value| value.normalized().to_string())
                .serialize(serializer),
            ValueType::Float(value) => value.serialize(serializer),
            ValueType::Double(value) => value.serialize(serializer),
            ValueType::Text(value) => value.serialize(serializer),
            ValueType::Enum(value, _) => value.as_ref().map(|value| value.inner()).serialize(serializer),
            ValueType::EnumArray(Some(variants), _) => {
                let mut seq = serializer.serialize_seq(Some(variants.len()))?;

                for variant in variants {
                    seq.serialize_element(variant.inner())?;
                }

                seq.end()
            }
            ValueType::EnumArray(None, _) => serializer.serialize_none(),
            ValueType::Bytes(value) => value.as_ref().map(base64::encode).serialize(serializer),
            ValueType::Boolean(value) => value.serialize(serializer),
            ValueType::Char(value) => value.serialize(serializer),
            ValueType::Json(value) => value.serialize(serializer),
            ValueType::Xml(value) => value.serialize(serializer),
            ValueType::Uuid(value) => value.serialize(serializer),
            ValueType::DateTime(value) => value.map(|value| value.to_rfc3339()).serialize(serializer),
            ValueType::Date(value) => value.serialize(serializer),
            ValueType::Time(value) => value.serialize(serializer),
        }
    }
}

fn get_value_type_name<'a>(value: &'a Value<'_>) -> &'a str {
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
        crate::ValueType::EnumArray(_, _) => "string-array",
        crate::ValueType::Array(_) => "array",
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        connector::SerializedResultSet,
        prelude::{EnumName, EnumVariant, ResultSet},
        Value,
    };
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, Utc};
    use std::str::FromStr;

    #[test]
    fn serialize_result_set() {
        let names = vec![
            "int32".to_string(),
            "int64".to_string(),
            "float".to_string(),
            "double".to_string(),
            "text".to_string(),
            "enum".to_string(),
            "bytes".to_string(),
            "boolean".to_string(),
            "char".to_string(),
            "numeric".to_string(),
            "json".to_string(),
            "xml".to_string(),
            "uuid".to_string(),
            "datetime".to_string(),
            "date".to_string(),
            "time".to_string(),
            "intArray".to_string(),
        ];
        let rows = vec![vec![
            Value::int32(42),
            Value::int64(42),
            Value::float(42.523),
            Value::double(42.523),
            Value::text("heLlo"),
            Value::enum_variant_with_name("Red", EnumName::new("Color", Option::<String>::None)),
            Value::bytes(b"hello".to_vec()),
            Value::boolean(true),
            Value::character('c'),
            Value::numeric(BigDecimal::from_str("123456789.123456789").unwrap()),
            Value::json(serde_json::json!({"hello": "world"})),
            Value::xml("<hello>world</hello>"),
            Value::uuid(uuid::Uuid::from_str("550e8400-e29b-41d4-a716-446655440000").unwrap()),
            Value::datetime(
                chrono::DateTime::parse_from_rfc3339("2021-01-01T02:00:00Z")
                    .map(DateTime::<Utc>::from)
                    .unwrap(),
            ),
            Value::date(chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()),
            Value::time(chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap()),
            Value::array(vec![Value::int32(42), Value::int32(42)]),
        ]];
        let result_set = ResultSet::new(names, rows);

        let serialized = serde_json::to_string_pretty(&SerializedResultSet(result_set)).unwrap();

        let expected = indoc::indoc! {r#"{
      "columns": [
        "int32",
        "int64",
        "float",
        "double",
        "text",
        "enum",
        "bytes",
        "boolean",
        "char",
        "numeric",
        "json",
        "xml",
        "uuid",
        "datetime",
        "date",
        "time",
        "intArray"
      ],
      "types": [
        "int",
        "bigint",
        "float",
        "double",
        "string",
        "enum",
        "bytes",
        "bool",
        "char",
        "decimal",
        "json",
        "xml",
        "uuid",
        "datetime",
        "date",
        "time",
        "array"
      ],
      "rows": [
        [
          42,
          "42",
          42.523,
          42.523,
          "heLlo",
          "Red",
          "aGVsbG8=",
          true,
          "c",
          "123456789.123456789",
          {
            "hello": "world"
          },
          "<hello>world</hello>",
          "550e8400-e29b-41d4-a716-446655440000",
          "2021-01-01T02:00:00+00:00",
          "2021-01-01",
          "02:00:00",
          [
            [
              "int",
              42
            ],
            [
              "int",
              42
            ]
          ]
        ]
      ]
    }"#};

        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_empty_result_set() {
        let names = vec!["hello".to_string()];
        let result_set = ResultSet::new(names, vec![]);

        let serialized = serde_json::to_string_pretty(&SerializedResultSet(result_set)).unwrap();
        println!("{}", serialized);

        let expected = indoc::indoc! {r#"{
          "columns": [
            "hello"
          ],
          "types": [],
          "rows": []
        }"#};

        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_arrays() {
        let names = vec!["array".to_string()];
        let rows = vec![
            vec![Value::null_array()],
            vec![Value::array(vec![Value::int32(42), Value::int64(42)])],
            vec![Value::array(vec![Value::text("heLlo"), Value::null_text()])],
        ];
        let result_set = ResultSet::new(names, rows);

        let serialized = serde_json::to_string_pretty(&SerializedResultSet(result_set)).unwrap();
        println!("{}", serialized);

        let expected = indoc::indoc! {r#"{
        "columns": [
          "array"
        ],
        "types": [
          "array"
        ],
        "rows": [
          [
            null
          ],
          [
            [
              [
                "int",
                42
              ],
              [
                "bigint",
                "42"
              ]
            ]
          ],
          [
            [
              [
                "string",
                "heLlo"
              ],
              [
                "string",
                null
              ]
            ]
          ]
        ]
      }"#};

        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_enum_array() {
        let names = vec!["array".to_string()];
        let rows = vec![
            vec![Value::enum_array_with_name(
                vec![EnumVariant::new("A"), EnumVariant::new("B")],
                EnumName::new("Alphabet", Some("foo")),
            )],
            vec![Value::null_enum_array()],
        ];
        let result_set = ResultSet::new(names, rows);

        let serialized = serde_json::to_string_pretty(&SerializedResultSet(result_set)).unwrap();

        let expected = indoc::indoc! {r#"{
        "columns": [
          "array"
        ],
        "types": [
          "string-array"
        ],
        "rows": [
          [
            [
              "A",
              "B"
            ]
          ],
          [
            null
          ]
        ]
      }"#};

        assert_eq!(serialized, expected);
    }
}
