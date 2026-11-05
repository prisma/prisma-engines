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
use kingbase_postgres_types::kingbase::{MySqlBit, MySqlJsonPath};
use kingbase_postgres_types::{FromSql, ToSql, WrongType};
use kingbase_tokio_postgres::{
    Row as KingbaseRow, Statement as KingbaseStatement,
    types::{self, IsNull, Type as KingbaseType},
};
use std::{borrow::Cow, convert::TryFrom, error::Error as StdError};

use uuid::Uuid;

fn unsupported_array() -> Error {
    let message = "Arrays are not supported in Kingbase MySQL.";
    let mut error = Error::builder(ErrorKind::conversion(message));
    error.set_original_message(message);
    error.build()
}

pub(crate) fn conv_params<'a>(params: &'a [Value<'a>]) -> crate::Result<Vec<&'a (dyn types::ToSql + Sync)>> {
    params
        .iter()
        .map(|value| match &value.typed {
            ValueType::Array(_) | ValueType::EnumArray(_, _) => Err(unsupported_array()),
            _ => Ok(value as &(dyn ToSql + Sync)),
        })
        .collect()
}

/// Maps a list of query parameter values to a list of Postgres type.
pub(crate) fn params_to_types(params: &[Value<'_>]) -> Vec<KingbaseType> {
    params
        .iter()
        .map(|p| -> KingbaseType {
            // While we can infer the underlying type of a null, Prisma can't.
            // Therefore, we let PG infer the underlying type.
            if p.is_null() {
                return KingbaseType::UNKNOWN;
            }

            if p.native_column_type_name() == Some("JSONPATH") {
                return KingbaseType::JSONPATH;
            }

            match &p.typed {
                ValueType::Int32(_) => KingbaseType::INT4,
                ValueType::Int64(_) => KingbaseType::INT8,
                ValueType::Float(_) => KingbaseType::FLOAT4,
                ValueType::Double(_) => KingbaseType::FLOAT8,
                ValueType::Text(_) => KingbaseType::TEXT,
                // Enums are user-defined types, we can't statically infer them, so we let PG infer it
                ValueType::Enum(_, _) => KingbaseType::UNKNOWN,
                ValueType::Bytes(_) => KingbaseType::MYSQL_LONGBLOB,
                ValueType::Boolean(_) => KingbaseType::MYSQL_TINYINT,
                ValueType::Char(_) => KingbaseType::CHAR,
                ValueType::Numeric(_) => KingbaseType::NUMERIC,
                ValueType::Json(_) => KingbaseType::MYSQL_SYS_JSON,
                ValueType::Xml(_) => KingbaseType::XML,
                ValueType::Uuid(_) => KingbaseType::UUID,
                ValueType::DateTime(_) => KingbaseType::MYSQL_DATETIME,
                ValueType::Date(_) => KingbaseType::MYSQL_SYS_DATE,
                ValueType::Time(_) => KingbaseType::MYSQL_SYS_TIME,

                ValueType::Array(_) | ValueType::EnumArray(_, _) => KingbaseType::UNKNOWN,

                ValueType::Opaque(opaque) => match opaque.typ() {
                    OpaqueType::Unknown => KingbaseType::UNKNOWN,
                    OpaqueType::Int32 => KingbaseType::INT4,
                    OpaqueType::Int64 => KingbaseType::INT8,
                    OpaqueType::Float => KingbaseType::FLOAT4,
                    OpaqueType::Double => KingbaseType::FLOAT8,
                    OpaqueType::Text => KingbaseType::TEXT,
                    OpaqueType::Enum => KingbaseType::UNKNOWN,
                    OpaqueType::Bytes => KingbaseType::BYTEA,
                    OpaqueType::Boolean => KingbaseType::BOOL,
                    OpaqueType::Char => KingbaseType::CHAR,
                    OpaqueType::Numeric => KingbaseType::NUMERIC,
                    OpaqueType::Json | OpaqueType::Object => KingbaseType::JSONB,
                    OpaqueType::Xml => KingbaseType::XML,
                    OpaqueType::Uuid => KingbaseType::UUID,
                    OpaqueType::DateTime => KingbaseType::TIMESTAMPTZ,
                    OpaqueType::Date => KingbaseType::TIMESTAMP,
                    OpaqueType::Time => KingbaseType::TIME,
                    OpaqueType::Array(_) => KingbaseType::UNKNOWN,
                    OpaqueType::Tuple(_) => KingbaseType::UNKNOWN,
                },
            }
        })
        .collect()
}

struct EnumString {
    value: String,
}

impl<'a> FromSql<'a> for EnumString {
    fn from_sql(_ty: &KingbaseType, raw: &'a [u8]) -> Result<EnumString, Box<dyn std::error::Error + Sync + Send>> {
        Ok(EnumString {
            value: String::from_utf8(raw.to_owned()).unwrap(),
        })
    }

    fn accepts(_ty: &KingbaseType) -> bool {
        true
    }
}

struct TimeTz(chrono::NaiveTime);

impl<'a> FromSql<'a> for TimeTz {
    fn from_sql(_ty: &KingbaseType, raw: &'a [u8]) -> Result<TimeTz, Box<dyn std::error::Error + Sync + Send>> {
        // We assume UTC.
        let time: chrono::NaiveTime = chrono::NaiveTime::from_sql(&KingbaseType::TIMETZ, &raw[..8])?;
        Ok(TimeTz(time))
    }

    fn accepts(ty: &KingbaseType) -> bool {
        ty == &KingbaseType::TIMETZ
    }
}

/// This implementation of FromSql assumes that the precision for money fields is configured to the default
/// of 2 decimals.
///
/// Postgres docs: https://www.postgresql.org/docs/current/datatype-money.html
struct NaiveMoney(BigDecimal);

impl<'a> FromSql<'a> for NaiveMoney {
    fn from_sql(_ty: &KingbaseType, raw: &'a [u8]) -> Result<NaiveMoney, Box<dyn std::error::Error + Sync + Send>> {
        let cents = i64::from_sql(&KingbaseType::INT8, raw)?;

        Ok(NaiveMoney(BigDecimal::new(BigInt::from_i64(cents).unwrap(), 2)))
    }

    fn accepts(ty: &KingbaseType) -> bool {
        ty == &KingbaseType::MONEY
    }
}

impl GetRow for KingbaseRow {
    fn get_result_row(&self) -> crate::Result<Vec<Value<'static>>> {
        fn convert(row: &KingbaseRow, i: usize) -> crate::Result<Value<'static>> {
            let column = &row.columns()[i];
            let pg_ty = column.type_();
            ensure_supported_type(pg_ty)?;

            if is_mysql_bit_one(column) {
                let val: Option<MySqlBit> = row.try_get(i)?;
                let val = val
                    .map(|value| {
                        value.as_bool().ok_or_else(|| {
                            Error::builder(ErrorKind::conversion(
                                "BIT(1) returned a value with a non-single-bit payload",
                            ))
                            .build()
                        })
                    })
                    .transpose()?;

                return Ok(ValueType::Boolean(val).into());
            }

            let column_type = KingbaseColumnType::from_pg_type(pg_ty);

            // This convoluted nested enum is macro-generated to ensure we have a single source of truth for
            // the mapping between Postgres types and ColumnType. The macro is in `./column_type.rs`.
            // KingbaseColumnValidator<Type> are used to softly ensure that the correct `ValueType` variants are created.
            // If you ever add a new type or change some mapping, please ensure you pass the data through `v.read()`.
            let result = match column_type {
                KingbaseColumnType::Boolean(ty, v) => match ty {
                    KingbaseColumnTypeBoolean::BOOL => ValueType::Boolean(v.read(row.try_get(i)?)),
                },
                KingbaseColumnType::Int32(ty, v) => match ty {
                    KingbaseColumnTypeInt32::MYSQL_TINYINT => {
                        let val: Option<i8> = row.try_get(i)?;

                        ValueType::Int32(v.read(val.map(i32::from)))
                    }
                    KingbaseColumnTypeInt32::INT2 => {
                        let val: Option<i16> = row.try_get(i)?;

                        ValueType::Int32(v.read(val.map(i32::from)))
                    }
                    KingbaseColumnTypeInt32::INT4 | KingbaseColumnTypeInt32::MYSQL_INT1 => {
                        let val: Option<i32> = row.try_get(i)?;

                        ValueType::Int32(v.read(val))
                    }
                    KingbaseColumnTypeInt32::MYSQL_YEAR => {
                        let val: Option<i32> = row.try_get(i)?;

                        ValueType::Int32(v.read(val))
                    }
                    KingbaseColumnTypeInt32::MYSQL_INT3
                    | KingbaseColumnTypeInt32::MYSQL_MEDIUMINT
                    | KingbaseColumnTypeInt32::MYSQL_MIDDLEINT => {
                        let val: Option<i32> = row.try_get(i)?;

                        ValueType::Int32(v.read(val))
                    }
                },
                KingbaseColumnType::Int64(ty, v) => match ty {
                    KingbaseColumnTypeInt64::INT8 => {
                        let val = v.read(row.try_get(i)?);

                        ValueType::Int64(val)
                    }
                    KingbaseColumnTypeInt64::OID => {
                        let val: Option<u32> = row.try_get(i)?;

                        ValueType::Int64(v.read(val.map(i64::from)))
                    }
                    KingbaseColumnTypeInt64::MYSQL_UINT4 => {
                        let val: Option<u32> = row.try_get(i)?;

                        ValueType::Int64(v.read(val.map(i64::from)))
                    }
                    KingbaseColumnTypeInt64::MYSQL_UINT8 => {
                        let val: Option<u64> = row.try_get(i)?;
                        let val = val.map(i64::try_from).transpose().map_err(|_| {
                            Error::builder(ErrorKind::value_out_of_range(
                                "Unsigned integers larger than 9_223_372_036_854_775_807 are currently not handled.",
                            ))
                            .build()
                        })?;

                        ValueType::Int64(v.read(val))
                    }
                },
                KingbaseColumnType::Float(ty, v) => match ty {
                    KingbaseColumnTypeFloat::FLOAT4 => ValueType::Float(v.read(row.try_get(i)?)),
                },
                KingbaseColumnType::Double(ty, v) => match ty {
                    KingbaseColumnTypeDouble::FLOAT8 => ValueType::Double(v.read(row.try_get(i)?)),
                },
                KingbaseColumnType::Bytes(ty, v) => match ty {
                    KingbaseColumnTypeBytes::BYTEA => {
                        let val: Option<&[u8]> = row.try_get(i)?;
                        let val = val.map(ToOwned::to_owned).map(Cow::Owned);

                        ValueType::Bytes(v.read(val))
                    }
                    KingbaseColumnTypeBytes::MYSQL_BINARY | KingbaseColumnTypeBytes::MYSQL_VARBINARY => {
                        let val: Option<&[u8]> = row.try_get(i)?;
                        ValueType::Bytes(v.read(val.map(ToOwned::to_owned).map(Cow::Owned)))
                    }
                    KingbaseColumnTypeBytes::MYSQL_SYS_BIT => {
                        let val: Option<MySqlBit> = row.try_get(i)?;
                        ValueType::Bytes(v.read(val.map(|value| Cow::Owned(value.payload().to_vec()))))
                    }
                    KingbaseColumnTypeBytes::MYSQL_BLOB
                    | KingbaseColumnTypeBytes::MYSQL_LONGBLOB
                    | KingbaseColumnTypeBytes::MYSQL_MEDIUMBLOB
                    | KingbaseColumnTypeBytes::MYSQL_TINYBLOB => {
                        let val: Option<Vec<u8>> = row.try_get(i)?;
                        ValueType::Bytes(v.read(val.map(Cow::Owned)))
                    }
                },
                KingbaseColumnType::Text(ty, v) => match ty {
                    KingbaseColumnTypeText::INET | KingbaseColumnTypeText::CIDR => {
                        let val: Option<std::net::IpAddr> = row.try_get(i)?;
                        let val = val.map(|val| val.to_string()).map(Cow::from);

                        ValueType::Text(v.read(val))
                    }
                    KingbaseColumnTypeText::VARBIT | KingbaseColumnTypeText::BIT => {
                        let val: Option<BitVec> = row.try_get(i)?;
                        let val_str = val.map(|val| bits_to_string(&val)).transpose()?.map(Cow::Owned);

                        ValueType::Text(v.read(val_str))
                    }
                    KingbaseColumnTypeText::MYSQL_LONGTEXT
                    | KingbaseColumnTypeText::MYSQL_MEDIUMTEXT
                    | KingbaseColumnTypeText::MYSQL_TINYTEXT => {
                        let val: Option<String> = row.try_get(i)?;
                        ValueType::Text(v.read(val.map(Cow::Owned)))
                    }
                    KingbaseColumnTypeText::MYSQL_BPCHARBYTE | KingbaseColumnTypeText::MYSQL_VARCHARBYTE => {
                        let val: Option<&str> = row.try_get(i)?;
                        ValueType::Text(v.read(val.map(ToOwned::to_owned).map(Cow::Owned)))
                    }
                },
                KingbaseColumnType::Char(ty, v) => match ty {
                    KingbaseColumnTypeChar::CHAR => {
                        let val: Option<i8> = row.try_get(i)?;
                        let val = val.map(|val| (val as u8) as char);

                        ValueType::Char(v.read(val))
                    }
                },
                KingbaseColumnType::Numeric(ty, v) => match ty {
                    KingbaseColumnTypeNumeric::NUMERIC => {
                        let dw: Option<DecimalWrapper> = row.try_get(i)?;
                        let val = dw.map(|dw| dw.0);

                        ValueType::Numeric(v.read(val))
                    }
                    KingbaseColumnTypeNumeric::MONEY => {
                        let val: Option<NaiveMoney> = row.try_get(i)?;

                        ValueType::Numeric(v.read(val.map(|val| val.0)))
                    }
                },
                KingbaseColumnType::DateTime(ty, v) => match ty {
                    KingbaseColumnTypeDateTime::TIMESTAMP
                    | KingbaseColumnTypeDateTime::MYSQL_DATETIME
                    | KingbaseColumnTypeDateTime::MYSQL_SYS_TIMESTAMP => {
                        let ts: Option<NaiveDateTime> = row.try_get(i)?;
                        let dt = ts.map(|ts| DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc));

                        ValueType::DateTime(v.read(dt))
                    }
                    KingbaseColumnTypeDateTime::TIMESTAMPTZ => {
                        let ts: Option<DateTime<Utc>> = row.try_get(i)?;

                        ValueType::DateTime(v.read(ts))
                    }
                },
                KingbaseColumnType::Date(ty, v) => match ty {
                    KingbaseColumnTypeDate::DATE | KingbaseColumnTypeDate::MYSQL_SYS_DATE => {
                        ValueType::Date(v.read(row.try_get(i)?))
                    }
                },
                KingbaseColumnType::Time(ty, v) => match ty {
                    KingbaseColumnTypeTime::TIME | KingbaseColumnTypeTime::MYSQL_SYS_TIME => {
                        ValueType::Time(v.read(row.try_get(i)?))
                    }
                    KingbaseColumnTypeTime::TIMETZ => {
                        let val: Option<TimeTz> = row.try_get(i)?;

                        ValueType::Time(v.read(val.map(|val| val.0)))
                    }
                },
                KingbaseColumnType::Json(ty, v) => match ty {
                    KingbaseColumnTypeJson::JSON
                    | KingbaseColumnTypeJson::JSONB
                    | KingbaseColumnTypeJson::MYSQL_SYS_JSON => ValueType::Json(v.read(row.try_get(i)?)),
                },
                KingbaseColumnType::Xml(ty, v) => match ty {
                    KingbaseColumnTypeXml::XML => {
                        let val: Option<String> = row.try_get(i)?;

                        ValueType::Xml(v.read(val.map(Cow::Owned)))
                    }
                },
                KingbaseColumnType::Uuid(ty, v) => match ty {
                    KingbaseColumnTypeUuid::UUID => ValueType::Uuid(v.read(row.try_get(i)?)),
                },
                KingbaseColumnType::Enum(v) => {
                    let val: Option<EnumString> = row.try_get(i)?;
                    let enum_variant = v.read(val.map(|x| Cow::Owned(x.value)));

                    ValueType::Enum(enum_variant.map(EnumVariant::new), None)
                }
                KingbaseColumnType::Set(v) => {
                    let val: Option<String> = row.try_get(i)?;

                    ValueType::Text(v.read(val.map(Cow::Owned)))
                }
                KingbaseColumnType::Unknown(v) => match row.try_get(i) {
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

impl ToColumnNames for KingbaseStatement {
    fn to_column_names(&self) -> Vec<String> {
        self.columns().iter().map(|c| c.name().into()).collect()
    }
}

// TODO: consider porting this logic to Driver Adapters as well
impl ToSql for Value<'_> {
    fn to_sql(
        &self,
        ty: &KingbaseType,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + 'static + Send + Sync>> {
        if matches!(&self.typed, ValueType::Array(_) | ValueType::EnumArray(_, _)) {
            return Err(unsupported_array().into());
        }

        let res = match (&self.typed, ty) {
            (ValueType::Array(_) | ValueType::EnumArray(_, _), _) => {
                unreachable!("array parameters are rejected above")
            }
            (ValueType::Int32(integer), &KingbaseType::INT2) => match integer {
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
            (ValueType::Int32(integer), &KingbaseType::MYSQL_TINYINT) => {
                integer.map(|integer| (integer as i8).to_sql(ty, out))
            }
            // Kingbase may infer a MySQL `tinyint(1)` parameter as BOOL. Encode
            // the integer value using BOOL's one-byte wire representation in
            // that case instead of sending an i32 payload.
            (ValueType::Int32(integer), &KingbaseType::BOOL) => integer.map(|integer| (integer != 0).to_sql(ty, out)),
            (ValueType::Int32(integer), &KingbaseType::MYSQL_SYS_BIT) => integer.map(|integer| match integer {
                0 => MySqlBit::from_bool(false).to_sql(ty, out),
                1 => MySqlBit::from_bool(true).to_sql(ty, out),
                _ => Err(Error::builder(ErrorKind::conversion(format!(
                    "Unable to fit integer value '{integer}' into a BIT(1)."
                )))
                .build()
                .into()),
            }),
            (ValueType::Int32(integer), &KingbaseType::INT4) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int32(integer), &KingbaseType::INT8) => integer.map(|integer| (integer as i64).to_sql(ty, out)),
            (ValueType::Int64(integer), &KingbaseType::INT2) => match integer {
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
            (ValueType::Int64(integer), &KingbaseType::INT4) => match integer {
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
            (ValueType::Int64(integer), &KingbaseType::INT8) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int32(integer), &KingbaseType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i32(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Int64(integer), &KingbaseType::NUMERIC) => integer
                .map(|integer| BigDecimal::from_i64(integer).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Int32(integer), &KingbaseType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueType::Int64(integer), &KingbaseType::TEXT) => {
                integer.map(|integer| format!("{integer}").to_sql(ty, out))
            }
            (ValueType::Int32(integer), &KingbaseType::OID) => match integer {
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
            (ValueType::Int64(integer), &KingbaseType::OID) => match integer {
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
            // Kingbase's MySQL unsigned types reject binary parameter payloads.
            // Send their decimal representation in text format so the server's
            // unsigned input function performs the range check and conversion.
            (ValueType::Int32(integer), &KingbaseType::MYSQL_UINT4 | &KingbaseType::MYSQL_UINT8) => {
                integer.map(|integer| {
                    out.extend_from_slice(integer.to_string().as_bytes());
                    Ok(IsNull::No)
                })
            }
            (ValueType::Int64(integer), &KingbaseType::MYSQL_UINT4 | &KingbaseType::MYSQL_UINT8) => {
                integer.map(|integer| {
                    out.extend_from_slice(integer.to_string().as_bytes());
                    Ok(IsNull::No)
                })
            }
            (ValueType::Int32(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Int64(integer), _) => integer.map(|integer| integer.to_sql(ty, out)),
            (ValueType::Float(float), &KingbaseType::FLOAT8) => float.map(|float| (float as f64).to_sql(ty, out)),
            (ValueType::Float(float), &KingbaseType::NUMERIC) => float
                .map(|float| BigDecimal::from_f32(float).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Float(float), _) => float.map(|float| float.to_sql(ty, out)),
            (ValueType::Double(double), &KingbaseType::FLOAT4) => double.map(|double| (double as f32).to_sql(ty, out)),
            (ValueType::Double(double), &KingbaseType::NUMERIC) => double
                .map(|double| BigDecimal::from_f64(double).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Double(double), _) => double.map(|double| double.to_sql(ty, out)),
            (ValueType::Numeric(decimal), &KingbaseType::FLOAT4) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f32>().expect("decimal to f32 conversion");
                f.to_sql(ty, out)
            }),
            (ValueType::Numeric(decimal), &KingbaseType::FLOAT8) => decimal.as_ref().map(|decimal| {
                let f = decimal.to_string().parse::<f64>().expect("decimal to f64 conversion");
                f.to_sql(ty, out)
            }),
            (ValueType::Numeric(decimal), &KingbaseType::MONEY) => decimal.as_ref().map(|decimal| {
                let decimal = (decimal * BigInt::from_i32(100).unwrap()).round(0);

                let i = decimal.to_i64().ok_or_else(|| {
                    let kind = ErrorKind::conversion("Couldn't convert BigDecimal to i64.");
                    Error::builder(kind).build()
                })?;

                i.to_sql(ty, out)
            }),
            (ValueType::Numeric(decimal), &KingbaseType::NUMERIC) => decimal
                .as_ref()
                .map(|decimal| DecimalWrapper(decimal.clone()).to_sql(ty, out)),
            (ValueType::Numeric(float), _) => float
                .as_ref()
                .map(|float| DecimalWrapper(float.clone()).to_sql(ty, out)),
            (ValueType::Text(string), &KingbaseType::UUID) => string.as_ref().map(|string| {
                let parsed_uuid: Uuid = string.parse()?;
                parsed_uuid.to_sql(ty, out)
            }),
            (ValueType::Text(string), &KingbaseType::INET) | (ValueType::Text(string), &KingbaseType::CIDR) => {
                string.as_ref().map(|string| {
                    let parsed_ip_addr: std::net::IpAddr = string.parse()?;
                    parsed_ip_addr.to_sql(ty, out)
                })
            }
            (ValueType::Text(string), &KingbaseType::JSON) | (ValueType::Text(string), &KingbaseType::JSONB) => string
                .as_ref()
                .map(|string| serde_json::from_str::<serde_json::Value>(string)?.to_sql(ty, out)),
            (ValueType::Text(string), &KingbaseType::MYSQL_SYS_JSON) => string
                .as_ref()
                .map(|string| serde_json::from_str::<serde_json::Value>(string)?.to_sql(ty, out)),
            (ValueType::Text(string), &KingbaseType::JSONPATH) => string.as_ref().map(|string| {
                // Kingbase's JSONPATH binary input starts with the protocol
                // version byte followed by the MySQL path text.
                let mut payload = Vec::with_capacity(string.len() + 1);
                payload.push(1);
                payload.extend_from_slice(string.as_bytes());
                MySqlJsonPath::new(payload).to_sql(ty, out)
            }),
            (ValueType::Json(value), &KingbaseType::MYSQL_SYS_JSON) => value
                .as_ref()
                .map(|value| <serde_json::Value as ToSql>::to_sql(value, &KingbaseType::JSONB, out)),
            (ValueType::Text(string), &KingbaseType::MYSQL_LONGTEXT)
            | (ValueType::Text(string), &KingbaseType::MYSQL_MEDIUMTEXT)
            | (ValueType::Text(string), &KingbaseType::MYSQL_TINYTEXT) => string.as_ref().map(|string| {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }),
            (ValueType::Text(string), &KingbaseType::BIT) | (ValueType::Text(string), &KingbaseType::VARBIT) => {
                string.as_ref().map(|string| {
                    let bits: BitVec = string_to_bits(string)?;

                    bits.to_sql(ty, out)
                })
            }
            (ValueType::Text(string), _) => string.as_ref().map(|ref string| string.to_sql(ty, out)),
            (ValueType::Bytes(bytes), &KingbaseType::MYSQL_BINARY)
            | (ValueType::Bytes(bytes), &KingbaseType::MYSQL_VARBINARY)
            | (ValueType::Bytes(bytes), &KingbaseType::MYSQL_BLOB)
            | (ValueType::Bytes(bytes), &KingbaseType::MYSQL_LONGBLOB)
            | (ValueType::Bytes(bytes), &KingbaseType::MYSQL_MEDIUMBLOB)
            | (ValueType::Bytes(bytes), &KingbaseType::MYSQL_TINYBLOB) => bytes.as_ref().map(|bytes| {
                out.extend_from_slice(bytes.as_ref());
                Ok(IsNull::No)
            }),
            (ValueType::Bytes(bytes), &KingbaseType::MYSQL_SYS_BIT) => bytes.as_ref().map(|bytes| {
                let bit = MySqlBit::new((bytes.len() * 8) as u64, bytes.to_vec())?;
                bit.to_sql(ty, out)
            }),
            (ValueType::Bytes(bytes), _) => bytes.as_ref().map(|bytes| bytes.as_ref().to_sql(ty, out)),
            (ValueType::Enum(string, _), _) => string.as_ref().map(|string| {
                out.extend_from_slice(string.as_bytes());
                Ok(IsNull::No)
            }),
            (ValueType::Boolean(boo), &KingbaseType::MYSQL_TINYINT) => {
                boo.map(|boo| (if boo { 1_i8 } else { 0_i8 }).to_sql(ty, out))
            }
            (ValueType::Boolean(boo), &KingbaseType::MYSQL_SYS_BIT) => {
                boo.map(|boo| MySqlBit::from_bool(boo).to_sql(ty, out))
            }
            (ValueType::Boolean(boo), _) => boo.map(|boo| boo.to_sql(ty, out)),
            (ValueType::Char(c), _) => c.map(|c| (c as i8).to_sql(ty, out)),
            (ValueType::Json(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (ValueType::Xml(value), _) => value.as_ref().map(|value| value.to_sql(ty, out)),
            (ValueType::Uuid(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::DateTime(value), &KingbaseType::DATE | &KingbaseType::MYSQL_SYS_DATE) => {
                value.map(|value| value.date_naive().to_sql(ty, out))
            }
            (ValueType::Date(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::Time(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::DateTime(value), &KingbaseType::TIME) => value.map(|value| value.time().to_sql(ty, out)),
            (ValueType::DateTime(value), &KingbaseType::TIMETZ) => value.map(|value| {
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

    fn accepts(_: &KingbaseType) -> bool {
        true // Please check later should we make this to be more restricted
    }

    fn encode_format(&self, ty: &KingbaseType) -> types::Format {
        if matches!(ty, &KingbaseType::MYSQL_UINT4 | &KingbaseType::MYSQL_UINT8) {
            types::Format::Text
        } else {
            types::Format::Binary
        }
    }

    kingbase_tokio_postgres::types::to_sql_checked!();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_array_parameters_are_rejected_including_empty_null_and_enum_arrays() {
        for value in [
            Value::array([1, 2]),
            Value::array(Vec::<i32>::new()),
            Value::null_array(),
            Value::enum_array([EnumVariant::new("A")]),
            Value::null_enum_array(),
        ] {
            let params = [value];
            assert!(
                conv_params(&params)
                    .unwrap_err()
                    .to_string()
                    .contains("Arrays are not supported")
            );
            let mut buffer = BytesMut::new();
            assert!(params[0].to_sql(&KingbaseType::INT4_ARRAY, &mut buffer).is_err());
            assert!(buffer.is_empty());
        }
        let params = [
            Value::json(serde_json::json!([1, "123", null])),
            Value::int32(1),
            Value::int32(2),
        ];
        assert_eq!(conv_params(&params).unwrap().len(), 3);
    }
}
