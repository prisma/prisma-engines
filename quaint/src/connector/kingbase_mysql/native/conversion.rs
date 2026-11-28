mod decimal;

use crate::{
    ast::{NativeColumnType, OpaqueType, TypeDataLength, Value, ValueType},
    connector::queryable::{GetRow, ToColumnNames},
    error::{Error, ErrorKind},
    prelude::EnumVariant,
};

use super::column_type::*;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive, num_bigint::BigInt};
use bit_vec::BitVec;
use bytes::BytesMut;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};

pub(crate) use decimal::DecimalWrapper;
use kingbase_postgres_types::kingbase::{MySqlBit, MySqlBitError, MySqlDate, MySqlJsonPath, MySqlTime};
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
                ValueType::Bytes(_) if p.native_column_type_name() == Some("BIT") => KingbaseType::MYSQL_SYS_BIT,
                ValueType::Bytes(_) => KingbaseType::MYSQL_LONGBLOB,
                ValueType::Boolean(_) => KingbaseType::MYSQL_TINYINT,
                ValueType::Char(_) => KingbaseType::CHAR,
                ValueType::Numeric(_) => KingbaseType::NUMERIC,
                ValueType::Json(_) => mysql_json_type(),
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

struct MysqlJson(serde_json::Value);

impl<'a> FromSql<'a> for MysqlJson {
    fn from_sql(_ty: &KingbaseType, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        // R3 mysql_json uses the same versioned JSON payload as JSONB.
        // Reuse the existing codec while accepting only its actual catalog type.
        serde_json::Value::from_sql(&KingbaseType::JSONB, raw).map(Self)
    }

