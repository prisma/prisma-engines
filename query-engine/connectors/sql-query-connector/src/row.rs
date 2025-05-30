use crate::{error::SqlError, value::to_prisma_value};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use connector_interface::{coerce_null_to_zero_value, AggregationResult};
use core::{f32, f64};
use quaint::{connector::ResultRow, Value, ValueType};
use query_structure::{AggregationSelection, ConversionFailure, FieldArity, PrismaValue, Record, TypeIdentifier};
use sql_query_builder::ColumnMetadata;
use std::{io, str::FromStr};
use uuid::Uuid;

/// An allocated representation of a `Row` returned from the database.
#[derive(Debug, Clone, Default)]
pub(crate) struct SqlRow {
    pub values: Vec<PrismaValue>,
}

impl SqlRow {
    pub fn into_aggregation_results(mut self, selections: &[AggregationSelection]) -> Vec<AggregationResult> {
        selections
            .iter()
            .flat_map(|selection| match selection {
                AggregationSelection::Field(field) => {
                    vec![AggregationResult::Field(
                        field.clone(),
                        self.values.drain(..1).next().unwrap(),
                    )]
                }

                AggregationSelection::Count { all, fields } => fields
                    .iter()
                    .map(|field| Some(field.clone()))
                    .chain(all.iter().map(|_| None))
                    .zip(
                        self.values
                            .drain(..fields.len() + if all.is_some() { 1 } else { 0 })
                            .map(coerce_null_to_zero_value),
                    )
                    .map(|(field, value)| AggregationResult::Count(field, value))
                    .collect(),

                AggregationSelection::Average(fields) => fields
                    .iter()
                    .zip(self.values.drain(..fields.len()))
                    .map(|(field, value)| AggregationResult::Average(field.clone(), value))
                    .collect(),

                AggregationSelection::Sum(fields) => fields
                    .iter()
                    .zip(self.values.drain(..fields.len()))
                    .map(|(field, value)| AggregationResult::Sum(field.clone(), value))
                    .collect(),

                AggregationSelection::Min(fields) => fields
                    .iter()
                    .zip(self.values.drain(..fields.len()))
                    .map(|(field, value)| AggregationResult::Min(field.clone(), value))
                    .collect(),

                AggregationSelection::Max(fields) => fields
                    .iter()
                    .zip(self.values.drain(..fields.len()))
                    .map(|(field, value)| AggregationResult::Max(field.clone(), value))
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

pub(crate) trait ToSqlRow {
    /// Conversion from a database specific row to an allocated `SqlRow`. To
    /// help deciding the right types, the provided `ColumnMetadata`s should map
    /// to the returned columns in the right order.
    fn to_sql_row(self, meta: &[ColumnMetadata<'_>]) -> crate::Result<SqlRow>;
}

impl ToSqlRow for ResultRow {
    fn to_sql_row(self, meta: &[ColumnMetadata<'_>]) -> crate::Result<SqlRow> {
        let mut row = SqlRow::default();
        let row_width = meta.len();

        row.values.reserve(row_width);

        for (i, p_value) in self.into_iter().enumerate().take(row_width) {
            let pv = match (meta[i].identifier(), meta[i].arity()) {
                (type_identifier, FieldArity::List) => match p_value.typed {
                    value if value.is_null() => Ok(PrismaValue::List(Vec::new())),
                    ValueType::Array(None) => Ok(PrismaValue::List(Vec::new())),
                    ValueType::Array(Some(l)) => l
                        .into_iter()
                        .map(|val| row_value_to_prisma_value(val, meta[i]))
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

fn row_value_to_prisma_value(p_value: Value, meta: ColumnMetadata<'_>) -> Result<PrismaValue, SqlError> {
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
        TypeIdentifier::Boolean => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Int32(Some(i)) => PrismaValue::Boolean(i != 0),
            ValueType::Int64(Some(i)) => PrismaValue::Boolean(i != 0),
            ValueType::Boolean(Some(b)) => PrismaValue::Boolean(b),
            ValueType::Bytes(Some(bytes)) if bytes.as_ref() == [0u8] => PrismaValue::Boolean(false),
            ValueType::Bytes(Some(bytes)) if bytes.as_ref() == [1u8] => PrismaValue::Boolean(true),
            ValueType::Double(Some(i)) => PrismaValue::Boolean(i.to_i64().unwrap() != 0),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Enum(_) => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Enum(Some(cow), _) => PrismaValue::Enum(cow.into_owned()),
            ValueType::Text(Some(cow)) => PrismaValue::Enum(cow.into_owned()),
            _ => return Err(create_error(&p_value)),
        },

        TypeIdentifier::Json => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Text(Some(json)) => PrismaValue::Json(json.into()),
            ValueType::Json(Some(json)) => PrismaValue::Json(json.to_string()),
            ValueType::Boolean(Some(bool)) => PrismaValue::Json(bool.to_string()),
            ValueType::Int32(Some(i)) => PrismaValue::Json(i.to_string()),
            ValueType::Int64(Some(i)) => PrismaValue::Json(i.to_string()),
            // we use serde_json::Number to ensure JSON-compliant float formatting
            ValueType::Float(Some(f)) => serde_json::Number::from_f64(f.into())
                .map(|num| PrismaValue::Json(num.to_string()))
                .ok_or_else(|| create_error(&p_value))?,
            ValueType::Double(Some(f)) => serde_json::Number::from_f64(f)
                .map(|num| PrismaValue::Json(num.to_string()))
                .ok_or_else(|| create_error(&p_value))?,
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::UUID => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Text(Some(uuid)) => PrismaValue::Uuid(Uuid::parse_str(&uuid)?),
            ValueType::Uuid(Some(uuid)) => PrismaValue::Uuid(uuid),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::DateTime => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            value if value.is_integer() => {
                let ts = value.as_integer().unwrap();
                let nsecs = ((ts % 1000) * 1_000_000) as u32;
                let secs = ts / 1000;
                let datetime = chrono::DateTime::from_timestamp(secs, nsecs).unwrap();

                PrismaValue::DateTime(datetime.into())
            }
            ValueType::DateTime(Some(dt)) => PrismaValue::DateTime(dt.into()),
            ValueType::Text(Some(ref dt_string)) => {
                let dt = DateTime::parse_from_rfc3339(dt_string)
                    .or_else(|_| DateTime::parse_from_rfc2822(dt_string))
                    .map_err(|_| create_error(&p_value))?;

                PrismaValue::DateTime(dt.with_timezone(&Utc).into())
            }
            ValueType::Date(Some(d)) => {
                let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
                PrismaValue::DateTime(dt.into())
            }
            ValueType::Time(Some(t)) => {
                let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_time(t), Utc);
                PrismaValue::DateTime(dt.into())
            }
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Float | TypeIdentifier::Decimal => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Numeric(Some(f)) => PrismaValue::Float(f.normalized()),
            ValueType::Double(Some(f)) => match f {
                f if f.is_nan() => return Err(create_error(&p_value)),
                f if f.is_infinite() => return Err(create_error(&p_value)),
                _ => PrismaValue::Float(BigDecimal::from_f64(f).unwrap().normalized()),
            },
            ValueType::Float(Some(f)) => match f {
                f if f.is_nan() => return Err(create_error(&p_value)),
                f if f.is_infinite() => return Err(create_error(&p_value)),
                _ => PrismaValue::Float(BigDecimal::from_f32(f).unwrap().normalized()),
            },
            ValueType::Int32(Some(i)) => match BigDecimal::from_i32(i) {
                Some(dec) => PrismaValue::Float(dec),
                None => return Err(create_error(&p_value)),
            },
            ValueType::Int64(Some(i)) => match BigDecimal::from_i64(i) {
                Some(dec) => PrismaValue::Float(dec),
                None => return Err(create_error(&p_value)),
            },
            ValueType::Text(_) | ValueType::Bytes(_) => {
                let dec: BigDecimal = p_value
                    .as_str()
                    .expect("text/bytes as str")
                    .parse()
                    .map_err(|_| create_error(&p_value))?;

                PrismaValue::Float(dec.normalized())
            }
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Int => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Int32(Some(i)) => PrismaValue::Int(i as i64),
            ValueType::Int64(Some(i)) => PrismaValue::Int(i),
            ValueType::Bytes(Some(bytes)) => PrismaValue::Int(interpret_bytes_as_i64(&bytes)),
            ValueType::Text(Some(ref txt)) => {
                PrismaValue::Int(i64::from_str(txt.trim_start_matches('\0')).map_err(|_| create_error(&p_value))?)
            }
            ValueType::Float(Some(f)) => {
                sanitize_f32(f, "Int")?;

                PrismaValue::Int(big_decimal_to_i64(BigDecimal::from_f32(f).unwrap(), "Int")?)
            }
            ValueType::Double(Some(f)) => {
                sanitize_f64(f, "Int")?;

                PrismaValue::Int(big_decimal_to_i64(BigDecimal::from_f64(f).unwrap(), "Int")?)
            }
            ValueType::Numeric(Some(dec)) => PrismaValue::Int(big_decimal_to_i64(dec, "Int")?),
            ValueType::Boolean(Some(bool)) => PrismaValue::Int(bool as i64),
            other => to_prisma_value(other)?,
        },
        TypeIdentifier::BigInt => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Int32(Some(i)) => PrismaValue::BigInt(i as i64),
            ValueType::Int64(Some(i)) => PrismaValue::BigInt(i),
            ValueType::Bytes(Some(bytes)) => PrismaValue::BigInt(interpret_bytes_as_i64(&bytes)),
            ValueType::Text(Some(ref txt)) => {
                PrismaValue::BigInt(i64::from_str(txt.trim_start_matches('\0')).map_err(|_| create_error(&p_value))?)
            }
            ValueType::Float(Some(f)) => {
                sanitize_f32(f, "BigInt")?;

                PrismaValue::BigInt(big_decimal_to_i64(BigDecimal::from_f32(f).unwrap(), "BigInt")?)
            }
            ValueType::Double(Some(f)) => {
                sanitize_f64(f, "BigInt")?;

                PrismaValue::BigInt(big_decimal_to_i64(BigDecimal::from_f64(f).unwrap(), "BigInt")?)
            }
            ValueType::Numeric(Some(dec)) => PrismaValue::BigInt(big_decimal_to_i64(dec, "BigInt")?),
            ValueType::Boolean(Some(bool)) => PrismaValue::BigInt(bool as i64),
            other => to_prisma_value(other)?,
        },
        TypeIdentifier::String => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Uuid(Some(uuid)) => PrismaValue::String(uuid.to_string()),
            ValueType::Json(Some(ref json_value)) => {
                PrismaValue::String(serde_json::to_string(json_value).map_err(|_| create_error(&p_value))?)
            }
            other => to_prisma_value(other)?,
        },
        TypeIdentifier::Bytes => match p_value.typed {
            value if value.is_null() => PrismaValue::Null,
            ValueType::Bytes(Some(bytes)) => PrismaValue::Bytes(bytes.into()),
            _ => return Err(create_error(&p_value)),
        },
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    })
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

pub(crate) fn sanitize_f32(n: f32, to: &'static str) -> crate::Result<()> {
    if n.is_nan() {
        return Err(ConversionFailure::new("NaN", to).into());
    }

    if n.is_infinite() {
        return Err(ConversionFailure::new("Infinity", to).into());
    }

    Ok(())
}

pub(crate) fn sanitize_f64(n: f64, to: &'static str) -> crate::Result<()> {
    if n.is_nan() {
        return Err(ConversionFailure::new("NaN", to).into());
    }

    if n.is_infinite() {
        return Err(ConversionFailure::new("Infinity", to).into());
    }

    Ok(())
}

pub(crate) fn big_decimal_to_i64(dec: BigDecimal, to: &'static str) -> Result<i64, SqlError> {
    dec.normalized()
        .to_i64()
        .ok_or_else(|| SqlError::from(ConversionFailure::new(format!("BigDecimal({dec})"), to)))
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
