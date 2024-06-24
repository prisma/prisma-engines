use std::borrow::Borrow;

use super::ResultSet;
use crate::{Value, ValueType};
use ser::SerializeSeq as _;
use serde::*;

#[derive(Serialize)]
struct SerializedResultSet<'a> {
    columns: &'a Vec<String>,
    strings: &'a SerializedStrings,
    types: SerializedTypes<'a>,
    rows: &'a SerializedRows<'a>,
}

impl serde::Serialize for ResultSet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let strings = SerializedStrings::new(&self.rows);

        SerializedResultSet {
            columns: self.columns(),
            strings: &strings,
            types: SerializedTypes(&self.rows[0]),
            rows: &SerializedRows(&self.rows, &strings),
        }
        .serialize(serializer)
    }
}

#[derive(Debug)]
struct SerializedStrings {
    strings: String,
    string_spans: Vec<Vec<(usize, usize)>>,
}

impl serde::Serialize for SerializedStrings {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.strings.serialize(serializer)
    }
}

impl SerializedStrings {
    pub fn new(rows: &Vec<Vec<Value>>) -> Self {
        let (strings, indexes) = Self::compute_string_and_indexes(rows);

        Self {
            strings,
            string_spans: indexes,
        }
    }

    pub fn get_span(&self, string_idx: (usize, usize)) -> (usize, usize) {
        self.string_spans[string_idx.0][string_idx.1]
    }

    fn compute_string_and_indexes(rows: &Vec<Vec<Value>>) -> (String, Vec<Vec<(usize, usize)>>) {
        let string_len = rows.iter().map(Self::compute_string_length).sum::<usize>();
        let mut strings = String::with_capacity(string_len);

        let mut string_spans = Vec::with_capacity(rows.len());

        for row in rows {
            let mut row_indexes = Vec::with_capacity(row.len());

            for (value_idx, value) in row.iter().enumerate() {
                row_indexes.push((0 as usize, 0 as usize));

                match &value.typed {
                    ValueType::Array(Some(values)) => {
                        for value in values {
                            match &value.typed {
                                ValueType::Text(Some(value)) => {
                                    Self::add_string(&mut strings, &mut row_indexes, value_idx, value.borrow())
                                }
                                _ => (),
                            }
                        }
                    }
                    ValueType::Text(Some(value)) => {
                        Self::add_string(&mut strings, &mut row_indexes, value_idx, value.borrow());
                    }
                    _ => (),
                }
            }

            string_spans.push(row_indexes);
        }

        (strings, string_spans)
    }

    fn add_string(strings: &mut String, indexes: &mut Vec<(usize, usize)>, value_idx: usize, string: &str) {
        let start = strings.len();

        strings.push_str(string);
        indexes[value_idx] = (start, strings.len());
    }

    fn compute_string_length(row: &Vec<Value>) -> usize {
        let mut len = 0;

        for value in row {
            match &value.typed {
                ValueType::Text(Some(value)) => {
                    len += value.len();
                }
                ValueType::Array(Some(values)) => len += Self::compute_string_length(values),
                _ => (),
            }
        }

        len
    }
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

struct SerializedRows<'a>(&'a Vec<Vec<Value<'a>>>, &'a SerializedStrings);

impl<'a> serde::Serialize for SerializedRows<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for (row_idx, row) in self.0.iter().enumerate() {
            seq.serialize_element(&SerializedRow {
                row,
                strings: self.1,
                row_idx,
            })?;
        }

        seq.end()
    }
}

struct SerializedRow<'a> {
    row: &'a Vec<Value<'a>>,
    row_idx: usize,
    strings: &'a SerializedStrings,
}

impl<'a> serde::Serialize for SerializedRow<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.row.len()))?;

        for (value_idx, value) in self.row.iter().enumerate() {
            seq.serialize_element(&SerializedValue {
                value,
                strings: self.strings,
                string_idx: (self.row_idx, value_idx),
            })?;
        }

        seq.end()
    }
}

#[derive(Debug)]
struct SerializedValue<'a> {
    value: &'a Value<'a>,
    strings: &'a SerializedStrings,
    string_idx: (usize, usize),
}

impl<'a> Serialize for SerializedValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = &self.value;

        if val.is_null() {
            return serde_json::Value::Null.serialize(serializer);
        }

        match &val.typed {
            ValueType::Array(Some(values)) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;

                for (elem_idx, value) in values.iter().enumerate() {
                    seq.serialize_element(&SerializedValue {
                        value,
                        strings: self.strings,
                        string_idx: (self.string_idx.0, self.string_idx.1 + elem_idx),
                    })?;
                }

                seq.end()
            }
            ValueType::Int32(Some(value)) => value.serialize(serializer),
            ValueType::Int64(Some(value)) => value.to_string().serialize(serializer),
            ValueType::Numeric(Some(value)) => value.normalized().to_string().serialize(serializer),

            ValueType::Float(Some(value)) => value.serialize(serializer),
            ValueType::Double(Some(value)) => value.serialize(serializer),
            ValueType::Text(Some(_)) => self.strings.get_span(self.string_idx).serialize(serializer),
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