use crate::{column_metadata::ColumnMetadata, error::SqlError, value::to_prisma_value};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use connector_interface::{coerce_null_to_zero_value, AggregationResult, AggregationSelection};
use datamodel::dml::FieldArity;
use prisma_models::{PrismaValue, Record, TypeIdentifier};
use quaint::{
    ast::{Expression, Value},
    connector::ResultRow,
};
use std::{io, str::FromStr};
use uuid::Uuid;

/// An allocated representation of a `Row` returned from the database.
#[derive(Debug, Clone, Default)]
pub struct SqlRow {
    pub values: Vec<PrismaValue>,
}

impl SqlRow {
    #[tracing::instrument(skip(self, selections))]
    pub fn into_aggregation_results(self, selections: &[AggregationSelection]) -> Vec<AggregationResult> {
        let mut values = self.values;
        values.reverse();

        selections
            .iter()
            .flat_map(|selection| match selection {
                AggregationSelection::Field(field) => {
                    vec![AggregationResult::Field(field.clone(), values.pop().unwrap())]
                }

                AggregationSelection::Count { all, fields } => {
                    let mut results: Vec<_> = fields
                        .iter()
                        .map(|field| {
                            AggregationResult::Count(
                                Some(field.clone()),
                                coerce_null_to_zero_value(values.pop().unwrap()),
                            )
                        })
                        .collect();

                    if *all {
                        results.push(AggregationResult::Count(
                            None,
                            coerce_null_to_zero_value(values.pop().unwrap()),
                        ))
                    }

                    results
                }

                AggregationSelection::Average(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Average(field.clone(), values.pop().unwrap()))
                    .collect(),

                AggregationSelection::Sum(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Sum(field.clone(), values.pop().unwrap()))
                    .collect(),

                AggregationSelection::Min(fields) => fields
                    .iter()
                    .map(|field| AggregationResult::Min(field.clone(), values.pop().unwrap()))
                    .collect(),

                AggregationSelection::Max(fields) => fields
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
    /// help deciding the right types, the provided `ColumnMetadata`s should map
    /// to the returned columns in the right order.
    fn to_sql_row(self, meta: &[ColumnMetadata<'_>]) -> crate::Result<SqlRow>;
}

impl ToSqlRow for ResultRow {
    #[tracing::instrument(skip(self, meta))]
    fn to_sql_row(self, meta: &[ColumnMetadata<'_>]) -> crate::Result<SqlRow> {
        let mut row = SqlRow::default();
        let row_width = meta.len();

        row.values.reserve(row_width);

        for (i, p_value) in self.into_iter().enumerate().take(row_width) {
            let pv = match (meta[i].identifier(), meta[i].arity()) {
                (type_identifier, FieldArity::List) => match p_value {
                    value if value.is_null() => Ok(PrismaValue::List(Vec::new())),
                    Value::Array(None) => Ok(PrismaValue::List(Vec::new())),
                    Value::Array(Some(l)) => l
                        .into_iter()
                        .map(|p_value| row_value_to_prisma_value(p_value, meta[i]))
                        .collect::<crate::Result<Vec<_>>>()
                        .map(PrismaValue::List),
                    _ => {
                        let error = io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("List field did not return an Array from database. Type identifier was {:?}. Value was {:?}.", &type_identifier, &p_value),
                        );
                        return Err(SqlError::ConversionError(error.into()));
                    }
                },
                _ => row_value_to_prisma_value(p_value, meta[i]),
            }?;

            row.values.push(pv);
        }

        Ok(row)
    }
}

#[tracing::instrument(skip(p_value, meta))]
pub fn row_value_to_prisma_value(p_value: Value, meta: ColumnMetadata<'_>) -> Result<PrismaValue, SqlError> {
    let create_error = |value: &Value| {
        let message = match meta.name() {
            Some(name) => {
                format!(
                    "Could not convert value {} of the field `{}` to type `{:?}`.",
                    value,
                    name,
                    meta.identifier()
                )
            }
            None => {
                format!("Could not convert value {} to type `{:?}`.", value, meta.identifier())
            }
        };

        let error = io::Error::new(io::ErrorKind::InvalidData, message);

        SqlError::ConversionError(error.into())
    };

    Ok(match meta.identifier() {
        TypeIdentifier::Boolean => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Int32(Some(i)) => PrismaValue::Boolean(i != 0),
            Value::Int64(Some(i)) => PrismaValue::Boolean(i != 0),
            Value::Boolean(Some(b)) => PrismaValue::Boolean(b),
            Value::Bytes(Some(bytes)) if bytes.as_ref() == [0u8] => PrismaValue::Boolean(false),
            Value::Bytes(Some(bytes)) if bytes.as_ref() == [1u8] => PrismaValue::Boolean(true),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Enum(_) => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Enum(Some(cow)) => PrismaValue::Enum(cow.into_owned()),
            Value::Text(Some(cow)) => PrismaValue::Enum(cow.into_owned()),
            _ => return Err(create_error(&p_value)),
        },

        TypeIdentifier::Json => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Text(Some(json)) => PrismaValue::Json(json.into()),
            Value::Json(Some(json)) => PrismaValue::Json(json.to_string()),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::UUID => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Text(Some(uuid)) => PrismaValue::Uuid(Uuid::parse_str(&uuid)?),
            Value::Uuid(Some(uuid)) => PrismaValue::Uuid(uuid),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::DateTime => match p_value {
            value if value.is_null() => PrismaValue::Null,
            value if value.is_integer() => {
                let ts = value.as_integer().unwrap();
                let nsecs = ((ts % 1000) * 1_000_000) as u32;
                let secs = (ts / 1000) as i64;
                let naive = chrono::NaiveDateTime::from_timestamp(secs, nsecs);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

                PrismaValue::DateTime(datetime.into())
            }
            Value::DateTime(Some(dt)) => PrismaValue::DateTime(dt.into()),
            Value::Text(Some(ref dt_string)) => {
                let dt = DateTime::parse_from_rfc3339(dt_string)
                    .or_else(|_| DateTime::parse_from_rfc2822(dt_string))
                    .map_err(|_| create_error(&p_value))?;

                PrismaValue::DateTime(dt.with_timezone(&Utc).into())
            }
            Value::Date(Some(d)) => {
                let dt = DateTime::<Utc>::from_utc(d.and_hms(0, 0, 0), Utc);
                PrismaValue::DateTime(dt.into())
            }
            Value::Time(Some(t)) => {
                let d = NaiveDate::from_ymd(1970, 1, 1);
                let dt = DateTime::<Utc>::from_utc(d.and_time(t), Utc);
                PrismaValue::DateTime(dt.into())
            }
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Float | TypeIdentifier::Decimal => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Numeric(Some(f)) => PrismaValue::Float(f.normalized()),
            Value::Double(Some(f)) => match f {
                f if f.is_nan() => return Err(create_error(&p_value)),
                f if f.is_infinite() => return Err(create_error(&p_value)),
                _ => PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized()),
            },
            Value::Float(Some(f)) => match f {
                f if f.is_nan() => return Err(create_error(&p_value)),
                f if f.is_infinite() => return Err(create_error(&p_value)),
                _ => PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized()),
            },
            Value::Int32(Some(i)) => match BigDecimal::from_i32(i) {
                Some(dec) => PrismaValue::Float(dec),
                None => return Err(create_error(&p_value)),
            },
            Value::Int64(Some(i)) => match BigDecimal::from_i64(i) {
                Some(dec) => PrismaValue::Float(dec),
                None => return Err(create_error(&p_value)),
            },
            Value::Text(_) | Value::Bytes(_) => {
                let dec: BigDecimal = p_value
                    .as_str()
                    .expect("text/bytes as str")
                    .parse()
                    .map_err(|_| create_error(&p_value))?;

                PrismaValue::Float(dec.normalized())
            }
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Int | TypeIdentifier::BigInt => match p_value {
            Value::Int32(Some(i)) => PrismaValue::Int(i as i64),
            Value::Int64(Some(i)) => PrismaValue::Int(i),
            Value::Bytes(Some(bytes)) => PrismaValue::Int(interpret_bytes_as_i64(&bytes)),
            Value::Text(Some(ref txt)) => {
                PrismaValue::Int(i64::from_str(txt.trim_start_matches('\0')).map_err(|_| create_error(&p_value))?)
            }
            other => to_prisma_value(other)?,
        },
        TypeIdentifier::String => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Uuid(Some(uuid)) => PrismaValue::String(uuid.to_string()),
            Value::Json(Some(ref json_value)) => {
                PrismaValue::String(serde_json::to_string(json_value).map_err(|_| create_error(&p_value))?)
            }
            other => to_prisma_value(other)?,
        },
        TypeIdentifier::Bytes => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Bytes(Some(bytes)) => PrismaValue::Bytes(bytes.into()),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Xml => match p_value {
            value if value.is_null() => PrismaValue::Null,
            Value::Xml(Some(xml)) => PrismaValue::Xml(xml.to_string()),
            Value::Text(Some(s)) => PrismaValue::Xml(s.into_owned()),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
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
