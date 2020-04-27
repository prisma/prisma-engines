use crate::{
    ast::Value,
    connector::queryable::{GetRow, ToColumnNames},
    error::{Error, ErrorKind},
};
use bit_vec::BitVec;
use bytes::BytesMut;
#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use std::{error::Error as StdError, str::FromStr};
use tokio_postgres::{
    types::{self, FromSql, IsNull, Kind, ToSql, Type as PostgresType},
    Row as PostgresRow, Statement as PostgresStatement,
};

#[cfg(feature = "uuid-0_8")]
use uuid::Uuid;

pub fn conv_params<'a>(params: &'a [Value<'a>]) -> Vec<&'a (dyn types::ToSql + Sync)> {
    params.iter().map(|x| x as &(dyn ToSql + Sync)).collect::<Vec<_>>()
}

struct EnumString {
    value: String,
}

impl<'a> FromSql<'a> for EnumString {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<EnumString, Box<dyn std::error::Error + Sync + Send>> {
        Ok(EnumString {
            value: String::from_utf8(raw.to_owned()).unwrap().into(),
        })
    }

    fn accepts(_ty: &PostgresType) -> bool {
        true
    }
}

struct TimeTz(chrono::NaiveTime);

impl<'a> FromSql<'a> for TimeTz {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<TimeTz, Box<dyn std::error::Error + Sync + Send>> {
        // We assume UTC.
        let time: chrono::NaiveTime = chrono::NaiveTime::from_sql(&PostgresType::TIMETZ, &raw[..8])?;
        Ok(TimeTz(time))
    }

    fn accepts(ty: &PostgresType) -> bool {
        ty == &PostgresType::TIMETZ
    }
}

/// This implementation of FromSql assumes that the precision for money fields is configured to the default
/// of 2 decimals.
///
/// Postgres docs: https://www.postgresql.org/docs/current/datatype-money.html
struct NaiveMoney(Decimal);

impl<'a> FromSql<'a> for NaiveMoney {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<NaiveMoney, Box<dyn std::error::Error + Sync + Send>> {
        let cents = i64::from_sql(&PostgresType::INT8, raw)?;

        Ok(NaiveMoney(Decimal::new(cents, 2)))
    }

    fn accepts(ty: &PostgresType) -> bool {
        ty == &PostgresType::MONEY
    }
}

impl GetRow for PostgresRow {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &PostgresRow, i: usize) -> crate::Result<Value<'static>> {
            let result = match *row.columns()[i].type_() {
                PostgresType::BOOL => match row.try_get(i)? {
                    Some(val) => Value::Boolean(val),
                    None => Value::Null,
                },
                PostgresType::INT2 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i16 = val;
                        Value::Integer(i64::from(val))
                    }
                    None => Value::Null,
                },
                PostgresType::INT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i32 = val;
                        Value::Integer(i64::from(val))
                    }
                    None => Value::Null,
                },
                PostgresType::INT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i64 = val;
                        Value::Integer(val)
                    }
                    None => Value::Null,
                },
                PostgresType::NUMERIC => match row.try_get(i)? {
                    Some(val) => {
                        let val: Decimal = val;
                        Value::Real(val)
                    }
                    None => Value::Null,
                },
                PostgresType::FLOAT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: Decimal = Decimal::from_f32(val).expect("f32 is not a Decimal");
                        Value::Real(val)
                    }
                    None => Value::Null,
                },
                PostgresType::FLOAT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f64 = val;
                        // Decimal::from_f64 is buggy. Issue: https://github.com/paupino/rust-decimal/issues/228
                        let val: Decimal = Decimal::from_str(&val.to_string()).expect("f64 is not a Decimal");
                        Value::Real(val)
                    }
                    None => Value::Null,
                },
                PostgresType::MONEY => match row.try_get(i)? {
                    Some(val) => {
                        let val: NaiveMoney = val;
                        Value::Real(val.0)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "chrono-0_4")]
                PostgresType::TIMESTAMP => match row.try_get(i)? {
                    Some(val) => {
                        let ts: NaiveDateTime = val;
                        let dt = DateTime::<Utc>::from_utc(ts, Utc);
                        Value::DateTime(dt)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "chrono-0_4")]
                PostgresType::TIMESTAMPTZ => match row.try_get(i)? {
                    Some(val) => {
                        let ts: DateTime<Utc> = val;
                        Value::DateTime(ts)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "chrono-0_4")]
                PostgresType::DATE => match row.try_get(i)? {
                    Some(val) => {
                        let ts: chrono::NaiveDate = val;
                        let dt = ts.and_time(chrono::NaiveTime::from_hms(0, 0, 0));
                        Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "chrono-0_4")]
                PostgresType::TIME => match row.try_get(i)? {
                    Some(val) => {
                        let time: chrono::NaiveTime = val;
                        let dt = NaiveDateTime::new(chrono::NaiveDate::from_ymd(1970, 1, 1), time);
                        Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "chrono-0_4")]
                PostgresType::TIMETZ => match row.try_get(i)? {
                    Some(val) => {
                        let time: TimeTz = val;
                        let dt = NaiveDateTime::new(chrono::NaiveDate::from_ymd(1970, 1, 1), time.0);
                        Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "uuid-0_8")]
                PostgresType::UUID => match row.try_get(i)? {
                    Some(val) => {
                        let val: Uuid = val;
                        Value::Uuid(val)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "uuid-0_8")]
                PostgresType::UUID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Uuid> = val;
                        let val = val.into_iter().map(Value::Uuid).collect();
                        Value::Array(val)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "json-1")]
                PostgresType::JSON | PostgresType::JSONB => match row.try_get(i)? {
                    Some(val) => {
                        let val: serde_json::Value = val;
                        Value::Json(val)
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT2_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i16> = val;
                        Value::Array(val.into_iter().map(|x| Value::Integer(i64::from(x))).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i32> = val;
                        Value::Array(val.into_iter().map(|x| Value::Integer(i64::from(x))).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::INT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i64> = val;
                        Value::Array(val.into_iter().map(|x| Value::Integer(x as i64)).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f32> = val;
                        Value::Array(val.into_iter().map(Value::from).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::FLOAT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f64> = val;
                        Value::Array(val.into_iter().map(Value::from).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::BOOL_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<bool> = val;
                        Value::Array(val.into_iter().map(Value::Boolean).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(all(feature = "array", feature = "chrono-0_4"))]
                PostgresType::TIMESTAMP_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveDateTime> = val;
                        Value::Array(
                            val.into_iter()
                                .map(|x| Value::DateTime(DateTime::<Utc>::from_utc(x, Utc)))
                                .collect(),
                        )
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::NUMERIC_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Decimal> = val;
                        Value::Array(
                            val.into_iter()
                                .map(|x| Value::Real(x.to_string().parse().unwrap()))
                                .collect(),
                        )
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TEXT_ARRAY | PostgresType::NAME_ARRAY | PostgresType::VARCHAR_ARRAY => {
                    match row.try_get(i)? {
                        Some(val) => {
                            let val: Vec<&str> = val;
                            Value::Array(val.into_iter().map(|x| Value::Text(String::from(x).into())).collect())
                        }
                        None => Value::Null,
                    }
                }
                #[cfg(feature = "array")]
                PostgresType::MONEY_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveMoney> = val;
                        Value::Array(val.into_iter().map(|x| Value::Real(x.0)).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::OID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<u32> = val;
                        Value::Array(val.into_iter().map(|x| Value::Integer(x as i64)).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TIMESTAMPTZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<DateTime<Utc>> = val;
                        Value::Array(val.into_iter().map(|x| Value::DateTime(x)).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::DATE_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<chrono::NaiveDate> = val;
                        Value::Array(
                            val.into_iter()
                                .map(|date| {
                                    let dt = date.and_time(chrono::NaiveTime::from_hms(0, 0, 0));
                                    Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                                })
                                .collect(),
                        )
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TIME_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<chrono::NaiveTime> = val;
                        Value::Array(
                            val.into_iter()
                                .map(|time| {
                                    let dt = NaiveDateTime::new(chrono::NaiveDate::from_ymd(1970, 1, 1), time);
                                    Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                                })
                                .collect(),
                        )
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::TIMETZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<TimeTz> = val;
                        Value::Array(
                            val.into_iter()
                                .map(|time| {
                                    let dt = NaiveDateTime::new(chrono::NaiveDate::from_ymd(1970, 1, 1), time.0);
                                    Value::DateTime(chrono::DateTime::from_utc(dt, Utc))
                                })
                                .collect(),
                        )
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::JSON_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<serde_json::Value> = val;
                        Value::Array(val.into_iter().map(|json| Value::Json(json)).collect())
                    }
                    None => Value::Null,
                },
                #[cfg(feature = "array")]
                PostgresType::JSONB_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<serde_json::Value> = val;
                        Value::Array(val.into_iter().map(|json| Value::Json(json)).collect())
                    }
                    None => Value::Null,
                },
                PostgresType::OID => match row.try_get(i)? {
                    Some(val) => {
                        let val: u32 = val;
                        Value::Integer(i64::from(val))
                    }
                    None => Value::Null,
                },
                PostgresType::CHAR => match row.try_get(i)? {
                    Some(val) => {
                        let val: i8 = val;
                        Value::Char((val as u8) as char)
                    }
                    None => Value::Null,
                },
                PostgresType::INET | PostgresType::CIDR => match row.try_get(i)? {
                    Some(val) => {
                        let val: std::net::IpAddr = val;
                        Value::Text(val.to_string().into())
                    }
                    None => Value::Null,
                },
                PostgresType::INET_ARRAY | PostgresType::CIDR_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<std::net::IpAddr> = val;
                        Value::Array(val.into_iter().map(|v| Value::Text(v.to_string().into())).collect())
                    }
                    None => Value::Null,
                },
                PostgresType::BIT | PostgresType::VARBIT => match row.try_get(i)? {
                    Some(val) => {
                        let val: BitVec = val;
                        Value::Text(bits_to_string(&val)?.into())
                    }
                    None => Value::Null,
                },
                PostgresType::BIT_ARRAY | PostgresType::VARBIT_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<BitVec> = val;
                        let stringified = val
                            .into_iter()
                            .map(|bits| bits_to_string(&bits).map(|s| Value::Text(s.into())))
                            .collect::<crate::Result<Vec<_>>>()?;

                        Value::Array(stringified)
                    }
                    None => Value::Null,
                },
                ref x => match x.kind() {
                    Kind::Enum(_) => match row.try_get(i)? {
                        Some(val) => {
                            let val: EnumString = val;
                            Value::Enum(val.value.into())
                        }
                        None => Value::Null,
                    },
                    Kind::Array(inner) => match inner.kind() {
                        Kind::Enum(_) => match row.try_get(i)? {
                            Some(val) => {
                                let val: Vec<EnumString> = val;
                                Value::Array(val.into_iter().map(|x| Value::Enum(x.value.into())).collect())
                            }
                            None => Value::Null,
                        },
                        _ => match row.try_get(i)? {
                            Some(val) => {
                                let val: Vec<String> = val;
                                Value::Array(val.into_iter().map(|x| Value::Text(x.into())).collect())
                            }
                            None => Value::Null,
                        },
                    },
                    _ => match row.try_get(i)? {
                        Some(val) => {
                            let val: String = val;
                            Value::Text(val.into())
                        }
                        None => Value::Null,
                    },
                },
            };

            Ok(result)
        }

        let num_columns = self.columns().len();
        let mut row = Vec::with_capacity(num_columns);

        for i in 0..num_columns {
            row.push(convert(self, i)?);
        }

        Ok(row)
    }
}

