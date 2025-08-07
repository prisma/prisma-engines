mod decimal;

use crate::{
    ast::{OpaqueType, Value, ValueType},
    connector::queryable::{GetRow, ToColumnNames},
    error::{Error, ErrorKind},
    prelude::EnumVariant,
};

use super::column_type::*;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive, num_bigint::BigInt};
use bit_vec::BitVec;
use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Utc};

pub(crate) use decimal::DecimalWrapper;
use postgres_types::{FromSql, ToSql, WrongType};
use std::{borrow::Cow, convert::TryFrom, error::Error as StdError};
use tokio_postgres::{
    Row as PostgresRow, Statement as PostgresStatement,
    types::{self, IsNull, Kind, Type as PostgresType},
};

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

            match &p.typed {
                ValueType::Int32(_) => PostgresType::INT4,
                ValueType::Int64(_) => PostgresType::INT8,
                ValueType::Float(_) => PostgresType::FLOAT4,
                ValueType::Double(_) => PostgresType::FLOAT8,
                ValueType::Text(_) => PostgresType::TEXT,
                // Enums are user-defined types, we can't statically infer them, so we let PG infer it
                ValueType::Enum(_, _) | ValueType::EnumArray(_, _) => PostgresType::UNKNOWN,
                ValueType::Bytes(_) => PostgresType::BYTEA,
                ValueType::Boolean(_) => PostgresType::BOOL,
                ValueType::Char(_) => PostgresType::CHAR,
                ValueType::Numeric(_) => PostgresType::NUMERIC,
                ValueType::Json(_) => PostgresType::JSONB,
                ValueType::Xml(_) => PostgresType::XML,
                ValueType::Uuid(_) => PostgresType::UUID,
                ValueType::DateTime(_) => PostgresType::TIMESTAMPTZ,
                ValueType::Date(_) => PostgresType::TIMESTAMP,
                ValueType::Time(_) => PostgresType::TIME,

                ValueType::Array(arr) => {
                    let arr = arr.as_ref().unwrap();

                    // If the array is empty, we can't infer the type so we let PG infer it
                    if arr.is_empty() {
                        return PostgresType::UNKNOWN;
                    }

                    let first = arr.first().unwrap();

                    // If the array does not contain the same types of values, we let PG infer the type
                    if arr
                        .iter()
                        .any(|val| std::mem::discriminant(&first.typed) != std::mem::discriminant(&val.typed))
                    {
                        return PostgresType::UNKNOWN;
                    }

                    match first.typed {
                        ValueType::Int32(_) => PostgresType::INT4_ARRAY,
                        ValueType::Int64(_) => PostgresType::INT8_ARRAY,
                        ValueType::Float(_) => PostgresType::FLOAT4_ARRAY,
                        ValueType::Double(_) => PostgresType::FLOAT8_ARRAY,
                        ValueType::Text(_) => PostgresType::TEXT_ARRAY,
                        // Enums are special types, we can't statically infer them, so we let PG infer it
                        ValueType::Enum(_, _) | ValueType::EnumArray(_, _) => PostgresType::UNKNOWN,
                        ValueType::Bytes(_) => PostgresType::BYTEA_ARRAY,
                        ValueType::Boolean(_) => PostgresType::BOOL_ARRAY,
                        ValueType::Char(_) => PostgresType::CHAR_ARRAY,
                        ValueType::Numeric(_) => PostgresType::NUMERIC_ARRAY,
                        ValueType::Json(_) => PostgresType::JSONB_ARRAY,
                        ValueType::Xml(_) => PostgresType::XML_ARRAY,
                        ValueType::Uuid(_) => PostgresType::UUID_ARRAY,
                        ValueType::DateTime(_) => PostgresType::TIMESTAMPTZ_ARRAY,
                        ValueType::Date(_) => PostgresType::TIMESTAMP_ARRAY,
                        ValueType::Time(_) => PostgresType::TIME_ARRAY,
                        // In the case of nested arrays, we let PG infer the type
                        ValueType::Array(_) => PostgresType::UNKNOWN,
                        ValueType::Opaque(_) => PostgresType::UNKNOWN,
                    }
                }

                ValueType::Opaque(opaque) => match opaque.typ() {
                    OpaqueType::Unknown => PostgresType::UNKNOWN,
                    OpaqueType::Int32 => PostgresType::INT4,
                    OpaqueType::Int64 => PostgresType::INT8,
                    OpaqueType::Float => PostgresType::FLOAT4,
                    OpaqueType::Double => PostgresType::FLOAT8,
                    OpaqueType::Text => PostgresType::TEXT,
                    OpaqueType::Enum => PostgresType::UNKNOWN,
                    OpaqueType::Bytes => PostgresType::BYTEA,
                    OpaqueType::Boolean => PostgresType::BOOL,
                    OpaqueType::Char => PostgresType::CHAR,
                    OpaqueType::Numeric => PostgresType::NUMERIC,
                    OpaqueType::Json | OpaqueType::Object => PostgresType::JSONB,
                    OpaqueType::Xml => PostgresType::XML,
                    OpaqueType::Uuid => PostgresType::UUID,
                    OpaqueType::DateTime => PostgresType::TIMESTAMPTZ,
                    OpaqueType::Date => PostgresType::TIMESTAMP,
                    OpaqueType::Time => PostgresType::TIME,
                    OpaqueType::Array(inner) => match &**inner {
                        OpaqueType::Unknown => PostgresType::UNKNOWN,
                        OpaqueType::Int32 => PostgresType::INT4_ARRAY,
                        OpaqueType::Int64 => PostgresType::INT8_ARRAY,
                        OpaqueType::Float => PostgresType::FLOAT4_ARRAY,
                        OpaqueType::Double => PostgresType::FLOAT8_ARRAY,
                        OpaqueType::Text => PostgresType::TEXT_ARRAY,
                        OpaqueType::Enum => PostgresType::TEXT_ARRAY,
                        OpaqueType::Bytes => PostgresType::BYTEA_ARRAY,
                        OpaqueType::Boolean => PostgresType::BOOL_ARRAY,
                        OpaqueType::Char => PostgresType::CHAR_ARRAY,
                        OpaqueType::Numeric => PostgresType::NUMERIC_ARRAY,
                        OpaqueType::Json | OpaqueType::Object => PostgresType::JSONB_ARRAY,
                        OpaqueType::Xml => PostgresType::XML_ARRAY,
                        OpaqueType::Uuid => PostgresType::UUID_ARRAY,
                        OpaqueType::DateTime => PostgresType::TIMESTAMPTZ_ARRAY,
                        OpaqueType::Date => PostgresType::TIMESTAMP_ARRAY,
                        OpaqueType::Time => PostgresType::TIME_ARRAY,
                        OpaqueType::Array(_) => PostgresType::UNKNOWN,
                    },
                },
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
struct NaiveMoney(BigDecimal);

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
            let pg_ty = row.columns()[i].type_();
            let column_type = PGColumnType::from_pg_type(pg_ty);

            // This convoluted nested enum is macro-generated to ensure we have a single source of truth for
            // the mapping between Postgres types and ColumnType. The macro is in `./column_type.rs`.
            // PGColumnValidator<Type> are used to softly ensure that the correct `ValueType` variants are created.
            // If you ever add a new type or change some mapping, please ensure you pass the data through `v.read()`.
            let result = match column_type {
                PGColumnType::Boolean(ty, v) => match ty {
                    PGColumnTypeBoolean::BOOL => ValueType::Boolean(v.read(row.try_get(i)?)),
                },
                PGColumnType::Int32(ty, v) => match ty {
                    PGColumnTypeInt32::INT2 => {
                        let val: Option<i16> = row.try_get(i)?;

                        ValueType::Int32(v.read(val.map(i32::from)))
                    }
                    PGColumnTypeInt32::INT4 => {
                        let val: Option<i32> = row.try_get(i)?;

                        ValueType::Int32(v.read(val))
                    }
                },
                PGColumnType::Int64(ty, v) => match ty {
                    PGColumnTypeInt64::INT8 => {
                        let val = v.read(row.try_get(i)?);

                        ValueType::Int64(val)
                    }
                    PGColumnTypeInt64::OID => {
                        let val: Option<u32> = row.try_get(i)?;

                        ValueType::Int64(v.read(val.map(i64::from)))
                    }
                },
                PGColumnType::Float(ty, v) => match ty {
                    PGColumnTypeFloat::FLOAT4 => ValueType::Float(v.read(row.try_get(i)?)),
                },
                PGColumnType::Double(ty, v) => match ty {
                    PGColumnTypeDouble::FLOAT8 => ValueType::Double(v.read(row.try_get(i)?)),
                },
                PGColumnType::Bytes(ty, v) => match ty {
                    PGColumnTypeBytes::BYTEA => {
                        let val: Option<&[u8]> = row.try_get(i)?;
                        let val = val.map(ToOwned::to_owned).map(Cow::Owned);

                        ValueType::Bytes(v.read(val))
                    }
                },
                PGColumnType::Text(ty, v) => match ty {
                    PGColumnTypeText::INET | PGColumnTypeText::CIDR => {
                        let val: Option<std::net::IpAddr> = row.try_get(i)?;
                        let val = val.map(|val| val.to_string()).map(Cow::from);

                        ValueType::Text(v.read(val))
                    }
                    PGColumnTypeText::VARBIT | PGColumnTypeText::BIT => {
                        let val: Option<BitVec> = row.try_get(i)?;
                        let val_str = val.map(|val| bits_to_string(&val)).transpose()?.map(Cow::Owned);

                        ValueType::Text(v.read(val_str))
                    }
                },
                PGColumnType::Char(ty, v) => match ty {
                    PGColumnTypeChar::CHAR => {
                        let val: Option<i8> = row.try_get(i)?;
                        let val = val.map(|val| (val as u8) as char);

                        ValueType::Char(v.read(val))
                    }
                },
                PGColumnType::Numeric(ty, v) => match ty {
                    PGColumnTypeNumeric::NUMERIC => {
                        let dw: Option<DecimalWrapper> = row.try_get(i)?;
                        let val = dw.map(|dw| dw.0);

                        ValueType::Numeric(v.read(val))
                    }
                    PGColumnTypeNumeric::MONEY => {
                        let val: Option<NaiveMoney> = row.try_get(i)?;

                        ValueType::Numeric(v.read(val.map(|val| val.0)))
                    }
                },
                PGColumnType::DateTime(ty, v) => match ty {
                    PGColumnTypeDateTime::TIMESTAMP => {
                        let ts: Option<NaiveDateTime> = row.try_get(i)?;
                        let dt = ts.map(|ts| DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc));

                        ValueType::DateTime(v.read(dt))
                    }
                    PGColumnTypeDateTime::TIMESTAMPTZ => {
                        let ts: Option<DateTime<Utc>> = row.try_get(i)?;

                        ValueType::DateTime(v.read(ts))
                    }
                },
                PGColumnType::Date(ty, v) => match ty {
                    PGColumnTypeDate::DATE => ValueType::Date(v.read(row.try_get(i)?)),
                },
                PGColumnType::Time(ty, v) => match ty {
                    PGColumnTypeTime::TIME => ValueType::Time(v.read(row.try_get(i)?)),
                    PGColumnTypeTime::TIMETZ => {
                        let val: Option<TimeTz> = row.try_get(i)?;

                        ValueType::Time(v.read(val.map(|val| val.0)))
                    }
                },
                PGColumnType::Json(ty, v) => match ty {
                    PGColumnTypeJson::JSON | PGColumnTypeJson::JSONB => ValueType::Json(v.read(row.try_get(i)?)),
                },
                PGColumnType::Xml(ty, v) => match ty {
                    PGColumnTypeXml::XML => {
                        let val: Option<XmlString> = row.try_get(i)?;

                        ValueType::Xml(v.read(val.map(|val| Cow::Owned(val.0))))
                    }
                },
                PGColumnType::Uuid(ty, v) => match ty {
                    PGColumnTypeUuid::UUID => ValueType::Uuid(v.read(row.try_get(i)?)),
                },
                PGColumnType::Int32Array(ty, v) => match ty {
                    PGColumnTypeInt32Array::INT2_ARRAY => {
                        let vals: Option<Vec<Option<i16>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let ints = vals.into_iter().map(|val| val.map(i32::from));

                                ValueType::Array(Some(
                                    v.read(ints).map(ValueType::Int32).map(ValueType::into_value).collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeInt32Array::INT4_ARRAY => {
                        let vals: Option<Vec<Option<i32>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Int32)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::Int64Array(ty, v) => match ty {
                    PGColumnTypeInt64Array::INT8_ARRAY => {
                        let vals: Option<Vec<Option<i64>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Int64)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeInt64Array::OID_ARRAY => {
                        let vals: Option<Vec<Option<u32>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let oids = vals.into_iter().map(|oid| oid.map(i64::from));

                                ValueType::Array(Some(
                                    v.read(oids).map(ValueType::Int64).map(ValueType::into_value).collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::FloatArray(ty, v) => match ty {
                    PGColumnTypeFloatArray::FLOAT4_ARRAY => {
                        let vals: Option<Vec<Option<f32>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Float)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::DoubleArray(ty, v) => match ty {
                    PGColumnTypeDoubleArray::FLOAT8_ARRAY => {
                        let vals: Option<Vec<Option<f64>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Double)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::TextArray(ty, v) => match ty {
                    PGColumnTypeTextArray::TEXT_ARRAY
                    | PGColumnTypeTextArray::NAME_ARRAY
                    | PGColumnTypeTextArray::VARCHAR_ARRAY => {
                        let vals: Option<Vec<Option<&str>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let strings = vals.into_iter().map(|s| s.map(ToOwned::to_owned).map(Cow::Owned));

                                ValueType::Array(Some(
                                    v.read(strings)
                                        .map(ValueType::Text)
                                        .map(ValueType::into_value)
                                        .collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeTextArray::INET_ARRAY | PGColumnTypeTextArray::CIDR_ARRAY => {
                        let vals: Option<Vec<Option<std::net::IpAddr>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let addrs = vals
                                    .into_iter()
                                    .map(|ip| ip.as_ref().map(ToString::to_string).map(Cow::Owned));

                                ValueType::Array(Some(
                                    v.read(addrs).map(ValueType::Text).map(ValueType::into_value).collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeTextArray::BIT_ARRAY | PGColumnTypeTextArray::VARBIT_ARRAY => {
                        let vals: Option<Vec<Option<BitVec>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let vals = vals
                                    .into_iter()
                                    .map(|bits| bits.map(|bits| bits_to_string(&bits).map(Cow::Owned)).transpose())
                                    .collect::<crate::Result<Vec<_>>>()?;

                                ValueType::Array(Some(
                                    v.read(vals.into_iter())
                                        .map(ValueType::Text)
                                        .map(ValueType::into_value)
                                        .collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeTextArray::XML_ARRAY => {
                        let vals: Option<Vec<Option<XmlString>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let xmls = vals.into_iter().map(|xml| xml.map(|xml| xml.0).map(Cow::Owned));

                                ValueType::Array(Some(
                                    v.read(xmls).map(ValueType::Text).map(ValueType::into_value).collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::BytesArray(ty, v) => match ty {
                    PGColumnTypeBytesArray::BYTEA_ARRAY => {
                        let vals: Option<Vec<Option<Vec<u8>>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(|b| b.map(Cow::Owned))
                                    .map(ValueType::Bytes)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::BooleanArray(ty, v) => match ty {
                    PGColumnTypeBooleanArray::BOOL_ARRAY => {
                        let vals: Option<Vec<Option<bool>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Boolean)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::NumericArray(ty, v) => match ty {
                    PGColumnTypeNumericArray::NUMERIC_ARRAY => {
                        let vals: Option<Vec<Option<DecimalWrapper>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let decimals = vals.into_iter().map(|dec| dec.map(|dec| dec.0));

                                ValueType::Array(Some(
                                    v.read(decimals.into_iter())
                                        .map(ValueType::Numeric)
                                        .map(ValueType::into_value)
                                        .collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                    PGColumnTypeNumericArray::MONEY_ARRAY => {
                        let vals: Option<Vec<Option<NaiveMoney>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => {
                                let nums = vals.into_iter().map(|num| num.map(|num| num.0));

                                ValueType::Array(Some(
                                    v.read(nums.into_iter())
                                        .map(ValueType::Numeric)
                                        .map(ValueType::into_value)
                                        .collect(),
                                ))
                            }
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::JsonArray(ty, v) => match ty {
                    PGColumnTypeJsonArray::JSON_ARRAY | PGColumnTypeJsonArray::JSONB_ARRAY => {
                        let vals: Option<Vec<Option<serde_json::Value>>> = row.try_get(i)?;

                        match vals {
                            Some(vals) => ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Json)
                                    .map(ValueType::into_value)
                                    .collect(),
                            )),
                            None => ValueType::Array(None),
                        }
                    }
                },
                PGColumnType::UuidArray(ty, v) => match ty {
                    PGColumnTypeUuidArray::UUID_ARRAY => match row.try_get(i)? {
                        Some(vals) => {
                            let vals: Vec<Option<Uuid>> = vals;

                            ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Uuid)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                },
                PGColumnType::DateTimeArray(ty, v) => match ty {
                    PGColumnTypeDateTimeArray::TIMESTAMP_ARRAY => match row.try_get(i)? {
                        Some(vals) => {
                            let vals: Vec<Option<NaiveDateTime>> = vals;
                            let dates = vals
                                .into_iter()
                                .map(|dt| dt.map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)));

                            ValueType::Array(Some(
                                v.read(dates)
                                    .map(ValueType::DateTime)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                    PGColumnTypeDateTimeArray::TIMESTAMPTZ_ARRAY => match row.try_get(i)? {
                        Some(vals) => {
                            let vals: Vec<Option<DateTime<Utc>>> = vals;

                            ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::DateTime)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                },
                PGColumnType::DateArray(ty, v) => match ty {
                    PGColumnTypeDateArray::DATE_ARRAY => match row.try_get(i)? {
                        Some(vals) => {
                            let vals: Vec<Option<chrono::NaiveDate>> = vals;

                            ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Date)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                },
                PGColumnType::TimeArray(ty, v) => match ty {
                    PGColumnTypeTimeArray::TIME_ARRAY => match row.try_get(i)? {
                        Some(vals) => {
                            let vals: Vec<Option<chrono::NaiveTime>> = vals;

                            ValueType::Array(Some(
                                v.read(vals.into_iter())
                                    .map(ValueType::Time)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                    PGColumnTypeTimeArray::TIMETZ_ARRAY => match row.try_get(i)? {
                        Some(val) => {
                            let val: Vec<Option<TimeTz>> = val;
                            let timetzs = val.into_iter().map(|time| time.map(|time| time.0));

                            ValueType::Array(Some(
                                v.read(timetzs.into_iter())
                                    .map(ValueType::Time)
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    },
                },
                PGColumnType::EnumArray(v) => {
                    let vals: Option<Vec<Option<EnumString>>> = row.try_get(i)?;

                    match vals {
                        Some(vals) => {
                            let enums = vals.into_iter().map(|val| val.map(|val| Cow::Owned(val.value)));

                            ValueType::Array(Some(
                                v.read(enums)
                                    .map(|variant| ValueType::Enum(variant.map(EnumVariant::new), None))
                                    .map(ValueType::into_value)
                                    .collect(),
                            ))
                        }
                        None => ValueType::Array(None),
                    }
                }
                PGColumnType::Enum(v) => {
                    let val: Option<EnumString> = row.try_get(i)?;
                    let enum_variant = v.read(val.map(|x| Cow::Owned(x.value)));

                    ValueType::Enum(enum_variant.map(EnumVariant::new), None)
                }
                PGColumnType::UnknownArray(v) => match row.try_get(i) {
                    Ok(Some(vals)) => {
                        let vals: Vec<Option<String>> = vals;
                        let strings = vals.into_iter().map(|str| str.map(Cow::Owned));

                        Ok(ValueType::Array(Some(
                            v.read(strings.into_iter())
                                .map(ValueType::Text)
                                .map(ValueType::into_value)
                                .collect(),
                        )))
                    }
                    Ok(None) => Ok(ValueType::Array(None)),
                    Err(err) => {
                        if err.source().map(|err| err.is::<WrongType>()).unwrap_or(false) {
                            let kind = ErrorKind::UnsupportedColumnType {
                                column_type: pg_ty.to_string(),
                            };

                            return Err(Error::builder(kind).build());
                        } else {
                            Err(err)
                        }
                    }
                }?,
                PGColumnType::Unknown(v) => match row.try_get(i) {
                    Ok(Some(val)) => {
                        let val: String = val;

                        Ok(ValueType::Text(v.read(Some(Cow::Owned(val)))))
                    }
                    Ok(None) => Ok(ValueType::Text(None)),
                    Err(err) => {
                        if err.source().map(|err| err.is::<WrongType>()).unwrap_or(false) {
                            let kind = ErrorKind::UnsupportedColumnType {
                                column_type: pg_ty.to_string(),
                            };

                            return Err(Error::builder(kind).build());
                        } else {
                            Err(err)
                        }
                    }
                }?,
            };

            Ok(result.into_value())
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

// TODO: consider porting this logic to Driver Adapters as well
impl ToSql for Value<'_> {
    fn to_sql(
        &self,
        ty: &PostgresType,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + 'static + Send + Sync>> {
        let res = match (&self.typed, ty) {
            (ValueType::Int32(integer), &PostgresType::INT2) => match integer {
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
            (ValueType::Int32(integer), &PostgresType::INT4) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int32(integer), &PostgresType::INT8) => integer.map(|integer| (integer as i64).to_sql(ty, out)),
            (ValueType::Int64(integer), &PostgresType::INT2) => match integer {
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
            (ValueType::Int64(integer), &PostgresType::INT4) => match integer {
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
            (ValueType::Int64(integer), &PostgresType::INT8) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int32(integer), &PostgresType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i32(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Int64(integer), &PostgresType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i64(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Int32(integer), &PostgresType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueType::Int64(integer), &PostgresType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueType::Int32(integer), &PostgresType::OID) => match integer {
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
            (ValueType::Int64(integer), &PostgresType::OID) => match integer {
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
            (ValueType::Int32(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int64(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Float(float), &PostgresType::FLOAT8) => float.map(|float| (float as f64).to_sql(ty, out)),
            (ValueType::Float(float), &PostgresType::NUMERIC) => float
                .map(|float| BigDecimal::from_f32(float).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Float(float), _) => float.map(|float| float.to_sql(ty, out)),
            (ValueType::Double(double), &PostgresType::FLOAT4) => double.map(|double| (double as f32).to_sql(ty, out)),
            (ValueType::Double(double), &PostgresType::NUMERIC) => double
                .map(|double| BigDecimal::from_f64(double).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Double(double), _) => double.map(|double| double.to_sql(ty, out)),
            (ValueType::Numeric(decimal), &PostgresType::FLOAT4) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f32>().expect("decimal to f32 conversion");
                f.to_sql(ty, out)
            }),
            (ValueType::Numeric(decimal), &PostgresType::FLOAT8) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f64>().expect("decimal to f64 conversion");
                f.to_sql(ty, out)
            }),
            (ValueType::Array(values), &PostgresType::FLOAT4_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match &value.typed {
                        ValueType::Numeric(n) => n.as_ref().and_then(|n| n.to_string().parse::<f32>().ok()),
                        ValueType::Int64(n) => n.map(|n| n as f32),
                        ValueType::Float(f) => *f,
                        ValueType::Double(d) => d.map(|d| d as f32),
                        _ if value.is_null() => None,
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
            (ValueType::Array(values), &PostgresType::FLOAT8_ARRAY) => values.as_ref().map(|values| {
                let mut floats = Vec::with_capacity(values.len());

                for value in values.iter() {
                    let float = match &value.typed {
                        ValueType::Numeric(n) => n.as_ref().and_then(|n| n.to_string().parse::<f64>().ok()),
                        ValueType::Int64(n) => n.map(|n| n as f64),
                        ValueType::Float(f) => f.map(|f| f as f64),
                        ValueType::Double(d) => *d,
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
            (ValueType::Numeric(decimal), &PostgresType::MONEY) => decimal.as_ref().map(|decimal| {
                let decimal = (decimal * BigInt::from_i32(100).unwrap()).round(0);

                let i = decimal.to_i64().ok_or_else(|| {
                    let kind = ErrorKind::conversion("Couldn't convert BigDecimal to i64.");
                    Error::builder(kind).build()
                })?;

                i.to_sql(ty, out)
            }),
            (ValueType::Numeric(decimal), &PostgresType::NUMERIC) => decimal
                .as_ref()
                .map(|decimal| DecimalWrapper(decimal.clone()).to_sql(ty, out)),
            (ValueType::Numeric(float), _) => float
                .as_ref()
                .map(|float| DecimalWrapper(float.clone()).to_sql(ty, out)),
            (ValueType::Text(string), &PostgresType::UUID) => string.as_ref().map(|string| {
                let parsed_uuid: Uuid = string.parse()?;
                parsed_uuid.to_sql(ty, out)
            }),
            (ValueType::Array(values), &PostgresType::UUID_ARRAY) => values.as_ref().map(|values| {
                let parsed_uuid: Vec<Option<Uuid>> = values
                    .iter()
                    .map(<Option<Uuid>>::try_from)
                    .collect::<crate::Result<Vec<_>>>()?;

                parsed_uuid.to_sql(ty, out)
            }),
            (ValueType::Text(string), &PostgresType::INET) | (ValueType::Text(string), &PostgresType::CIDR) => {
                string.as_ref().map(|string| {
                    let parsed_ip_addr: std::net::IpAddr = string.parse()?;
                    parsed_ip_addr.to_sql(ty, out)
                })
            }
            (ValueType::Array(values), &PostgresType::INET_ARRAY)
            | (ValueType::Array(values), &PostgresType::CIDR_ARRAY) => values.as_ref().map(|values| {
                let parsed_ip_addr: Vec<Option<std::net::IpAddr>> = values
                    .iter()
                    .map(<Option<std::net::IpAddr>>::try_from)
                    .collect::<crate::Result<_>>()?;

                parsed_ip_addr.to_sql(ty, out)
            }),
            (ValueType::Text(string), &PostgresType::JSON) | (ValueType::Text(string), &PostgresType::JSONB) => string
                .as_ref()
                .map(|string| serde_json::from_str::<serde_json::Value>(string)?.to_sql(ty, out)),
            (ValueType::Text(string), &PostgresType::BIT) | (ValueType::Text(string), &PostgresType::VARBIT) => {
                string.as_ref().map(|string| {
                    let bits: BitVec = string_to_bits(string)?;

                    bits.to_sql(ty, out)
                })
            }
            (ValueType::Text(string), _) => string.as_ref().map(|ref string| string.to_sql(ty, out)),
            (ValueType::Array(values), &PostgresType::BIT_ARRAY)
            | (ValueType::Array(values), &PostgresType::VARBIT_ARRAY) => values.as_ref().map(|values| {
                let bitvecs: Vec<Option<BitVec>> = values
                    .iter()
                    .map(|value| value.try_into())
                    .collect::<crate::Result<Vec<_>>>()?;

                bitvecs.to_sql(ty, out)
            }),
            (ValueType::Bytes(bytes), _) => bytes.as_ref().map(|bytes| bytes.as_ref().to_sql(ty, out)),
            (ValueType::Enum(string, _), _) => string.as_ref().map(|string| {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }),
            (ValueType::Boolean(boo), _) => boo.map(|boo| boo.to_sql(ty, out)),
            (ValueType::Char(c), _) => c.map(|c| (c as i8).to_sql(ty, out)),
            (ValueType::Array(vec), typ) if matches!(typ.kind(), Kind::Array(_)) => {
                vec.as_ref().map(|vec| vec.to_sql(ty, out))
            }
            (ValueType::EnumArray(variants, _), typ) if matches!(typ.kind(), Kind::Array(_)) => variants
                .as_ref()
                .map(|vec| vec.iter().map(|val| val.as_ref()).collect::<Vec<_>>().to_sql(ty, out)),
            (ValueType::EnumArray(variants, _), typ) => {
                let kind = ErrorKind::conversion(format!(
                    "Couldn't serialize value `{variants:?}` into a `{typ}`. Value is a list but `{typ}` is not."
                ));

                return Err(Error::builder(kind).build().into());
            }
            (ValueType::Array(vec), typ) => {
                let kind = ErrorKind::conversion(format!(
                    "Couldn't serialize value `{vec:?}` into a `{typ}`. Value is a list but `{typ}` is not."
                ));

                return Err(Error::builder(kind).build().into());
            }
            (ValueType::Json(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (ValueType::Xml(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (ValueType::Uuid(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::DateTime(value), &PostgresType::DATE) => value.map(|value| value.date_naive().to_sql(ty, out)),
            (ValueType::Date(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::Time(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::DateTime(value), &PostgresType::TIME) => value.map(|value| value.time().to_sql(ty, out)),
            (ValueType::DateTime(value), &PostgresType::TIMETZ) => value.map(|value| {
                let result = value.time().to_sql(ty, out)?;
                // We assume UTC. see https://www.postgresql.org/docs/9.5/datatype-datetime.html
                out.extend_from_slice(&[0; 4]);
                Ok(result)
            }),
            (ValueType::DateTime(value), _) => value.map(|value| value.naive_utc().to_sql(ty, out)),
            (ValueType::Opaque(opaque), _) => {
                let error: Box<dyn std::error::Error + Send + Sync> =
                    Box::new(Error::builder(ErrorKind::RanQueryWithOpaqueParam(opaque.to_string())).build());
                Some(Err(error))
            }
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
            val @ Value {
                typed: ValueType::Text(Some(_)),
                ..
            } => {
                let text = val.as_str().unwrap();

                string_to_bits(text).map(Option::Some)
            }
            val @ Value {
                typed: ValueType::Bytes(Some(_)),
                ..
            } => {
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
