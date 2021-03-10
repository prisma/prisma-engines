#[cfg(feature = "bigdecimal")]
mod decimal;

use crate::{
    ast::Value,
    connector::queryable::{GetRow, ToColumnNames},
    error::{Error, ErrorKind},
};
#[cfg(feature = "bigdecimal")]
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use bit_vec::BitVec;
use bytes::BytesMut;
#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDateTime, Utc};
#[cfg(feature = "bigdecimal")]
pub(crate) use decimal::DecimalWrapper;
#[cfg(feature = "bigdecimal")]
use num_bigint::BigInt;
use postgres_types::{FromSql, ToSql};
use std::error::Error as StdError;
use tokio_postgres::{
    types::{self, IsNull, Kind, Type as PostgresType},
    Row as PostgresRow, Statement as PostgresStatement,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[tracing::instrument(skip(params))]
pub fn conv_params<'a>(params: &'a [Value<'a>]) -> Vec<&'a (dyn types::ToSql + Sync)> {
    params.iter().map(|x| x as &(dyn ToSql + Sync)).collect::<Vec<_>>()
}

struct XmlString(pub String);

impl<'a> FromSql<'a> for XmlString {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<XmlString, Box<dyn std::error::Error + Sync + Send>> {
        Ok(XmlString(String::from_utf8(raw.to_owned()).unwrap()))
    }

    fn accepts(ty: &PostgresType) -> bool {
        ty == &PostgresType::XML
    }
}

struct EnumString {
    value: String,
}

impl<'a> FromSql<'a> for EnumString {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<EnumString, Box<dyn std::error::Error + Sync + Send>> {
        Ok(EnumString {
            value: String::from_utf8(raw.to_owned()).unwrap(),
        })
    }

    fn accepts(_ty: &PostgresType) -> bool {
        true
    }
}

#[cfg(feature = "chrono")]
struct TimeTz(chrono::NaiveTime);

#[cfg(feature = "chrono")]
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
#[cfg(feature = "bigdecimal")]
struct NaiveMoney(BigDecimal);

#[cfg(feature = "bigdecimal")]
impl<'a> FromSql<'a> for NaiveMoney {
    fn from_sql(_ty: &PostgresType, raw: &'a [u8]) -> Result<NaiveMoney, Box<dyn std::error::Error + Sync + Send>> {
        let cents = i64::from_sql(&PostgresType::INT8, raw)?;

        Ok(NaiveMoney(BigDecimal::new(BigInt::from_i64(cents).unwrap(), 2)))
    }

    fn accepts(ty: &PostgresType) -> bool {
        ty == &PostgresType::MONEY
    }
}

impl GetRow for PostgresRow {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &PostgresRow, i: usize) -> crate::Result<Value<'static>> {
            let result = match *row.columns()[i].type_() {
                PostgresType::BOOL => Value::Boolean(row.try_get(i)?),
                PostgresType::INT2 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i16 = val;
                        Value::integer(val)
                    }
                    None => Value::Integer(None),
                },
                PostgresType::INT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i32 = val;
                        Value::integer(val)
                    }
                    None => Value::Integer(None),
                },
                PostgresType::INT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i64 = val;
                        Value::integer(val)
                    }
                    None => Value::Integer(None),
                },
                PostgresType::FLOAT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f32 = val;
                        Value::float(val)
                    }
                    None => Value::Float(None),
                },
                PostgresType::FLOAT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f64 = val;
                        Value::double(val)
                    }
                    None => Value::Double(None),
                },
                PostgresType::BYTEA => match row.try_get(i)? {
                    Some(val) => {
                        let val: &[u8] = val;
                        Value::bytes(val.to_owned())
                    }
                    None => Value::Bytes(None),
                },
                PostgresType::BYTEA_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Vec<u8>> = val;
                        let byteas = val.into_iter().map(Value::bytes);
                        Value::array(byteas)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "bigdecimal")]
                PostgresType::NUMERIC => {
                    let dw: Option<DecimalWrapper> = row.try_get(i)?;

                    Value::Numeric(dw.map(|dw| dw.0))
                }
                #[cfg(feature = "bigdecimal")]
                PostgresType::MONEY => match row.try_get(i)? {
                    Some(val) => {
                        let val: NaiveMoney = val;
                        Value::numeric(val.0)
                    }
                    None => Value::Numeric(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMESTAMP => match row.try_get(i)? {
                    Some(val) => {
                        let ts: NaiveDateTime = val;
                        let dt = DateTime::<Utc>::from_utc(ts, Utc);
                        Value::datetime(dt)
                    }
                    None => Value::DateTime(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMESTAMPTZ => match row.try_get(i)? {
                    Some(val) => {
                        let ts: DateTime<Utc> = val;
                        Value::datetime(ts)
                    }
                    None => Value::DateTime(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::DATE => match row.try_get(i)? {
                    Some(val) => Value::date(val),
                    None => Value::Date(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIME => match row.try_get(i)? {
                    Some(val) => Value::time(val),
                    None => Value::Time(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMETZ => match row.try_get(i)? {
                    Some(val) => {
                        let time: TimeTz = val;
                        Value::time(time.0)
                    }
                    None => Value::Time(None),
                },
                #[cfg(feature = "uuid")]
                PostgresType::UUID => match row.try_get(i)? {
                    Some(val) => {
                        let val: Uuid = val;
                        Value::uuid(val)
                    }
                    None => Value::Uuid(None),
                },
                #[cfg(feature = "uuid")]
                PostgresType::UUID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Uuid> = val;
                        let val = val.into_iter().map(Value::uuid);
                        Value::array(val)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "json")]
                PostgresType::JSON | PostgresType::JSONB => Value::Json(row.try_get(i)?),
                PostgresType::INT2_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i16> = val;
                        let ints = val.into_iter().map(Value::integer);
                        Value::array(ints)
                    }
                    None => Value::Array(None),
                },
                PostgresType::INT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i32> = val;
                        let ints = val.into_iter().map(Value::integer);
                        Value::array(ints)
                    }
                    None => Value::Array(None),
                },
                PostgresType::INT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<i64> = val;
                        let ints = val.into_iter().map(Value::integer);
                        Value::array(ints)
                    }
                    None => Value::Array(None),
                },
                PostgresType::FLOAT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f32> = val;
                        let floats = val.into_iter().map(Value::float);
                        Value::array(floats)
                    }
                    None => Value::Array(None),
                },
                PostgresType::FLOAT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<f64> = val;
                        let floats = val.into_iter().map(Value::double);
                        Value::array(floats)
                    }
                    None => Value::Array(None),
                },
                PostgresType::BOOL_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<bool> = val;
                        let bools = val.into_iter().map(Value::from);
                        Value::array(bools)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMESTAMP_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveDateTime> = val;

                        let dates = val
                            .into_iter()
                            .map(|x| Value::datetime(DateTime::<Utc>::from_utc(x, Utc)));

                        Value::array(dates)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "bigdecimal")]
                PostgresType::NUMERIC_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<DecimalWrapper> = val;

                        let decimals = val
                            .into_iter()
                            .map(|x| Value::numeric(x.0.to_string().parse().unwrap()));

                        Value::array(decimals)
                    }
                    None => Value::Array(None),
                },
                PostgresType::TEXT_ARRAY | PostgresType::NAME_ARRAY | PostgresType::VARCHAR_ARRAY => {
                    match row.try_get(i)? {
                        Some(val) => {
                            let strings: Vec<&str> = val;
                            Value::array(strings.into_iter().map(|s| s.to_string()))
                        }
                        None => Value::Array(None),
                    }
                }
                #[cfg(feature = "bigdecimal")]
                PostgresType::MONEY_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<NaiveMoney> = val;
                        let nums = val.into_iter().map(|x| Value::numeric(x.0));
                        Value::array(nums)
                    }
                    None => Value::Array(None),
                },
                PostgresType::OID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<u32> = val;
                        let nums = val.into_iter().map(|x| Value::integer(x as i64));
                        Value::array(nums)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMESTAMPTZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<DateTime<Utc>> = val;
                        let dates = val.into_iter().map(Value::datetime);
                        Value::array(dates)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::DATE_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<chrono::NaiveDate> = val;
                        Value::array(val.into_iter().map(Value::date))
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIME_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<chrono::NaiveTime> = val;
                        Value::array(val.into_iter().map(Value::time))
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "chrono")]
                PostgresType::TIMETZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<TimeTz> = val;

                        let dates = val.into_iter().map(|time| Value::time(time.0));

                        Value::array(dates)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "json")]
                PostgresType::JSON_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<serde_json::Value> = val;
                        let jsons = val.into_iter().map(Value::json);
                        Value::array(jsons)
                    }
                    None => Value::Array(None),
                },
                #[cfg(feature = "json")]
                PostgresType::JSONB_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<serde_json::Value> = val;
                        let jsons = val.into_iter().map(Value::json);
                        Value::array(jsons)
                    }
                    None => Value::Array(None),
                },
                PostgresType::OID => match row.try_get(i)? {
                    Some(val) => {
                        let val: u32 = val;
                        Value::integer(val)
                    }
                    None => Value::Integer(None),
                },
                PostgresType::CHAR => match row.try_get(i)? {
                    Some(val) => {
                        let val: i8 = val;
                        Value::character((val as u8) as char)
                    }
                    None => Value::Char(None),
                },
                PostgresType::INET | PostgresType::CIDR => match row.try_get(i)? {
                    Some(val) => {
                        let val: std::net::IpAddr = val;
                        Value::text(val.to_string())
                    }
                    None => Value::Text(None),
                },
                PostgresType::INET_ARRAY | PostgresType::CIDR_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<std::net::IpAddr> = val;
                        let addrs = val.into_iter().map(|v| Value::text(v.to_string()));
                        Value::array(addrs)
                    }
                    None => Value::Array(None),
                },
                PostgresType::BIT | PostgresType::VARBIT => match row.try_get(i)? {
                    Some(val) => {
                        let val: BitVec = val;
                        Value::text(bits_to_string(&val)?)
                    }
                    None => Value::Text(None),
                },
                PostgresType::BIT_ARRAY | PostgresType::VARBIT_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<BitVec> = val;

                        let stringified = val
                            .into_iter()
                            .map(|bits| bits_to_string(&bits).map(Value::text))
                            .collect::<crate::Result<Vec<_>>>()?;

                        Value::array(stringified)
                    }
                    None => Value::Array(None),
                },
                PostgresType::XML => match row.try_get(i)? {
                    Some(val) => {
                        let val: XmlString = val;
                        Value::xml(val.0)
                    }
                    None => Value::Xml(None),
                },
                PostgresType::XML_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<XmlString> = val;
                        Value::array(val.into_iter().map(|v| v.0))
                    }
                    None => Value::Array(None),
                },
                ref x => match x.kind() {
                    Kind::Enum(_) => match row.try_get(i)? {
                        Some(val) => {
                            let val: EnumString = val;
                            Value::enum_variant(val.value)
                        }
                        None => Value::Enum(None),
                    },
                    Kind::Array(inner) => match inner.kind() {
                        Kind::Enum(_) => match row.try_get(i)? {
                            Some(val) => {
                                let val: Vec<EnumString> = val;
                                let variants = val.into_iter().map(|x| Value::enum_variant(x.value));
                                Value::array(variants)
                            }
                            None => Value::Array(None),
                        },
                        _ => match row.try_get(i)? {
                            Some(val) => {
                                let val: Vec<String> = val;
                                let strings = val.into_iter().map(Value::text);
                                Value::array(strings)
                            }
                            None => Value::Array(None),
                        },
                    },
                    _ => match row.try_get(i)? {
                        Some(val) => {
                            let val: String = val;
                            Value::text(val)
                        }
                        None => Value::Text(None),
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
        self.columns().iter().map(|c| c.name().into()).collect()
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(
        &self,
        ty: &PostgresType,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + 'static + Send + Sync>> {
        let res = match (self, ty) {
            (Value::Integer(integer), &PostgresType::INT2) => integer.map(|integer| (integer as i16).to_sql(ty, out)),
            (Value::Integer(integer), &PostgresType::INT4) => integer.map(|integer| (integer as i32).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (Value::Integer(integer), &PostgresType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i64(integer).unwrap())
                .map(|bd| DecimalWrapper(bd))
                .map(|dw| dw.to_sql(ty, out)),
            (Value::Integer(integer), &PostgresType::TEXT) => {
                integer.map(|integer| format!("{}", integer).to_sql(ty, out))
            }
            (Value::Integer(integer), &PostgresType::OID) => integer.map(|integer| (integer as u32).to_sql(ty, out)),
            (Value::Integer(integer), _) => integer.map(|integer| (integer as i64).to_sql(ty, out)),
            (Value::Float(float), &PostgresType::FLOAT8) => float.map(|float| (float as f64).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (Value::Float(float), &PostgresType::NUMERIC) => float
                .map(|float| BigDecimal::from_f32(float).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (Value::Float(float), _) => float.map(|float| float.to_sql(ty, out)),
            (Value::Double(double), &PostgresType::FLOAT4) => double.map(|double| (double as f32).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (Value::Double(double), &PostgresType::NUMERIC) => double
                .map(|double| BigDecimal::from_f64(double).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (Value::Double(double), _) => double.map(|double| double.to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (Value::Numeric(decimal), &PostgresType::FLOAT4) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_f32().expect("decimal to f32 conversion");
                f.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (Value::Numeric(decimal), &PostgresType::FLOAT8) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_f64().expect("decimal to f64 conversion");
                f.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (Value::Array(values), &PostgresType::FLOAT4_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match value {
                        Value::Numeric(n) => n.as_ref().and_then(|n| n.to_f32()),
                        Value::Float(f) => *f,
                        Value::Double(d) => d.map(|d| d as f32),
                        v => {
                            let kind = ErrorKind::conversion(format!(
                                "Couldn't add value of type `{:?}` into a float array.",
                                v
                            ));

                            Err(Error::builder(kind).build())?
                        }
                    };

                    floats.push(float);
                }

                floats.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (Value::Array(values), &PostgresType::FLOAT8_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match value {
                        Value::Numeric(n) => n.as_ref().and_then(|n| n.to_f64()),
                        Value::Float(f) => f.map(|f| f as f64),
                        Value::Double(d) => *d,
                        v => {
                            let kind = ErrorKind::conversion(format!(
                                "Couldn't add value of type `{:?}` into a double array.",
                                v
                            ));

                            Err(Error::builder(kind).build())?
                        }
                    };

                    floats.push(float);
                }

                floats.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (Value::Numeric(decimal), &PostgresType::MONEY) => decimal.as_ref().map(|decimal| {
                let decimal = (decimal * BigInt::from_i32(100).unwrap()).round(0);

                let i = decimal.to_i64().ok_or_else(|| {
                    let kind = ErrorKind::conversion("Couldn't convert BigDecimal to i64.");
                    Error::builder(kind).build()
                })?;

                i.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (Value::Numeric(decimal), &PostgresType::NUMERIC) => decimal
                .as_ref()
                .map(|decimal| DecimalWrapper(decimal.clone()).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (Value::Numeric(float), _) => float
                .as_ref()
                .map(|float| DecimalWrapper(float.clone()).to_sql(ty, out)),
            #[cfg(feature = "uuid")]
            (Value::Text(string), &PostgresType::UUID) => string.as_ref().map(|string| {
                let parsed_uuid: Uuid = string.parse()?;
                parsed_uuid.to_sql(ty, out)
            }),
            #[cfg(feature = "uuid")]
            (Value::Array(values), &PostgresType::UUID_ARRAY) => values.as_ref().map(|values| {
                let parsed_uuid: Vec<Uuid> = values
                    .iter()
                    .filter_map(|v| v.to_string().and_then(|v| v.parse().ok()))
                    .collect();
                parsed_uuid.to_sql(ty, out)
            }),
            (Value::Text(string), &PostgresType::INET) | (Value::Text(string), &PostgresType::CIDR) => {
                string.as_ref().map(|string| {
                    let parsed_ip_addr: std::net::IpAddr = string.parse()?;
                    parsed_ip_addr.to_sql(ty, out)
                })
            }
            (Value::Array(values), &PostgresType::INET_ARRAY) | (Value::Array(values), &PostgresType::CIDR_ARRAY) => {
                values.as_ref().map(|values| {
                    let parsed_ip_addr: Vec<std::net::IpAddr> = values
                        .iter()
                        .filter_map(|v| v.to_string().and_then(|s| s.parse().ok()))
                        .collect();
                    parsed_ip_addr.to_sql(ty, out)
                })
            }
            #[cfg(feature = "json")]
            (Value::Text(string), &PostgresType::JSON) | (Value::Text(string), &PostgresType::JSONB) => string
                .as_ref()
                .map(|string| serde_json::from_str::<serde_json::Value>(&string)?.to_sql(ty, out)),
            (Value::Text(string), &PostgresType::BIT) | (Value::Text(string), &PostgresType::VARBIT) => {
                string.as_ref().map(|string| {
                    let bits: BitVec = string_to_bits(string)?;

                    bits.to_sql(ty, out)
                })
            }
            (Value::Text(string), _) => string.as_ref().map(|ref string| string.to_sql(ty, out)),
            (Value::Array(values), &PostgresType::BIT_ARRAY) | (Value::Array(values), &PostgresType::VARBIT_ARRAY) => {
                values.as_ref().map(|values| {
                    let bitvecs: Vec<BitVec> = values
                        .iter()
                        .filter_map(|val| val.as_str().map(|s| string_to_bits(s)))
                        .collect::<crate::Result<Vec<_>>>()?;

                    bitvecs.to_sql(ty, out)
                })
            }
            (Value::Bytes(bytes), _) => bytes.as_ref().map(|bytes| bytes.as_ref().to_sql(ty, out)),
            (Value::Enum(string), _) => string.as_ref().map(|string| {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }),
            (Value::Boolean(boo), _) => boo.map(|boo| boo.to_sql(ty, out)),
            (Value::Char(c), _) => c.map(|c| (c as i8).to_sql(ty, out)),
            (Value::Array(vec), _) => vec.as_ref().map(|vec| vec.to_sql(ty, out)),
            #[cfg(feature = "json")]
            (Value::Json(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (Value::Xml(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            #[cfg(feature = "uuid")]
            (Value::Uuid(value), _) => value.map(|value| value.to_sql(ty, out)),
            #[cfg(feature = "chrono")]
            (Value::DateTime(value), &PostgresType::DATE) => {
                value.map(|value| value.date().naive_utc().to_sql(ty, out))
            }
            #[cfg(feature = "chrono")]
            (Value::Date(value), _) => value.map(|value| value.to_sql(ty, out)),
            #[cfg(feature = "chrono")]
            (Value::Time(value), _) => value.map(|value| value.to_sql(ty, out)),
            #[cfg(feature = "chrono")]
            (Value::DateTime(value), &PostgresType::TIME) => value.map(|value| value.time().to_sql(ty, out)),
            #[cfg(feature = "chrono")]
            (Value::DateTime(value), &PostgresType::TIMETZ) => value.map(|value| {
                let result = value.time().to_sql(ty, out)?;
                // We assume UTC. see https://www.postgresql.org/docs/9.5/datatype-datetime.html
                out.extend_from_slice(&[0; 4]);
                Ok(result)
            }),
            #[cfg(feature = "chrono")]
            (Value::DateTime(value), _) => value.map(|value| value.naive_utc().to_sql(ty, out)),
        };

        match res {
            Some(res) => res,
            None => Ok(IsNull::Yes),
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
                let msg = "Unexpected character for bits input. Expected only 1 and 0.";
                let kind = ErrorKind::conversion(msg);

                return Err(Error::builder(kind).build());
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
