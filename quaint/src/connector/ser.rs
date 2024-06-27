use super::ResultSet;
use crate::{Value, ValueType};
use serde::{ser::*, Serialize, Serializer};

pub struct SerializedResultSet(pub ResultSet);

#[derive(Debug, Serialize)]
struct InnerSerializedResultSet<'a> {
    columns: SerializedColumns<'a>,
    types: &'a SerializedTypes,
    rows: SerializedRows<'a>,
}

impl serde::Serialize for SerializedResultSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let this = &self.0;
        let types = SerializedTypes::new(&this.rows);

        InnerSerializedResultSet {
            columns: SerializedColumns(this),
            types: &types,
            rows: SerializedRows(&this.rows, &types),
        }
        .serialize(serializer)
    }
}

#[derive(Debug)]
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
                // `query_raw` does not return column names in `ResultSet` when a call to a stored procedure is done
                // See https://github.com/prisma/prisma/issues/6173
                seq.serialize_element(&format!("f{idx}"))?;
            }
        }

        seq.end()
    }
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
struct SerializedTypes(Vec<SerializedValueType>);

impl SerializedTypes {
    fn new<'a>(rows: &'a Vec<Vec<Value<'a>>>) -> Self {
        if rows.is_empty() {
            return Self(Vec::with_capacity(0));
        }

        let row_len = rows.first().unwrap().len();
        let mut types = vec![SerializedValueType::Unknown; row_len];
        let mut types_found = 0;
        let mut unknown_array_types = 0;

        'outer: for row in rows.iter() {
            for (idx, value) in row.iter().enumerate() {
                let current_type = types[idx];

                if matches!(
                    current_type,
                    SerializedValueType::Unknown | SerializedValueType::UnknownArray
                ) {
                    let inferred_type = SerializedValueType::infer_from(value);

                    if inferred_type != SerializedValueType::Unknown && inferred_type != current_type {
                        types[idx] = inferred_type;
                        types_found += 1;

                        if inferred_type == SerializedValueType::UnknownArray {
                            unknown_array_types += 1;
                        }

                        if current_type == SerializedValueType::UnknownArray {
                            unknown_array_types -= 1;
                        }
                    }
                }

                if types_found == row_len && unknown_array_types <= 0 {
                    break 'outer;
                }
            }
        }

        Self(types)
    }

    pub(crate) fn get(&self, idx: usize) -> SerializedValueType {
        *self.0.get(idx).unwrap()
    }
}

#[derive(Debug)]
struct SerializedRows<'a>(&'a Vec<Vec<Value<'a>>>, &'a SerializedTypes);

impl<'a> Serialize for SerializedRows<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for row in self.0.iter() {
            seq.serialize_element(&SerializedRow(row, self.1))?;
        }

        seq.end()
    }
}

struct SerializedRow<'a>(&'a Vec<Value<'a>>, &'a SerializedTypes);

impl Serialize for SerializedRow<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (row, types) = (self.0, self.1);
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for (value_idx, value) in row.iter().enumerate() {
            seq.serialize_element(&SerializedValue(value, types.get(value_idx)))?;
        }

        seq.end()
    }
}

struct SerializedAnyArrayValue<'a>(&'a Value<'a>);

impl<'a> Serialize for SerializedAnyArrayValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        let typ = SerializedValueType::infer_from(self.0);

        tuple.serialize_element(&typ)?;
        tuple.serialize_element(&SerializedValue(self.0, typ))?;

        tuple.end()
    }
}

struct SerializedValue<'a>(&'a Value<'a>, SerializedValueType);

impl<'a> Serialize for SerializedValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = &self.0;
        let typ = &self.1;

        match &val.typed {
            ValueType::Array(Some(values)) if typ.is_known_array() => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for value in values {
                    seq.serialize_element(&SerializedValue(value, SerializedValueType::infer_from(value)))?;
                }

                seq.end()
            }
            ValueType::Array(Some(values)) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for value in values {
                    seq.serialize_element(&SerializedAnyArrayValue(value))?;
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

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
enum SerializedValueType {
    #[serde(rename = "int")]
    Int32,
    #[serde(rename = "bigint")]
    Int64,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "double")]
    Double,
    #[serde(rename = "string")]
    Text,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "bool")]
    Boolean,
    #[serde(rename = "char")]
    Char,
    #[serde(rename = "decimal")]
    Numeric,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "xml")]
    Xml,
    #[serde(rename = "uuid")]
    Uuid,
    #[serde(rename = "datetime")]
    DateTime,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "time")]
    Time,

    #[serde(rename = "int-array")]
    Int32Array,
    #[serde(rename = "bigint-array")]
    Int64Array,
    #[serde(rename = "float-array")]
    FloatArray,
    #[serde(rename = "double-array")]
    DoubleArray,
    #[serde(rename = "string-array")]
    TextArray,
    #[serde(rename = "bytes-array")]
    BytesArray,
    #[serde(rename = "bool-array")]
    BooleanArray,
    #[serde(rename = "char-array")]
    CharArray,
    #[serde(rename = "decimal-array")]
    NumericArray,
    #[serde(rename = "json-array")]
    JsonArray,
    #[serde(rename = "xml-array")]
    XmlArray,
    #[serde(rename = "uuid-array")]
    UuidArray,
    #[serde(rename = "datetime-array")]
    DateTimeArray,
    #[serde(rename = "date-array")]
    DateArray,
    #[serde(rename = "time-array")]
    TimeArray,

    #[serde(rename = "unknown-array")]
    UnknownArray,

    #[serde(rename = "unknown")]
    Unknown,
}