impl ToColumnNames for PostgresStatement {
    fn to_column_names(&self) -> Vec<String> {
        self.columns().into_iter().map(|c| c.name().into()).collect()
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(
        &self,
        ty: &PostgresType,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + 'static + Send + Sync>> {
        match (self, ty) {
            (Value::Null, _) => Ok(IsNull::Yes),
            (Value::Integer(integer), &PostgresType::INT2) => (*integer as i16).to_sql(ty, out),
            (Value::Integer(integer), &PostgresType::INT4) => (*integer as i32).to_sql(ty, out),
            (Value::Integer(integer), &PostgresType::TEXT) => format!("{}", integer).to_sql(ty, out),
            (Value::Integer(integer), &PostgresType::OID) => (*integer as u32).to_sql(ty, out),
            (Value::Integer(integer), _) => (*integer as i64).to_sql(ty, out),
            (Value::Real(decimal), &PostgresType::FLOAT4) => {
                let f = decimal.to_f32().expect("decimal to f32 conversion");
                f.to_sql(ty, out)
            }
            (Value::Real(decimal), &PostgresType::FLOAT8) => {
                let f = decimal.to_f64().expect("decimal to f64 conversion");
                f.to_sql(ty, out)
            }
            (Value::Array(decimals), &PostgresType::FLOAT4_ARRAY) => {
                let f: Vec<f32> = decimals
                    .into_iter()
                    .filter_map(|v| v.as_decimal().and_then(|decimal| decimal.to_f32()))
                    .collect();
                f.to_sql(ty, out)
            }
            (Value::Array(decimals), &PostgresType::FLOAT8_ARRAY) => {
                let f: Vec<f64> = decimals
                    .into_iter()
                    .filter_map(|v| v.as_decimal().and_then(|decimal| decimal.to_f64()))
                    .collect();
                f.to_sql(ty, out)
            }
            (Value::Real(decimal), &PostgresType::MONEY) => {
                let mut i64_bytes: [u8; 8] = [0; 8];
                let decimal = (decimal * Decimal::new(100, 0)).round();
                i64_bytes.copy_from_slice(&decimal.serialize()[4..12]);
                let i = i64::from_le_bytes(i64_bytes);
                i.to_sql(ty, out)
            }
            (Value::Real(decimal), &PostgresType::NUMERIC) => decimal.to_sql(ty, out),
            (Value::Real(float), _) => float.to_sql(ty, out),
            #[cfg(feature = "uuid-0_8")]
            (Value::Text(string), &PostgresType::UUID) => {
                let parsed_uuid: Uuid = string.parse()?;
                parsed_uuid.to_sql(ty, out)
            }
            #[cfg(feature = "uuid-0_8")]
            (Value::Array(values), &PostgresType::UUID_ARRAY) => {
                let parsed_uuid: Vec<Uuid> = values
                    .into_iter()
                    .filter_map(|v| v.to_string().and_then(|v| v.parse().ok()))
                    .collect();
                parsed_uuid.to_sql(ty, out)
            }
            (Value::Text(string), &PostgresType::INET) | (Value::Text(string), &PostgresType::CIDR) => {
                let parsed_ip_addr: std::net::IpAddr = string.parse()?;
                parsed_ip_addr.to_sql(ty, out)
            }
            (Value::Array(values), &PostgresType::INET_ARRAY) | (Value::Array(values), &PostgresType::CIDR_ARRAY) => {
                let parsed_ip_addr: Vec<std::net::IpAddr> = values
                    .into_iter()
                    .filter_map(|v| v.to_string().and_then(|s| s.parse().ok()))
                    .collect();
                parsed_ip_addr.to_sql(ty, out)
            }
            (Value::Text(string), &PostgresType::JSON) | (Value::Text(string), &PostgresType::JSONB) => {
                serde_json::from_str::<serde_json::Value>(&string)?.to_sql(ty, out)
            }
            (Value::Text(string), &PostgresType::BIT) | (Value::Text(string), &PostgresType::VARBIT) => {
                let bits: BitVec = string_to_bits(string)?;

                bits.to_sql(ty, out)
            }
            (Value::Text(string), _) => string.to_sql(ty, out),
            (Value::Array(values), &PostgresType::BIT_ARRAY) | (Value::Array(values), &PostgresType::VARBIT_ARRAY) => {
                let bitvecs: Vec<BitVec> = values
                    .into_iter()
                    .filter_map(|val| val.as_str().map(|s| string_to_bits(s)))
                    .collect::<crate::Result<Vec<_>>>()?;

                bitvecs.to_sql(ty, out)
            }
            (Value::Bytes(bytes), _) => bytes.as_ref().to_sql(ty, out),
            (Value::Enum(string), _) => {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }
            (Value::Boolean(boo), _) => boo.to_sql(ty, out),
            (Value::Char(c), _) => (*c as i8).to_sql(ty, out),
            #[cfg(feature = "array")]
            (Value::Array(vec), _) => vec.to_sql(ty, out),
            #[cfg(feature = "json-1")]
            (Value::Json(value), _) => value.to_sql(ty, out),
            #[cfg(feature = "uuid-0_8")]
            (Value::Uuid(value), _) => value.to_sql(ty, out),
            #[cfg(feature = "chrono-0_4")]
            (Value::DateTime(value), &PostgresType::DATE) => value.date().naive_utc().to_sql(ty, out),
            #[cfg(feature = "chrono-0_4")]
            (Value::DateTime(value), &PostgresType::TIME) => value.time().to_sql(ty, out),
            (Value::DateTime(value), &PostgresType::TIMETZ) => {
                let result = value.time().to_sql(ty, out)?;
                // We assume UTC. see https://www.postgresql.org/docs/9.5/datatype-datetime.html
                out.extend_from_slice(&[0; 4]);
                Ok(result)
            }
            #[cfg(feature = "chrono-0_4")]
            (Value::DateTime(value), _) => value.naive_utc().to_sql(ty, out),
        }
    }

    fn accepts(_: &PostgresType) -> bool {
        true // Please check later should we make this to be more restricted
    }

    tokio_postgres::types::to_sql_checked!();
}

fn string_to_bits(s: &str) -> crate::Result<BitVec> {
    use bit_vec::*;

    let mut bits = BitVec::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '0' => bits.push(false),
            '1' => bits.push(true),
            _ => {
                return Err(Error::builder(ErrorKind::ConversionError(
                    "Unexpected character for bits input. Expected only 1 and 0.",
                ))
                .build())
            }
        }
    }

    Ok(bits)
}

fn bits_to_string(bits: &BitVec) -> crate::Result<String> {
    let mut s = String::with_capacity(bits.len());

    for bit in bits {
        if bit {
            s.push('1');
        } else {
            s.push('0');
        }
    }

    Ok(s)
}