    fn accepts(ty: &KingbaseType) -> bool {
        is_mysql_json(ty)
    }
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
                        let bit_width = mysql_bit_length_from_type_modifier(column.type_modifier());
                        ValueType::Bytes(v.read(val.map(|value| Cow::Owned(mysql_bit_to_bytes(&value, bit_width)))))
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
                    KingbaseColumnTypeDate::DATE => ValueType::Date(v.read(row.try_get(i)?)),
                    KingbaseColumnTypeDate::MYSQL_SYS_DATE => {
                        let date: Option<MySqlDate> = row.try_get(i)?;
                        ValueType::Date(v.read(date.map(mysql_date_to_naive).transpose()?))
                    }
                },
                KingbaseColumnType::Time(ty, v) => match ty {
                    KingbaseColumnTypeTime::TIME => ValueType::Time(v.read(row.try_get(i)?)),
                    KingbaseColumnTypeTime::MYSQL_SYS_TIME => {
                        let time: Option<MySqlTime> = row.try_get(i)?;
                        ValueType::Time(v.read(time.map(mysql_time_to_naive).transpose()?))
                    }
                    KingbaseColumnTypeTime::TIMETZ => {
                        let val: Option<TimeTz> = row.try_get(i)?;

                        ValueType::Time(v.read(val.map(|val| val.0)))
                    }
                },
                KingbaseColumnType::MysqlJson(v) => {
                    let value: Option<MysqlJson> = row.try_get(i)?;
                    ValueType::Json(v.read(value.map(|json| json.0)))
                }
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
            (ValueType::Int32(integer), &KingbaseType::MYSQL_TINYINT) => match integer {
                Some(i) => {
                    let integer = i8::try_from(*i).map_err(|_| {
                        let kind = ErrorKind::conversion(format!(
                            "Unable to fit integer value '{i}' into a TINYINT (8-bit signed integer)."
                        ));

                        Error::builder(kind).build()
                    })?;

                    Some(integer.to_sql(ty, out))
                }
                _ => None,
            },
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
            // MySQL-mode inference can describe numeric placeholders as TEXT.
            // Widen f32 before formatting so a Double result keeps its value.
            (ValueType::Float(float), &KingbaseType::TEXT) => {
                float.map(|float| f64::from(float).to_string().to_sql(ty, out))
            }
            (ValueType::Float(float), &KingbaseType::FLOAT8) => float.map(|float| (float as f64).to_sql(ty, out)),
            (ValueType::Float(float), &KingbaseType::NUMERIC) => float
                .map(|float| BigDecimal::from_f32(float).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Float(float), _) => float.map(|float| float.to_sql(ty, out)),
            (ValueType::Double(double), &KingbaseType::TEXT) => double.map(|double| double.to_string().to_sql(ty, out)),
            (ValueType::Double(double), &KingbaseType::FLOAT4) => double.map(|double| (double as f32).to_sql(ty, out)),
            (ValueType::Double(double), &KingbaseType::NUMERIC) => double
                .map(|double| BigDecimal::from_f64(double).unwrap())
                .map(DecimalWrapper)
                .map(|dw| dw.to_sql(ty, out)),
            (ValueType::Double(double), _) => double.map(|double| double.to_sql(ty, out)),
            // MySQL-mode prepared statements can describe SUM(?) as a TEXT
            // parameter. Encode the decimal text instead of a NUMERIC payload.
            (ValueType::Numeric(decimal), &KingbaseType::TEXT) => {
                decimal.as_ref().map(|decimal| decimal.to_string().to_sql(ty, out))
            }
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
            (ValueType::Text(string), ty) if is_mysql_json(ty) => string
                .as_ref()
                .map(|string| serde_json::from_str::<serde_json::Value>(string)?.to_sql(&KingbaseType::JSONB, out)),
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
            (ValueType::Json(value), ty) if is_mysql_json(ty) => {
                value.as_ref().map(|value| value.to_sql(&KingbaseType::JSONB, out))
            }
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
                let bit = mysql_bit_from_bytes(bytes, self.native_column_type.as_ref())?;
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
            (ValueType::DateTime(value), &KingbaseType::MYSQL_SYS_DATE) => {
                value.map(|value| mysql_date_from_naive(value.date_naive())?.to_sql(ty, out))
            }
            (ValueType::Date(value), &KingbaseType::MYSQL_SYS_DATE) => {
                value.map(|value| mysql_date_from_naive(value)?.to_sql(ty, out))
            }
            (ValueType::DateTime(value), &KingbaseType::DATE) => value.map(|value| value.date_naive().to_sql(ty, out)),
            (ValueType::Date(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::Time(value), &KingbaseType::MYSQL_SYS_TIME) => {
                value.map(|value| mysql_time_from_naive(value)?.to_sql(ty, out))
            }
            (ValueType::Time(value), _) => value.map(|value| value.to_sql(ty, out)),
            (ValueType::DateTime(value), &KingbaseType::MYSQL_SYS_TIME) => {
                value.map(|value| mysql_time_from_naive(value.time())?.to_sql(ty, out))
            }
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

fn mysql_date_from_naive(date: NaiveDate) -> Result<MySqlDate, Box<dyn StdError + Send + Sync>> {
    Ok(MySqlDate::from_ymd(date.year(), date.month(), date.day())?)
}

fn mysql_date_to_naive(date: MySqlDate) -> crate::Result<NaiveDate> {
    let (year, month, day) = date.year_month_day();
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        Error::builder(ErrorKind::value_out_of_range(format!(
            "Kingbase MySQL date {date} is outside the supported date range."
        )))
        .build()
    })
}

fn mysql_time_from_naive(time: NaiveTime) -> Result<MySqlTime, Box<dyn StdError + Send + Sync>> {
    Ok(MySqlTime::from_parts(
        false,
        time.hour(),
        time.minute() as u8,
        time.second() as u8,
        time.nanosecond() / 1_000,
    )?)
}

fn mysql_time_to_naive(time: MySqlTime) -> crate::Result<NaiveTime> {
    if time.is_valid_time_of_day()
        && let Some(value) = NaiveTime::from_hms_micro_opt(
            time.hours(),
            u32::from(time.minutes()),
            u32::from(time.seconds()),
            time.microseconds(),
        )
    {
        return Ok(value);
    }
    Err(Error::builder(ErrorKind::value_out_of_range(format!(
        "Kingbase MySQL TIME duration {} microseconds cannot be represented as a time of day.",
        time.micros()
    )))
    .build())
}

