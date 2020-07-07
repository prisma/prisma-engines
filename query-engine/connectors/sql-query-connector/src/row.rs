use crate::error::SqlError;
use chrono::{DateTime, NaiveDate, Utc};
use connector_interface::{AggregationResult, Aggregator};
use datamodel::FieldArity;
use prisma_models::{PrismaValue, Record, TypeIdentifier};
use quaint::{
    ast::{Expression, Value},
    connector::ResultRow,
};
use rust_decimal::Decimal;
use std::{borrow::Borrow, io, str::FromStr};
use uuid::Uuid;

/// An allocated representation of a `Row` returned from the database.
#[derive(Debug, Clone, Default)]
pub struct SqlRow {
    pub values: Vec<PrismaValue>,
}

impl SqlRow {
    pub fn into_aggregation_results(self, aggregators: &[Aggregator]) -> Vec<AggregationResult> {
        let mut values = self.values;
        values.reverse();

        aggregators
            .iter()
            .flat_map(|aggregator| match aggregator {
                Aggregator::Count => vec![AggregationResult::Count(values.pop().unwrap())],

                Aggregator::Average(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Average(field.clone(), values.pop().unwrap()))
                    .collect(),

                Aggregator::Sum(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Sum(field.clone(), values.pop().unwrap()))
                    .collect(),

                Aggregator::Min(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Min(field.clone(), values.pop().unwrap()))
                    .collect(),

                Aggregator::Max(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Max(field.clone(), values.pop().unwrap()))
                    .collect(),
            })
            .collect()
    }
}

impl From<SqlRow> for Record {
    fn from(row: SqlRow) -> Record {
        Record::new(row.values)
    }
}

pub trait ToSqlRow {
    /// Conversion from a database specific row to an allocated `SqlRow`. To
    /// help deciding the right types, the provided `TypeIdentifier`s should map
    /// to the returned columns in the right order.
    fn to_sql_row<'b>(self, idents: &[(TypeIdentifier, FieldArity)]) -> crate::Result<SqlRow>;
}

impl ToSqlRow for ResultRow {
    fn to_sql_row<'b>(self, idents: &[(TypeIdentifier, FieldArity)]) -> crate::Result<SqlRow> {
        let mut row = SqlRow::default();
        let row_width = idents.len();

        row.values.reserve(row_width);

        for (i, p_value) in self.into_iter().enumerate().take(row_width) {
            let pv = match &idents[i] {
                (type_identifier, FieldArity::List) => match p_value {
                    value if value.is_null() => Ok(PrismaValue::List(Vec::new())),
                    Value::Array(None) => Ok(PrismaValue::List(Vec::new())),
                    Value::Array(Some(l)) => l
                        .into_iter()
                        .map(|p_value| row_value_to_prisma_value(p_value, &type_identifier))
                        .collect::<crate::Result<Vec<_>>>()
                        .map(|vec| PrismaValue::List(vec)),
                    _ => {
                        let error = io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("List field did not return an Array from database. Type identifier was {:?}. Value was {:?}.", &type_identifier, &p_value),
                        );
                        return Err(SqlError::ConversionError(error.into()));
                    }
                },
                (type_identifier, _) => row_value_to_prisma_value(p_value, &type_identifier),
            }?;

            row.values.push(pv);
        }

        Ok(row)
    }
}

