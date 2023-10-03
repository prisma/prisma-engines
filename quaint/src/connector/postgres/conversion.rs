#[cfg(feature = "bigdecimal")]
mod decimal;

use crate::{
    ast::Value,
    connector::queryable::{GetRow, ToColumnNames},
    error::{Error, ErrorKind},
};
#[cfg(feature = "bigdecimal")]
use bigdecimal::{num_bigint::BigInt, BigDecimal, FromPrimitive, ToPrimitive};
use bit_vec::BitVec;
use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Utc};
#[cfg(feature = "bigdecimal")]
pub(crate) use decimal::DecimalWrapper;
use postgres_types::{FromSql, ToSql, WrongType};
use std::{convert::TryFrom, error::Error as StdError};
use tokio_postgres::{
    types::{self, IsNull, Kind, Type as PostgresType},
    Row as PostgresRow, Statement as PostgresStatement,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

pub(crate) fn conv_params<'a>(params: &'a [Value<'a>]) -> Vec<&'a (dyn types::ToSql + Sync)> {
    params.iter().map(|x| x as &(dyn ToSql + Sync)).collect::<Vec<_>>()
}

/// Maps a list of query parameter values to a list of Postgres type.
pub(crate) fn params_to_types(params: &[Value<'_>]) -> Vec<PostgresType> {
    params
        .iter()
        .map(|p| -> PostgresType {
            // While we can infer the underlying type of a null, Prisma can't.
            // Therefore, we let PG infer the underlying type.
            if p.is_null() {
                return PostgresType::UNKNOWN;
            }

            match p {
                ValueInner::Int32(_) => PostgresType::INT4,
                ValueInner::Int64(_) => PostgresType::INT8,
                ValueInner::Float(_) => PostgresType::FLOAT4,
                ValueInner::Double(_) => PostgresType::FLOAT8,
                ValueInner::Text(_) => PostgresType::TEXT,
                // Enums are user-defined types, we can't statically infer them, so we let PG infer it
                ValueInner::Enum(_, _) | ValueInner::EnumArray(_, _) => PostgresType::UNKNOWN,
                ValueInner::Bytes(_) => PostgresType::BYTEA,
                ValueInner::Boolean(_) => PostgresType::BOOL,
                ValueInner::Char(_) => PostgresType::CHAR,
                #[cfg(feature = "bigdecimal")]
                ValueInner::Numeric(_) => PostgresType::NUMERIC,
                ValueInner::Json(_) => PostgresType::JSONB,
                ValueInner::Xml(_) => PostgresType::XML,
                #[cfg(feature = "uuid")]
                ValueInner::Uuid(_) => PostgresType::UUID,
                ValueInner::DateTime(_) => PostgresType::TIMESTAMPTZ,
                ValueInner::Date(_) => PostgresType::TIMESTAMP,
                ValueInner::Time(_) => PostgresType::TIME,
                ValueInner::Array(ref arr) => {
                    let arr = arr.as_ref().unwrap();

                    // If the array is empty, we can't infer the type so we let PG infer it
                    if arr.is_empty() {
                        return PostgresType::UNKNOWN;
                    }

                    let first = arr.first().unwrap();

                    // If the array does not contain the same types of values, we let PG infer the type
                    if arr
                        .iter()
                        .any(|val| std::mem::discriminant(first) != std::mem::discriminant(val))
                    {
                        return PostgresType::UNKNOWN;
                    }

                    match first {
                        ValueInner::Int32(_) => PostgresType::INT4_ARRAY,
                        ValueInner::Int64(_) => PostgresType::INT8_ARRAY,
                        ValueInner::Float(_) => PostgresType::FLOAT4_ARRAY,
                        ValueInner::Double(_) => PostgresType::FLOAT8_ARRAY,
                        ValueInner::Text(_) => PostgresType::TEXT_ARRAY,
                        // Enums are special types, we can't statically infer them, so we let PG infer it
                        ValueInner::Enum(_, _) | ValueInner::EnumArray(_, _) => PostgresType::UNKNOWN,
                        ValueInner::Bytes(_) => PostgresType::BYTEA_ARRAY,
                        ValueInner::Boolean(_) => PostgresType::BOOL_ARRAY,
                        ValueInner::Char(_) => PostgresType::CHAR_ARRAY,
                        #[cfg(feature = "bigdecimal")]
                        ValueInner::Numeric(_) => PostgresType::NUMERIC_ARRAY,
                        ValueInner::Json(_) => PostgresType::JSONB_ARRAY,
                        ValueInner::Xml(_) => PostgresType::XML_ARRAY,
                        #[cfg(feature = "uuid")]
                        ValueInner::Uuid(_) => PostgresType::UUID_ARRAY,
                        ValueInner::DateTime(_) => PostgresType::TIMESTAMPTZ_ARRAY,
                        ValueInner::Date(_) => PostgresType::TIMESTAMP_ARRAY,
                        ValueInner::Time(_) => PostgresType::TIME_ARRAY,
                        // In the case of nested arrays, we let PG infer the type
                        ValueInner::Array(_) => PostgresType::UNKNOWN,
                    }
                }
            }
        })
        .collect()
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
    fn get_result_row(&self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &PostgresRow, i: usize) -> crate::Result<Value<'static>> {
            let result = match *row.columns()[i].type_() {
                PostgresType::BOOL => ValueInner::Boolean(row.try_get(i)?),
                PostgresType::INT2 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i16 = val;
                        Value::int32(val)
                    }
                    None => ValueInner::Int32(None),
                },
                PostgresType::INT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i32 = val;
                        Value::int32(val)
                    }
                    None => ValueInner::Int32(None),
                },
                PostgresType::INT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: i64 = val;
                        Value::int64(val)
                    }
                    None => ValueInner::Int64(None),
                },
                PostgresType::FLOAT4 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f32 = val;
                        Value::float(val)
                    }
                    None => ValueInner::Float(None),
                },
                PostgresType::FLOAT8 => match row.try_get(i)? {
                    Some(val) => {
                        let val: f64 = val;
                        Value::double(val)
                    }
                    None => ValueInner::Double(None),
                },
                PostgresType::BYTEA => match row.try_get(i)? {
                    Some(val) => {
                        let val: &[u8] = val;
                        Value::bytes(val.to_owned())
                    }
                    None => ValueInner::Bytes(None),
                },
                PostgresType::BYTEA_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<Vec<u8>>> = val;
                        let byteas = val.into_iter().map(|b| ValueInner::Bytes(b.map(Into::into)));

                        Value::array(byteas)
                    }
                    None => ValueInner::Array(None),
                },
                #[cfg(feature = "bigdecimal")]
                PostgresType::NUMERIC => {
                    let dw: Option<DecimalWrapper> = row.try_get(i)?;

                    ValueInner::Numeric(dw.map(|dw| dw.0))
                }
                #[cfg(feature = "bigdecimal")]
                PostgresType::MONEY => match row.try_get(i)? {
                    Some(val) => {
                        let val: NaiveMoney = val;
                        Value::numeric(val.0)
                    }
                    None => ValueInner::Numeric(None),
                },
                PostgresType::TIMESTAMP => match row.try_get(i)? {
                    Some(val) => {
                        let ts: NaiveDateTime = val;
                        let dt = DateTime::<Utc>::from_utc(ts, Utc);
                        Value::datetime(dt)
                    }
                    None => ValueInner::DateTime(None),
                },
                PostgresType::TIMESTAMPTZ => match row.try_get(i)? {
                    Some(val) => {
                        let ts: DateTime<Utc> = val;
                        Value::datetime(ts)
                    }
                    None => ValueInner::DateTime(None),
                },
                PostgresType::DATE => match row.try_get(i)? {
                    Some(val) => Value::date(val),
                    None => ValueInner::Date(None),
                },
                PostgresType::TIME => match row.try_get(i)? {
                    Some(val) => Value::time(val),
                    None => ValueInner::Time(None),
                },
                PostgresType::TIMETZ => match row.try_get(i)? {
                    Some(val) => {
                        let time: TimeTz = val;
                        Value::time(time.0)
                    }
                    None => ValueInner::Time(None),
                },
                #[cfg(feature = "uuid")]
                PostgresType::UUID => match row.try_get(i)? {
                    Some(val) => {
                        let val: Uuid = val;
                        Value::uuid(val)
                    }
                    None => ValueInner::Uuid(None),
                },
                #[cfg(feature = "uuid")]
                PostgresType::UUID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<Uuid>> = val;
                        let val = val.into_iter().map(ValueInner::Uuid);

                        Value::array(val)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::JSON | PostgresType::JSONB => ValueInner::Json(row.try_get(i)?),
                PostgresType::INT2_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<i16>> = val;
                        let ints = val.into_iter().map(|i| ValueInner::Int32(i.map(|i| i as i32)));

                        Value::array(ints)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::INT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<i32>> = val;
                        let ints = val.into_iter().map(ValueInner::Int32);

                        Value::array(ints)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::INT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<i64>> = val;
                        let ints = val.into_iter().map(ValueInner::Int64);

                        Value::array(ints)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::FLOAT4_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<f32>> = val;
                        let floats = val.into_iter().map(ValueInner::Float);

                        Value::array(floats)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::FLOAT8_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<f64>> = val;
                        let floats = val.into_iter().map(ValueInner::Double);

                        Value::array(floats)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::BOOL_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<bool>> = val;
                        let bools = val.into_iter().map(ValueInner::Boolean);

                        Value::array(bools)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::TIMESTAMP_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<NaiveDateTime>> = val;

                        let dates = val
                            .into_iter()
                            .map(|dt| ValueInner::DateTime(dt.map(|dt| DateTime::<Utc>::from_utc(dt, Utc))));

                        Value::array(dates)
                    }
                    None => ValueInner::Array(None),
                },
                #[cfg(feature = "bigdecimal")]
                PostgresType::NUMERIC_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<DecimalWrapper>> = val;

                        let decimals = val
                            .into_iter()
                            .map(|dec| ValueInner::Numeric(dec.map(|dec| dec.0.to_string().parse().unwrap())));

                        Value::array(decimals)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::TEXT_ARRAY | PostgresType::NAME_ARRAY | PostgresType::VARCHAR_ARRAY => {
                    match row.try_get(i)? {
                        Some(val) => {
                            let strings: Vec<Option<&str>> = val;

                            Value::array(strings.into_iter().map(|s| s.map(|s| s.to_string())))
                        }
                        None => ValueInner::Array(None),
                    }
                }
                #[cfg(feature = "bigdecimal")]
                PostgresType::MONEY_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<NaiveMoney>> = val;
                        let nums = val.into_iter().map(|num| ValueInner::Numeric(num.map(|num| num.0)));

                        Value::array(nums)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::OID_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<u32>> = val;
                        let nums = val.into_iter().map(|oid| ValueInner::Int64(oid.map(|oid| oid as i64)));

                        Value::array(nums)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::TIMESTAMPTZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<DateTime<Utc>>> = val;
                        let dates = val.into_iter().map(ValueInner::DateTime);

                        Value::array(dates)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::DATE_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<chrono::NaiveDate>> = val;
                        let dates = val.into_iter().map(ValueInner::Date);

                        Value::array(dates)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::TIME_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<chrono::NaiveTime>> = val;
                        let times = val.into_iter().map(ValueInner::Time);

                        Value::array(times)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::TIMETZ_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<TimeTz>> = val;
                        let timetzs = val.into_iter().map(|time| ValueInner::Time(time.map(|time| time.0)));

                        Value::array(timetzs)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::JSON_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<serde_json::Value>> = val;
                        let jsons = val.into_iter().map(ValueInner::Json);

                        Value::array(jsons)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::JSONB_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<serde_json::Value>> = val;
                        let jsons = val.into_iter().map(ValueInner::Json);

                        Value::array(jsons)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::OID => match row.try_get(i)? {
                    Some(val) => {
                        let val: u32 = val;
                        Value::int64(val)
                    }
                    None => ValueInner::Int64(None),
                },
                PostgresType::CHAR => match row.try_get(i)? {
                    Some(val) => {
                        let val: i8 = val;
                        Value::character((val as u8) as char)
                    }
                    None => ValueInner::Char(None),
                },
                PostgresType::INET | PostgresType::CIDR => match row.try_get(i)? {
                    Some(val) => {
                        let val: std::net::IpAddr = val;
                        Value::text(val.to_string())
                    }
                    None => ValueInner::Text(None),
                },
                PostgresType::INET_ARRAY | PostgresType::CIDR_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<std::net::IpAddr>> = val;
                        let addrs = val
                            .into_iter()
                            .map(|ip| ValueInner::Text(ip.map(|ip| ip.to_string().into())));

                        Value::array(addrs)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::BIT | PostgresType::VARBIT => match row.try_get(i)? {
                    Some(val) => {
                        let val: BitVec = val;
                        Value::text(bits_to_string(&val)?)
                    }
                    None => ValueInner::Text(None),
                },
                PostgresType::BIT_ARRAY | PostgresType::VARBIT_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<BitVec>> = val;
                        let stringified = val
                            .into_iter()
                            .map(|bits| match bits {
                                Some(bits) => bits_to_string(&bits).map(Value::text),
                                None => Ok(ValueInner::Text(None)),
                            })
                            .collect::<crate::Result<Vec<_>>>()?;

                        Value::array(stringified)
                    }
                    None => ValueInner::Array(None),
                },
                PostgresType::XML => match row.try_get(i)? {
                    Some(val) => {
                        let val: XmlString = val;
                        Value::xml(val.0)
                    }
                    None => ValueInner::Xml(None),
                },
                PostgresType::XML_ARRAY => match row.try_get(i)? {
                    Some(val) => {
                        let val: Vec<Option<XmlString>> = val;
                        let xmls = val.into_iter().map(|xml| xml.map(|xml| xml.0));

                        Value::array(xmls)
                    }
                    None => ValueInner::Array(None),
                },
                ref x => match x.kind() {
                    Kind::Enum => match row.try_get(i)? {
                        Some(val) => {
                            let val: EnumString = val;

                            Value::enum_variant(val.value)
                        }
                        None => ValueInner::Enum(None, None),
                    },
                    Kind::Array(inner) => match inner.kind() {
                        Kind::Enum => match row.try_get(i)? {
                            Some(val) => {
                                let val: Vec<Option<EnumString>> = val;
                                let variants = val
                                    .into_iter()
                                    .map(|x| ValueInner::Enum(x.map(|x| x.value.into()), None));

                                Ok(Value::array(variants))
                            }
                            None => Ok(ValueInner::Array(None)),
                        },
                        _ => match row.try_get(i) {
                            Ok(Some(val)) => {
                                let val: Vec<Option<String>> = val;
                                let strings = val.into_iter().map(|str| ValueInner::Text(str.map(Into::into)));

                                Ok(Value::array(strings))
                            }
                            Ok(None) => Ok(ValueInner::Array(None)),
                            Err(err) => {
                                if err.source().map(|err| err.is::<WrongType>()).unwrap_or(false) {
                                    let kind = ErrorKind::UnsupportedColumnType {
                                        column_type: x.to_string(),
                                    };

                                    return Err(Error::builder(kind).build());
                                } else {
                                    Err(err)
                                }
                            }
                        },
                    }?,
                    _ => match row.try_get(i) {
                        Ok(Some(val)) => {
                            let val: String = val;

                            Ok(Value::text(val))
                        }
                        Ok(None) => Ok(ValueInner::Text(None)),
                        Err(err) => {
                            if err.source().map(|err| err.is::<WrongType>()).unwrap_or(false) {
                                let kind = ErrorKind::UnsupportedColumnType {
                                    column_type: x.to_string(),
                                };

                                return Err(Error::builder(kind).build());
                            } else {
                                Err(err)
                            }
                        }
                    }?,
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
            (ValueInner::Int32(integer), &PostgresType::INT2) => match integer {
                Some(i) => {
                    let integer = i16::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into an INT2 (16-bit signed integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
            (ValueInner::Int32(integer), &PostgresType::INT4) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueInner::Int32(integer), &PostgresType::INT8) => {
                integer.map(|integer| (integer as i64).to_sql(ty, out))
            }
            (ValueInner::Int64(integer), &PostgresType::INT2) => match integer {
                Some(i) => {
                    let integer = i16::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into an INT2 (16-bit signed integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
            (ValueInner::Int64(integer), &PostgresType::INT4) => match integer {
                Some(i) => {
                    let integer = i32::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into an INT4 (32-bit signed integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
            (ValueInner::Int64(integer), &PostgresType::INT8) => integer.map(|integer| integer.to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Int32(integer), &PostgresType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i32(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Int64(integer), &PostgresType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i64(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueInner::Int32(integer), &PostgresType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueInner::Int64(integer), &PostgresType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueInner::Int32(integer), &PostgresType::OID) => match integer {
                Some(i) => {
                    let integer = u32::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into an OID (32-bit unsigned integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
            (ValueInner::Int64(integer), &PostgresType::OID) => match integer {
                Some(i) => {
                    let integer = u32::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into an OID (32-bit unsigned integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
            (ValueInner::Int32(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueInner::Int64(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueInner::Float(float), &PostgresType::FLOAT8) => float.map(|float| (float as f64).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Float(float), &PostgresType::NUMERIC) => float
                .map(|float| BigDecimal::from_f32(float).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueInner::Float(float), _) => float.map(|float| float.to_sql(ty, out)),
            (ValueInner::Double(double), &PostgresType::FLOAT4) => double.map(|double| (double as f32).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Double(double), &PostgresType::NUMERIC) => double
                .map(|double| BigDecimal::from_f64(double).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueInner::Double(double), _) => double.map(|double| double.to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Numeric(decimal), &PostgresType::FLOAT4) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f32>().expect("decimal to f32 conversion");
                f.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Numeric(decimal), &PostgresType::FLOAT8) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f64>().expect("decimal to f64 conversion");
                f.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Array(values), &PostgresType::FLOAT4_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match value {
                        ValueInner::Numeric(n) => n.as_ref().and_then(|n| n.to_string().parse::<f32>().ok()),
                        ValueInner::Int64(n) => n.map(|n| n as f32),
                        ValueInner::Float(f) => *f,
                        ValueInner::Double(d) => d.map(|d| d as f32),
                        v if v.is_null() => None,
                        v => {
                            let kind = ErrorKind::conversion(format!(
                                "Couldn't add value of type `{v:?}` into a float array."
                            ));

                            return Err(Error::builder(kind).build().into());
                        }
                    };

                    floats.push(float);
                }

                floats.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Array(values), &PostgresType::FLOAT8_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match value {
                        ValueInner::Numeric(n) => n.as_ref().and_then(|n| n.to_string().parse::<f64>().ok()),
                        ValueInner::Int64(n) => n.map(|n| n as f64),
                        ValueInner::Float(f) => f.map(|f| f as f64),
                        ValueInner::Double(d) => *d,
                        v if v.is_null() => None,
                        v => {
                            let kind = ErrorKind::conversion(format!(
                                "Couldn't add value of type `{v:?}` into a double array."
                            ));

                            return Err(Error::builder(kind).build().into());
                        }
                    };

                    floats.push(float);
                }

                floats.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Numeric(decimal), &PostgresType::MONEY) => decimal.as_ref().map(|decimal| {
                let decimal = (decimal * BigInt::from_i32(100).unwrap()).round(0);

                let i = decimal.to_i64().ok_or_else(|| {
                    let kind = ErrorKind::conversion("Couldn't convert BigDecimal to i64.");
                    Error::builder(kind).build()
                })?;

                i.to_sql(ty, out)
            }),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Numeric(decimal), &PostgresType::NUMERIC) => decimal
                .as_ref()
                .map(|decimal| DecimalWrapper(decimal.clone()).to_sql(ty, out)),
            #[cfg(feature = "bigdecimal")]
            (ValueInner::Numeric(float), _) => float
                .as_ref()
                .map(|float| DecimalWrapper(float.clone()).to_sql(ty, out)),
            #[cfg(feature = "uuid")]
            (ValueInner::Text(string), &PostgresType::UUID) => string.as_ref().map(|string| {
                let parsed_uuid: Uuid = string.parse()?;
                parsed_uuid.to_sql(ty, out)
            }),
            #[cfg(feature = "uuid")]
            (ValueInner::Array(values), &PostgresType::UUID_ARRAY) => values.as_ref().map(|values| {
                let parsed_uuid: Vec<Option<Uuid>> = values
                    .iter()
                    .map(<Option<Uuid>>::try_from)
                    .collect::<crate::Result<Vec<_>>>()?;

                parsed_uuid.to_sql(ty, out)
            }),
            (ValueInner::Text(string), &PostgresType::INET) | (ValueInner::Text(string), &PostgresType::CIDR) => {
                string.as_ref().map(|string| {
                    let parsed_ip_addr: std::net::IpAddr = string.parse()?;
                    parsed_ip_addr.to_sql(ty, out)
                })
            }
            (ValueInner::Array(values), &PostgresType::INET_ARRAY)
            | (ValueInner::Array(values), &PostgresType::CIDR_ARRAY) => values.as_ref().map(|values| {
                let parsed_ip_addr: Vec<Option<std::net::IpAddr>> = values
                    .iter()
                    .map(<Option<std::net::IpAddr>>::try_from)
                    .collect::<crate::Result<_>>()?;

                parsed_ip_addr.to_sql(ty, out)
            }),
            (ValueInner::Text(string), &PostgresType::JSON) | (ValueInner::Text(string), &PostgresType::JSONB) => {
                string
                    .as_ref()
                    .map(|string| serde_json::from_str::<serde_json::Value>(string)?.to_sql(ty, out))
            }
            (ValueInner::Text(string), &PostgresType::BIT) | (ValueInner::Text(string), &PostgresType::VARBIT) => {
                string.as_ref().map(|string| {
                    let bits: BitVec = string_to_bits(string)?;

                    bits.to_sql(ty, out)
                })
            }
            (ValueInner::Text(string), _) => string.as_ref().map(|ref string| string.to_sql(ty, out)),
            (ValueInner::Array(values), &PostgresType::BIT_ARRAY)
            | (ValueInner::Array(values), &PostgresType::VARBIT_ARRAY) => values.as_ref().map(|values| {
                let bitvecs: Vec<Option<BitVec>> = values
                    .iter()
                    .map(<Option<BitVec>>::try_from)
                    .collect::<crate::Result<Vec<_>>>()?;

                bitvecs.to_sql(ty, out)
            }),
            (ValueInner::Bytes(bytes), _) => bytes.as_ref().map(|bytes| bytes.as_ref().to_sql(ty, out)),
            (ValueInner::Enum(string, _), _) => string.as_ref().map(|string| {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }),
            (ValueInner::Boolean(boo), _) => boo.map(|boo| boo.to_sql(ty, out)),
            (ValueInner::Char(c), _) => c.map(|c| (c as i8).to_sql(ty, out)),
            (ValueInner::Array(vec), typ) if matches!(typ.kind(), Kind::Array(_)) => {
                vec.as_ref().map(|vec| vec.to_sql(ty, out))
            }
            (ValueInner::EnumArray(variants, _), typ) if matches!(typ.kind(), Kind::Array(_)) => variants
                .as_ref()
                .map(|vec| vec.iter().map(|val| val.as_ref()).collect::<Vec<_>>().to_sql(ty, out)),
            (ValueInner::EnumArray(variants, _), typ) => {
                let kind = ErrorKind::conversion(format!(
                    "Couldn't serialize value `{variants:?}` into a `{typ}`. Value is a list but `{typ}` is not."
                ));

                return Err(Error::builder(kind).build().into());
            }
            (ValueInner::Array(vec), typ) => {
                let kind = ErrorKind::conversion(format!(
                    "Couldn't serialize value `{vec:?}` into a `{typ}`. Value is a list but `{typ}` is not."
                ));

                return Err(Error::builder(kind).build().into());
            }
            (ValueInner::Json(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (ValueInner::Xml(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            #[cfg(feature = "uuid")]
            (ValueInner::Uuid(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueInner::DateTime(value), &PostgresType::DATE) => value.map(|value| value.date_naive().to_sql(ty, out)),
            (ValueInner::Date(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueInner::Time(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueInner::DateTime(value), &PostgresType::TIME) => value.map(|value| value.time().to_sql(ty, out)),
            (ValueInner::DateTime(value), &PostgresType::TIMETZ) => value.map(|value| {
                let result = value.time().to_sql(ty, out)?;
                // We assume UTC. see https://www.postgresql.org/docs/9.5/datatype-datetime.html
                out.extend_from_slice(&[0; 4]);
                Ok(result)
            }),
            (ValueInner::DateTime(value), _) => value.map(|value| value.naive_utc().to_sql(ty, out)),
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

impl<'a> TryFrom<&Value<'a>> for Option<BitVec> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<BitVec>, Self::Error> {
        match value {
            val @ ValueInner::Text(Some(_)) => {
                let text = val.as_str().unwrap();

                string_to_bits(text).map(Option::Some)
            }
            val @ ValueInner::Bytes(Some(_)) => {
                let text = val.as_str().unwrap();

                string_to_bits(text).map(Option::Some)
            }
            v if v.is_null() => Ok(None),
            v => {
                let kind = ErrorKind::conversion(format!("Couldn't convert value of type `{v:?}` to bit_vec::BitVec."));

                Err(Error::builder(kind).build())
            }
        }
    }
}