impl SerializedValueType {
    fn infer_from(value: &Value) -> SerializedValueType {
        match &value.typed {
            ValueType::Int32(_) => SerializedValueType::Int32,
            ValueType::Int64(_) => SerializedValueType::Int64,
            ValueType::Float(_) => SerializedValueType::Float,
            ValueType::Double(_) => SerializedValueType::Double,
            ValueType::Text(_) => SerializedValueType::Text,
            ValueType::Enum(_, _) => SerializedValueType::Enum,
            ValueType::EnumArray(_, _) => SerializedValueType::TextArray,
            ValueType::Bytes(_) => SerializedValueType::Bytes,
            ValueType::Boolean(_) => SerializedValueType::Boolean,
            ValueType::Char(_) => SerializedValueType::Char,
            ValueType::Numeric(_) => SerializedValueType::Numeric,
            ValueType::Json(_) => SerializedValueType::Json,
            ValueType::Xml(_) => SerializedValueType::Xml,
            ValueType::Uuid(_) => SerializedValueType::Uuid,
            ValueType::DateTime(_) => SerializedValueType::DateTime,
            ValueType::Date(_) => SerializedValueType::Date,
            ValueType::Time(_) => SerializedValueType::Time,

            ValueType::Array(Some(values)) => {
                if values.is_empty() {
                    return SerializedValueType::UnknownArray;
                }

                match &values[0].typed {
                    ValueType::Int32(_) => SerializedValueType::Int32Array,
                    ValueType::Int64(_) => SerializedValueType::Int64Array,
                    ValueType::Float(_) => SerializedValueType::FloatArray,
                    ValueType::Double(_) => SerializedValueType::DoubleArray,
                    ValueType::Text(_) => SerializedValueType::TextArray,
                    ValueType::Bytes(_) => SerializedValueType::BytesArray,
                    ValueType::Boolean(_) => SerializedValueType::BooleanArray,
                    ValueType::Char(_) => SerializedValueType::CharArray,
                    ValueType::Numeric(_) => SerializedValueType::NumericArray,
                    ValueType::Json(_) => SerializedValueType::JsonArray,
                    ValueType::Xml(_) => SerializedValueType::XmlArray,
                    ValueType::Uuid(_) => SerializedValueType::UuidArray,
                    ValueType::DateTime(_) => SerializedValueType::DateTimeArray,
                    ValueType::Date(_) => SerializedValueType::DateArray,
                    ValueType::Time(_) => SerializedValueType::TimeArray,
                    ValueType::Enum(_, _) => SerializedValueType::TextArray,
                    ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                        unreachable!("Only PG supports scalar lists and tokio-postgres does not support 2d arrays")
                    }
                }
            }
            ValueType::Array(None) => SerializedValueType::UnknownArray,
        }
    }

    pub fn is_known_array(&self) -> bool {
        matches!(
            self,
            SerializedValueType::Int32Array
                | SerializedValueType::Int64Array
                | SerializedValueType::FloatArray
                | SerializedValueType::DoubleArray
                | SerializedValueType::TextArray
                | SerializedValueType::BytesArray
                | SerializedValueType::BooleanArray
                | SerializedValueType::CharArray
                | SerializedValueType::NumericArray
                | SerializedValueType::JsonArray
                | SerializedValueType::XmlArray
                | SerializedValueType::UuidArray
                | SerializedValueType::DateTimeArray
                | SerializedValueType::DateArray
                | SerializedValueType::TimeArray
        )
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
    use expect_test::expect;
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

        let expected = expect![[r#"
            {
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
                "int-array"
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
                    42,
                    42
                  ]
                ]
              ]
            }"#]];

        expected.assert_eq(&serialized);
    }

    #[test]
    fn serialize_empty_result_set() {
        let names = vec!["hello".to_string()];
        let result_set = ResultSet::new(names, vec![]);

        let serialized = serde_json::to_string_pretty(&SerializedResultSet(result_set)).unwrap();

        let expected = expect![[r#"
            {
              "columns": [
                "hello"
              ],
              "types": [],
              "rows": []
            }"#]];

        expected.assert_eq(&serialized)
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

        let expected = expect![[r#"
            {
              "columns": [
                "array"
              ],
              "types": [
                "int-array"
              ],
              "rows": [
                [
                  null
                ],
                [
                  [
                    42,
                    "42"
                  ]
                ],
                [
                  [
                    "heLlo",
                    null
                  ]
                ]
              ]
            }"#]];

        expected.assert_eq(&serialized);
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

        let expected = expect![[r#"
            {
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
            }"#]];

        expected.assert_eq(&serialized);
    }
}