pub fn row_value_to_prisma_value(p_value: Value, type_identifier: &TypeIdentifier) -> Result<PrismaValue, SqlError> {
    Ok(match type_identifier {
        TypeIdentifier::Boolean => match p_value {
            // Value::Array(vec) => PrismaValue::Boolean(b),
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Integer(Some(i)) => PrismaValue::Boolean(i != 0),
            Value::Boolean(Some(b)) => PrismaValue::Boolean(b),
            _ => {
                let error = io::Error::new(io::ErrorKind::InvalidData, "Bool value not stored as bool or int");
                return Err(SqlError::ConversionError(error.into()));
            }
        },
        TypeIdentifier::Enum(_) => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Enum(Some(cow)) => PrismaValue::Enum(cow.into_owned()),
            Value::Text(Some(cow)) => PrismaValue::Enum(cow.into_owned()),
            _ => {
                let error = io::Error::new(io::ErrorKind::InvalidData, "Enum value not stored as enum");
                return Err(SqlError::ConversionError(error.into()));
            }
        },

        TypeIdentifier::Json => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Text(Some(json)) => PrismaValue::Json(json.into()),
            Value::Json(Some(json)) => PrismaValue::Json(json.to_string()),
            _ => {
                let error = io::Error::new(io::ErrorKind::InvalidData, "Json value not stored as text or json");
                return Err(SqlError::ConversionError(error.into()));
            }
        },
        TypeIdentifier::UUID => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Text(Some(uuid)) => PrismaValue::Uuid(Uuid::parse_str(&uuid)?),
            Value::Uuid(Some(uuid)) => PrismaValue::Uuid(uuid),
            _ => {
                let error = io::Error::new(io::ErrorKind::InvalidData, "Uuid value not stored as text or uuid");
                return Err(SqlError::ConversionError(error.into()));
            }
        },
        TypeIdentifier::DateTime => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::DateTime(Some(dt)) => PrismaValue::DateTime(dt),
            Value::Integer(Some(ts)) => {
                let nsecs = ((ts % 1000) * 1_000_000) as u32;
                let secs = (ts / 1000) as i64;
                let naive = chrono::NaiveDateTime::from_timestamp(secs, nsecs);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

                PrismaValue::DateTime(datetime)
            }
            Value::Text(Some(dt_string)) => {
                let dt = DateTime::parse_from_rfc3339(dt_string.borrow())
                    .or_else(|_| DateTime::parse_from_rfc2822(dt_string.borrow()))
                    .map_err(|err| {
                        failure::format_err!("Could not parse stored DateTime string: {} ({})", dt_string, err)
                    })
                    .unwrap();

                PrismaValue::DateTime(dt.with_timezone(&Utc))
            }
            Value::Date(Some(d)) => {
                let dt = DateTime::<Utc>::from_utc(d.and_hms(0, 0, 0), Utc);
                PrismaValue::DateTime(dt)
            }
            Value::Time(Some(t)) => {
                let d = NaiveDate::from_ymd(1970, 1, 1);
                let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                PrismaValue::DateTime(dt)
            }
            _ => {
                let error = io::Error::new(
                    io::ErrorKind::InvalidData,
                    "DateTime value not stored as datetime, int or text",
                );
                return Err(SqlError::ConversionError(error.into()));
            }
        },
        TypeIdentifier::Float => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Real(Some(f)) => PrismaValue::Float(f),
            Value::Integer(Some(i)) => {
                // Decimal::from_f64 is buggy. Issue: https://github.com/paupino/rust-decimal/issues/228
                PrismaValue::Float(Decimal::from_str(&(i as f64).to_string()).expect("f64 was not a Decimal."))
            }
            Value::Text(_) | Value::Bytes(_) => PrismaValue::Float(
                p_value
                    .as_str()
                    .expect("text/bytes as str")
                    .parse()
                    .map_err(|err: rust_decimal::Error| SqlError::ColumnReadFailure(err.into()))?,
            ),
            _ => {
                let error = io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Float value not stored as float, int or text",
                );
                return Err(SqlError::ConversionError(error.into()));
            }
        },
        TypeIdentifier::Int => match p_value {
            Value::Integer(Some(i)) => PrismaValue::Int(i),
            Value::Bytes(Some(bytes)) => PrismaValue::Int(interpret_bytes_as_i64(&bytes)),
            Value::Text(Some(txt)) => PrismaValue::Int(
                i64::from_str(txt.trim_start_matches('\0')).map_err(|err| SqlError::ConversionError(err.into()))?,
            ),
            other => PrismaValue::from(other),
        },
        TypeIdentifier::String => match p_value {
            value if value.is_null() => PrismaValue::null(type_identifier.clone()),
            Value::Uuid(Some(uuid)) => PrismaValue::String(uuid.to_string()),
            Value::Json(Some(json_value)) => {
                PrismaValue::String(serde_json::to_string(&json_value).expect("JSON value to string"))
            }
            other => PrismaValue::from(other),
        },
    })
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SqlId {
    String(String),
    Int(usize),
    UUID(Uuid),
}

impl From<SqlId> for Expression<'static> {
    fn from(id: SqlId) -> Self {
        match id {
            SqlId::String(s) => s.into(),
            SqlId::Int(i) => (i as i64).into(),
            SqlId::UUID(u) => u.into(),
        }
    }
}

impl From<&SqlId> for Expression<'static> {
    fn from(id: &SqlId) -> Self {
        id.clone().into()
    }
}

// We assume the bytes are stored as a big endian signed integer, because that is what
// mysql does if you enter a numeric value for a bits column.
fn interpret_bytes_as_i64(bytes: &[u8]) -> i64 {
    match bytes.len() {
        8 => i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        len if len < 8 => {
            let sign_bit_mask: u8 = 0b10000000;
            // The first byte will only contain the sign bit.
            let most_significant_bit_byte = bytes[0] & sign_bit_mask;
            let padding = if most_significant_bit_byte == 0 { 0 } else { 0b11111111 };
            let mut i64_bytes = [padding; 8];

            for (target_byte, source_byte) in i64_bytes.iter_mut().rev().zip(bytes.iter().rev()) {
                *target_byte = *source_byte;
            }

            i64::from_be_bytes(i64_bytes)
        }
        0 => 0,
        _ => panic!("Attempted to interpret more than 8 bytes as an integer."),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn quaint_bytes_to_integer_conversion_works() {
        // Negative i64
        {
            let i: i64 = -123456789123;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i);
        }

        // Positive i64
        {
            let i: i64 = 123456789123;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i);
        }

        // Positive i32
        {
            let i: i32 = 123456789;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i as i64);
        }

        // Negative i32
        {
            let i: i32 = -123456789;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i as i64);
        }

        // Positive i16
        {
            let i: i16 = 12345;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i as i64);
        }

        // Negative i16
        {
            let i: i16 = -12345;
            let bytes = i.to_be_bytes();
            let roundtripped = interpret_bytes_as_i64(&bytes);
            assert_eq!(roundtripped, i as i64);
        }
    }
}