// Prisma/MySQL Bytes represent a big-endian integer. Keep the native BIT(n)
// width when available, otherwise preserve the input byte width. Reducing the
// width to significant bits makes R3 return a header shorter than its payload.
fn mysql_bit_from_bytes(
    bytes: &[u8],
    native_type: Option<&NativeColumnType<'_>>,
) -> Result<MySqlBit, Box<dyn StdError + Send + Sync>> {
    if bytes.len() > 8 {
        return Err(MySqlBitError::TooLong {
            bit_len: bytes.len() as u64 * 8,
        }
        .into());
    }

    let bit_width = native_type
        .filter(|native_type| native_type.name == "BIT")
        .and_then(|native_type| match native_type.length {
            Some(TypeDataLength::Constant(width)) => Some(u64::from(width)),
            _ => None,
        })
        .unwrap_or_else(|| match bytes {
            // Untyped 0/1 byte parameters can target BIT(1). Keep their
            // single-bit representation; explicit BIT(n) always wins.
            [] | [0] | [1] => 1,
            _ => bytes.len() as u64 * 8,
        });
    let value = bytes.iter().fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));

    MySqlBit::from_u64(bit_width, value).map_err(Into::into)
}

fn mysql_bit_to_bytes(value: &MySqlBit, column_bit_width: Option<u64>) -> Vec<u8> {
    let padding = (8 - value.bit_len() % 8) % 8;
    // A parameter-bound value can retain its significant bit count in R3's
    // header. MySQL fixed-width BIT results still include leading zero bytes.
    let bit_width = column_bit_width.unwrap_or(value.bit_len()).max(value.bit_len());
    let mut bytes = vec![0; bit_width.div_ceil(8) as usize];
    let offset = bytes.len() - value.payload().len();
    for (index, byte) in value.payload().iter().enumerate() {
        bytes[offset + index] = ((u16::from(*byte) >> padding)
            | (u16::from(index.checked_sub(1).map_or(0, |previous| value.payload()[previous])) << (8 - padding)))
            as u8;
    }
    bytes
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
    fn mysql_tinyint_parameters_preserve_values_and_null() {
        for integer in [-128, -1, 0, 1, 127] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                Value::int32(integer)
                    .to_sql_checked(&KingbaseType::MYSQL_TINYINT, &mut buffer)
                    .unwrap(),
                IsNull::No
            ));
            let decoded = i8::from_sql(&KingbaseType::MYSQL_TINYINT, &buffer).unwrap();
            assert_eq!(i32::from(decoded), integer);
        }

        let mut buffer = BytesMut::new();
        assert!(matches!(
            Value::null_int32()
                .to_sql_checked(&KingbaseType::MYSQL_TINYINT, &mut buffer)
                .unwrap(),
            IsNull::Yes
        ));
        assert!(buffer.is_empty());
    }

    #[test]
    fn mysql_tinyint_parameters_reject_out_of_range_values() {
        for integer in [i32::MIN, -129, 128, 255, 256, i32::MAX] {
            let mut buffer = BytesMut::new();
            let error = Value::int32(integer)
                .to_sql_checked(&KingbaseType::MYSQL_TINYINT, &mut buffer)
                .err()
                .expect("out-of-range TINYINT parameter must be rejected");
            let error = error.downcast_ref::<Error>().unwrap();
            assert!(matches!(error.kind(), ErrorKind::ConversionError(_)));
            assert!(buffer.is_empty(), "out-of-range value {integer} wrote a payload");
        }
    }

    #[test]
    fn mysql_date_parameters_use_the_driver_date_codec() {
        for date in [
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2000, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            NaiveDate::from_ymd_opt(-4713, 11, 24).unwrap(),
            NaiveDate::MAX,
        ] {
            let mut buffer = BytesMut::new();
            Value::date(date)
                .to_sql_checked(&KingbaseType::MYSQL_SYS_DATE, &mut buffer)
                .unwrap();
            let decoded = MySqlDate::from_sql(&KingbaseType::MYSQL_SYS_DATE, &buffer).unwrap();
            assert_eq!(mysql_date_to_naive(decoded).unwrap(), date);
        }
        let instant = DateTime::parse_from_rfc3339("2026-11-11T12:34:56Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut buffer = BytesMut::new();
        Value::datetime(instant)
            .to_sql_checked(&KingbaseType::MYSQL_SYS_DATE, &mut buffer)
            .unwrap();
        let decoded = MySqlDate::from_sql(&KingbaseType::MYSQL_SYS_DATE, &buffer).unwrap();
        assert_eq!(mysql_date_to_naive(decoded).unwrap(), instant.date_naive());
        buffer.clear();
        assert!(matches!(
            Value::null_date()
                .to_sql_checked(&KingbaseType::MYSQL_SYS_DATE, &mut buffer)
                .unwrap(),
            IsNull::Yes
        ));
        assert!(buffer.is_empty());
    }

    #[test]
    fn mysql_dates_outside_chrono_range_report_an_error() {
        let date = MySqlDate::from_ymd(300_000, 1, 1).unwrap();
        assert!(matches!(
            mysql_date_to_naive(date).unwrap_err().kind(),
            ErrorKind::ValueOutOfRange { .. }
        ));
        let mut buffer = BytesMut::new();
        assert!(
            Value::date(NaiveDate::MIN)
                .to_sql_checked(&KingbaseType::MYSQL_SYS_DATE, &mut buffer)
                .is_err()
        );
    }

    #[test]
    fn mysql_time_parameters_use_the_driver_duration_codec() {
        for time in [
            NaiveTime::MIN,
            NaiveTime::from_hms_micro_opt(12, 34, 56, 123_456).unwrap(),
            NaiveTime::from_hms_micro_opt(23, 59, 59, 999_999).unwrap(),
        ] {
            let mut buffer = BytesMut::new();
            Value::time(time)
                .to_sql_checked(&KingbaseType::MYSQL_SYS_TIME, &mut buffer)
                .unwrap();
            let decoded = MySqlTime::from_sql(&KingbaseType::MYSQL_SYS_TIME, &buffer).unwrap();
            assert_eq!(mysql_time_to_naive(decoded).unwrap(), time);
        }
        let instant = DateTime::parse_from_rfc3339("2026-11-11T12:34:56.123456Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut buffer = BytesMut::new();
        Value::datetime(instant)
            .to_sql_checked(&KingbaseType::MYSQL_SYS_TIME, &mut buffer)
            .unwrap();
        let decoded = MySqlTime::from_sql(&KingbaseType::MYSQL_SYS_TIME, &buffer).unwrap();
        assert_eq!(mysql_time_to_naive(decoded).unwrap(), instant.time());
        buffer.clear();
        assert!(matches!(
            Value::null_time()
                .to_sql_checked(&KingbaseType::MYSQL_SYS_TIME, &mut buffer)
                .unwrap(),
            IsNull::Yes
        ));
        assert!(buffer.is_empty());
    }

    #[test]
    fn mysql_time_durations_do_not_wrap_to_a_time_of_day() {
        for micros in [-1, 86_400_000_000, MySqlTime::MAX_MICROS] {
            assert!(matches!(
                mysql_time_to_naive(MySqlTime::new(micros).unwrap()).unwrap_err().kind(),
                ErrorKind::ValueOutOfRange { .. }
            ));
        }
    }

    #[test]
    fn mysql_bit_parameters_preserve_byte_width_without_native_metadata() {
        for (input, width, expected) in [
            (vec![], 1, 0),
            (vec![0], 1, 0),
            (vec![0, 0], 16, 0),
            (vec![1], 1, 1),
            (vec![0, 1], 16, 1),
            (vec![0x7f], 8, 0x7f),
            (vec![0xaa], 8, 0xaa),
            (vec![1, 1], 16, 0x101),
            (vec![1, 0xff], 16, 0x1ff),
            (vec![1, 1, 1], 24, 0x10101),
            (vec![0, 0, 0, 0, 0, 6, 107, 58], 64, 420666),
            (vec![0xff; 8], 64, u64::MAX),
        ] {
            let mut buffer = BytesMut::new();
            Value::bytes(input)
                .to_sql(&KingbaseType::MYSQL_SYS_BIT, &mut buffer)
                .unwrap();
            let decoded = MySqlBit::from_sql(&KingbaseType::MYSQL_SYS_BIT, &buffer).unwrap();
            assert_eq!(decoded.bit_len(), width);
            assert_eq!(decoded.to_u64(), expected);
        }
        assert!(mysql_bit_from_bytes(&[1; 9], None).is_err());
        let mut buffer = BytesMut::new();
        assert!(matches!(
            Value::null_bytes()
                .to_sql(&KingbaseType::MYSQL_SYS_BIT, &mut buffer)
                .unwrap(),
            IsNull::Yes
        ));
        assert!(buffer.is_empty());
    }

    #[test]
    fn mysql_bit_parameters_use_native_width_and_reject_overflow() {
        for width in [1, 7, 9, 17, 64] {
            for expected in [0, 1] {
                let parameter = Value::bytes(vec![expected as u8])
                    .with_native_column_type(Some(("BIT", Some(TypeDataLength::Constant(width)))));
                assert_eq!(params_to_types(&[parameter.clone()]), [KingbaseType::MYSQL_SYS_BIT]);
                let mut buffer = BytesMut::new();
                parameter.to_sql(&KingbaseType::MYSQL_SYS_BIT, &mut buffer).unwrap();
                let decoded = MySqlBit::from_sql(&KingbaseType::MYSQL_SYS_BIT, &buffer).unwrap();
                assert_eq!(decoded.bit_len(), u64::from(width));
                assert_eq!(decoded.to_u64(), expected);
            }
        }
        for (width, input) in [(7, vec![0x80]), (9, vec![2, 0]), (0, vec![0]), (65, vec![0])] {
            let parameter =
                Value::bytes(input).with_native_column_type(Some(("BIT", Some(TypeDataLength::Constant(width)))));
            assert!(
                parameter
                    .to_sql(&KingbaseType::MYSQL_SYS_BIT, &mut BytesMut::new())
                    .is_err()
            );
        }
    }

    #[test]
    fn mysql_bit_payloads_decode_to_mysql_numeric_bytes() {
        for (length, payload, expected) in [
            (7, vec![0xfe], vec![0x7f]),
            (8, vec![0xaa], vec![0xaa]),
            (9, vec![0x80, 0x80], vec![1, 1]),
            (9, vec![0, 0x80], vec![0, 1]),
            (17, vec![0x80, 0x80, 0x80], vec![1, 1, 1]),
            (64, vec![0xff; 8], vec![0xff; 8]),
        ] {
            assert_eq!(
                mysql_bit_to_bytes(&MySqlBit::new(length, payload).unwrap(), None),
                expected
            );
        }
        let significant = MySqlBit::from_u64(19, 0x066b3a).unwrap();
        assert_eq!(
            mysql_bit_to_bytes(&significant, Some(64)),
            vec![0, 0, 0, 0, 0, 6, 0x6b, 0x3a]
        );
        for value in [false, true] {
            let bit = MySqlBit::from_bool(value);
            assert_eq!(bit.as_bool(), Some(value));
        }
    }

    #[test]
    fn r3_mysql_json_parameters_and_results_use_the_existing_jsonb_codec() {
        let ty = mysql_json_type();
        assert!(<MysqlJson as FromSql>::accepts(&ty));
        for json in [
            serde_json::json!({"value": "123"}),
            serde_json::json!([1, "123", null]),
            serde_json::json!("null"),
            serde_json::json!("123"),
            serde_json::json!(123),
            serde_json::json!(u64::MAX),
            serde_json::json!(true),
            serde_json::json!(false),
            serde_json::Value::Null,
        ] {
            let parameter = Value::json(json.clone());
            assert_eq!(params_to_types(&[parameter.clone()]), [ty.clone()]);
            for value in [parameter, Value::text(json.to_string())] {
                let mut buffer = BytesMut::new();
                assert!(matches!(value.to_sql(&ty, &mut buffer).unwrap(), IsNull::No));
                assert_eq!(buffer[0], 1);
                assert_eq!(MysqlJson::from_sql(&ty, &buffer).unwrap().0, json);
            }
        }
        let mut buffer = BytesMut::new();
        assert!(matches!(
            Value::null_json().to_sql(&ty, &mut buffer).unwrap(),
            IsNull::Yes
        ));
        assert!(buffer.is_empty());
        assert_eq!(params_to_types(&[Value::null_json()]), [KingbaseType::UNKNOWN]);
        assert!(Value::text("not JSON").to_sql(&ty, &mut buffer).is_err());
    }

    #[test]
    fn r3_mysql_json_rejects_invalid_payloads_and_unrelated_types() {
        let ty = mysql_json_type();
        for raw in [&[][..], &[0, b'0'], &[2, b'0'], &[1], &[1, b'x'], &[1, 255]] {
            assert!(MysqlJson::from_sql(&ty, raw).is_err(), "{raw:?}");
        }
        for unrelated in [
            KingbaseType::JSONB,
            KingbaseType::TEXT,
            KingbaseType::new("mysql_json".into(), 4802, types::Kind::Simple, "public".into()),
            KingbaseType::new("other".into(), 4802, types::Kind::Simple, "sys".into()),
            KingbaseType::new("mysql_json".into(), 100_001, types::Kind::Simple, "sys".into()),
        ] {
            assert!(!<MysqlJson as FromSql>::accepts(&unrelated), "{unrelated}");
        }
    }

    #[test]
    fn floating_parameters_follow_text_or_binary_wire_types() {
        for float in [0.1_f32, -1.25, f32::MIN_POSITIVE, f32::MAX] {
            let value = Value::float(float);
            let mut text = BytesMut::new();
            value.to_sql(&KingbaseType::TEXT, &mut text).unwrap();
            let decoded = String::from_sql(&KingbaseType::TEXT, &text).unwrap();
            assert_eq!(decoded.parse::<f64>().unwrap(), f64::from(float));
            let mut binary = BytesMut::new();
            value.to_sql(&KingbaseType::FLOAT4, &mut binary).unwrap();
            assert_eq!(f32::from_sql(&KingbaseType::FLOAT4, &binary).unwrap(), float);
        }
        for double in [0.1_f64, -1.25, 1e-31, f64::MAX] {
            let value = Value::double(double);
            let mut text = BytesMut::new();
            value.to_sql(&KingbaseType::TEXT, &mut text).unwrap();
            let decoded = String::from_sql(&KingbaseType::TEXT, &text).unwrap();
            assert_eq!(decoded.parse::<f64>().unwrap(), double);
            let mut binary = BytesMut::new();
            value.to_sql(&KingbaseType::FLOAT8, &mut binary).unwrap();
            assert_eq!(f64::from_sql(&KingbaseType::FLOAT8, &binary).unwrap(), double);
        }
        for (value, native) in [
            (Value::null_float(), KingbaseType::FLOAT4),
            (Value::null_double(), KingbaseType::FLOAT8),
        ] {
            for ty in [KingbaseType::TEXT, native] {
                let mut buffer = BytesMut::new();
                assert!(matches!(value.to_sql(&ty, &mut buffer).unwrap(), IsNull::Yes));
                assert!(buffer.is_empty());
            }
        }
    }

    #[test]
    fn decimal_parameters_use_the_described_wire_type() {
        for value in [
            Value::numeric("1.25".parse().unwrap()),
            Value::numeric("-1.25".parse().unwrap()),
            Value::numeric("1E-31".parse().unwrap()),
            Value::numeric("123456789012345678901234567890.123456789".parse().unwrap()),
            Value::null_numeric(),
        ] {
            for ty in [KingbaseType::TEXT, KingbaseType::NUMERIC] {
                let mut buffer = BytesMut::new();
                let null = value.to_sql(&ty, &mut buffer).unwrap();

                if let Some(decimal) = value.as_numeric() {
                    assert!(matches!(null, IsNull::No));
                    if ty == KingbaseType::TEXT {
                        assert_eq!(String::from_sql(&ty, &buffer).unwrap(), decimal.to_string());
                    } else {
                        assert_eq!(DecimalWrapper::from_sql(&ty, &buffer).unwrap().0, *decimal);
                    }
                } else {
                    assert!(matches!(null, IsNull::Yes));
                    assert!(buffer.is_empty());
                }
            }
        }
    }

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
