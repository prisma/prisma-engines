mod decimal;

use crate::{
    ast::{Value, ValueType},
    error::{Error, ErrorKind},
};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Utc};
use kingbase_postgres_types::{FromSql, IsNull, ToSql, kingbase::OracleRowId};
use kingbase_tokio_postgres::{
    Row,
    types::{Kind, Type},
};
use std::borrow::Cow;

use super::{column_type::*, driver_result};

pub(crate) use decimal::DecimalWrapper;

pub(super) fn convert_row(row: &Row) -> crate::Result<Vec<Value<'static>>> {
    let mut values = Vec::with_capacity(row.columns().len());

    for (index, column) in row.columns().iter().enumerate() {
        let typ = column.type_();
        let value = match KingbaseColumnType::from_pg_type(typ) {
            KingbaseColumnType::Boolean(ty, v) => match ty {
                KingbaseColumnTypeBoolean::BOOL => ValueType::Boolean(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Int32(ty, v) => match ty {
                KingbaseColumnTypeInt32::ORACLE_TINYINT => {
                    let value: Option<OracleTinyInt> = driver_result(row.try_get(index))?;
                    ValueType::Int32(v.read(value.map(|value| i32::from(value.0))))
                }
                KingbaseColumnTypeInt32::INT2 => {
                    let value: Option<i16> = driver_result(row.try_get(index))?;
                    ValueType::Int32(v.read(value.map(i32::from)))
                }
                KingbaseColumnTypeInt32::INT4 => ValueType::Int32(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Int64(ty, v) => match ty {
                KingbaseColumnTypeInt64::INT8 => ValueType::Int64(v.read(driver_result(row.try_get(index))?)),
                KingbaseColumnTypeInt64::OID => {
                    let value: Option<u32> = driver_result(row.try_get(index))?;
                    ValueType::Int64(v.read(value.map(i64::from)))
                }
            },
            KingbaseColumnType::Float(ty, v) => match ty {
                KingbaseColumnTypeFloat::FLOAT4 => ValueType::Float(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Double(ty, v) => match ty {
                KingbaseColumnTypeDouble::FLOAT8 => ValueType::Double(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Numeric(ty, v) => match ty {
                KingbaseColumnTypeNumeric::NUMERIC => {
                    let value: Option<DecimalWrapper> = driver_result(row.try_get(index))?;
                    ValueType::Numeric(v.read(value.map(|value| value.0)))
                }
            },
            KingbaseColumnType::Text(ty, v) => match ty {
                KingbaseColumnTypeText::ORACLE_ROWID => {
                    let value: Option<OracleRowId> = driver_result(row.try_get(index))?;
                    let value = value
                        .map(|value| value.to_text())
                        .transpose()
                        .map_err(|error| Error::builder(ErrorKind::conversion(error.to_string())).build())?;
                    ValueType::Text(v.read(value.map(Cow::Owned)))
                }
                KingbaseColumnTypeText::TEXT
                | KingbaseColumnTypeText::VARCHAR
                | KingbaseColumnTypeText::BPCHAR
                | KingbaseColumnTypeText::NAME
                | KingbaseColumnTypeText::UNKNOWN
                | KingbaseColumnTypeText::ORACLE_UROWID
                | KingbaseColumnTypeText::ORACLE_CLOB
                | KingbaseColumnTypeText::ORACLE_NCLOB
                | KingbaseColumnTypeText::ORACLE_BPCHARBYTE
                | KingbaseColumnTypeText::ORACLE_VARCHARBYTE
                | KingbaseColumnTypeText::ORACLE_BFILE => {
                    let value: Option<String> = driver_result(row.try_get(index))?;
                    ValueType::Text(v.read(value.map(Cow::Owned)))
                }
            },
            KingbaseColumnType::Bytes(ty, v) => match ty {
                KingbaseColumnTypeBytes::BYTEA | KingbaseColumnTypeBytes::ORACLE_BLOB => {
                    let value: Option<Vec<u8>> = driver_result(row.try_get(index))?;
                    ValueType::Bytes(v.read(value.map(Cow::Owned)))
                }
            },
            KingbaseColumnType::Json(ty, v) => match ty {
                KingbaseColumnTypeJson::JSON | KingbaseColumnTypeJson::JSONB => {
                    ValueType::Json(v.read(driver_result(row.try_get(index))?))
                }
            },
            KingbaseColumnType::Xml(ty, v) => match ty {
                KingbaseColumnTypeXml::XML => {
                    let value: Option<String> = driver_result(row.try_get(index))?;
                    ValueType::Xml(v.read(value.map(Cow::Owned)))
                }
            },
            KingbaseColumnType::Uuid(ty, v) => match ty {
                KingbaseColumnTypeUuid::UUID => ValueType::Uuid(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Char(ty, v) => match ty {
                KingbaseColumnTypeChar::CHAR => {
                    let value: Option<i8> = driver_result(row.try_get(index))?;
                    ValueType::Char(v.read(value.map(|value| (value as u8) as char)))
                }
            },
            KingbaseColumnType::DateTime(ty, v) => match ty {
                KingbaseColumnTypeDateTime::TIMESTAMP | KingbaseColumnTypeDateTime::ORACLE_SYS_DATE => {
                    let value: Option<NaiveDateTime> = driver_result(row.try_get(index))?;
                    ValueType::DateTime(
                        v.read(value.map(|value| DateTime::<Utc>::from_naive_utc_and_offset(value, Utc))),
                    )
                }
                KingbaseColumnTypeDateTime::TIMESTAMPTZ => {
                    ValueType::DateTime(v.read(driver_result(row.try_get(index))?))
                }
            },
            KingbaseColumnType::Date(ty, v) => match ty {
                KingbaseColumnTypeDate::DATE => ValueType::Date(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::Time(ty, v) => match ty {
                KingbaseColumnTypeTime::TIME => ValueType::Time(v.read(driver_result(row.try_get(index))?)),
            },
            KingbaseColumnType::TextArray(ty, v) => match ty {
                KingbaseColumnTypeTextArray::TEXT_ARRAY
                | KingbaseColumnTypeTextArray::NAME_ARRAY
                | KingbaseColumnTypeTextArray::VARCHAR_ARRAY => {
                    let values: Option<Vec<Option<&str>>> = driver_result(row.try_get(index))?;
                    ValueType::Array(v.read(values.map(|values| {
                        values
                            .into_iter()
                            .map(|value| ValueType::Text(value.map(ToOwned::to_owned).map(Cow::Owned)).into())
                            .collect()
                    })))
                }
            },
            KingbaseColumnType::Enum(v) => {
                let value: Option<EnumString> = driver_result(row.try_get(index))?;
                let value = v.read(value.map(|value| Cow::Owned(value.0)));
                ValueType::Enum(value.map(crate::prelude::EnumVariant::new), None)
            }
            KingbaseColumnType::Unsupported(_) => {
                return Err(Error::builder(ErrorKind::UnsupportedColumnType {
                    column_type: typ.to_string(),
                })
                .build());
            }
        };

        values.push(value.into());
    }

    Ok(values)
}

/// R2 TINYINT uses the signed one-byte format already decoded by the driver's
/// i8 codec. The wrapper accepts the Oracle OID, which that codec does not list.
struct OracleTinyInt(i8);

impl<'a> FromSql<'a> for OracleTinyInt {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        i8::from_sql(ty, raw).map(Self)
    }

    fn accepts(ty: &Type) -> bool {
        ty == &Type::ORACLE_TINYINT
    }
}

/// PostgreSQL-wire enum binary values contain the UTF-8 variant label.
pub(crate) struct EnumString(pub String);

impl<'a> FromSql<'a> for EnumString {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(std::str::from_utf8(raw)?.to_owned()))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty.kind(), Kind::Enum(_))
    }
}

#[derive(Debug)]
pub(crate) struct OracleParam<'value, 'data>(&'value Value<'data>);

pub(crate) fn convert_params<'value, 'data>(params: &'value [Value<'data>]) -> Vec<OracleParam<'value, 'data>> {
    params.iter().map(OracleParam).collect()
}

pub(crate) fn as_params<'params, 'value, 'data>(
    params: &'params [OracleParam<'value, 'data>],
) -> Vec<&'params (dyn ToSql + Sync)> {
    params.iter().map(|param| param as &(dyn ToSql + Sync)).collect()
}

/// Maps Prisma values to the default Kingbase Oracle-mode wire types.
pub(crate) fn params_to_types(params: &[Value<'_>]) -> Vec<Type> {
    params
        .iter()
        .map(|param| {
            if param.is_null() {
                return Type::UNKNOWN;
            }

            // The query builder carries the native type for values originating
            // from a model field. A few Oracle-mode types need a more specific
            // PostgreSQL-wire OID than their Prisma scalar alone can convey.
            // In particular, binding a `TIMESTAMP WITH TIME ZONE` value as a
            // plain timestamp makes Kingbase interpret it in the session time
            // zone. JSON SQL/JSON functions in Kingbase operate on JSONB.
            match param.native_column_type_name() {
                Some("TIMESTAMPTZ") => return Type::TIMESTAMPTZ,
                // R2 represents LOCAL TIME ZONE columns as plain TIMESTAMP.
                // Match the decoder's UTC wall-clock convention instead of
                // converting a TIMESTAMPTZ parameter in the session time zone.
                Some("DATE" | "TIMESTAMP" | "TIMESTAMPLOCALTZ") => return Type::TIMESTAMP,
                Some("BOOLEAN") => return Type::BOOL,
                Some("CLOB") => return Type::ORACLE_CLOB,
                Some("NCLOB") => return Type::ORACLE_NCLOB,
                Some("JSON") => return Type::JSONB,
                Some("UUID") => return Type::UUID,
                Some("ROWID") => return Type::ORACLE_ROWID,
                Some("UROWID") => return Type::ORACLE_UROWID,
                Some("CHARBYTE") => return Type::ORACLE_BPCHARBYTE,
                Some("VARCHAR2BYTE") => return Type::ORACLE_VARCHARBYTE,
                Some("BINARYINTEGER") => return Type::INT4,
                _ => {}
            }

            match &param.typed {
                ValueType::Int32(_) | ValueType::Int64(_) | ValueType::Numeric(_) => Type::NUMERIC,
                ValueType::Boolean(_) => Type::BOOL,
                ValueType::Float(_) | ValueType::Double(_) => Type::FLOAT8,
                ValueType::Text(_) => Type::VARCHAR,
                ValueType::Enum(_, _) => Type::UNKNOWN,
                ValueType::Bytes(_) => Type::ORACLE_BLOB,
                ValueType::Json(_) => Type::JSONB,
                ValueType::Xml(_) => Type::XML,
                ValueType::Uuid(_) => Type::UUID,
                // Raw values have no column hint. Keep their instant explicit;
                // a zoned target must not infer a session-local wall clock.
                ValueType::DateTime(_) => Type::TIMESTAMPTZ,
                ValueType::Date(_) => Type::DATE,
                ValueType::Time(_) => Type::TIME,
                ValueType::Char(_) => Type::CHAR,
                ValueType::Array(_) | ValueType::EnumArray(_, _) | ValueType::Opaque(_) => Type::UNKNOWN,
            }
        })
        .collect()
}

fn integer_to_sql(
    value: i64,
    ty: &Type,
    out: &mut BytesMut,
) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
    let out_of_range = |_| {
        Error::builder(ErrorKind::conversion(format!(
            "Unable to fit integer value '{value}' into {}.",
            ty.name()
        )))
        .build()
    };
    match *ty {
        Type::ORACLE_TINYINT => i8::try_from(value).map_err(out_of_range)?.to_sql(ty, out),
        Type::INT2 => i16::try_from(value).map_err(out_of_range)?.to_sql(ty, out),
        Type::INT4 => i32::try_from(value).map_err(out_of_range)?.to_sql(ty, out),
        Type::INT8 => value.to_sql(ty, out),
        Type::OID => u32::try_from(value).map_err(out_of_range)?.to_sql(ty, out),
        Type::FLOAT4 => (value as f32).to_sql(ty, out),
        Type::FLOAT8 => (value as f64).to_sql(ty, out),
        _ => unreachable!("only integer/float wire types are passed here"),
    }
}

impl ToSql for OracleParam<'_, '_> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let result = match &self.0.typed {
            // Oracle-mode inference can describe numeric placeholders as TEXT.
            // Encode for the described type rather than sending numeric bytes.
            ValueType::Int32(value) if ty == &Type::TEXT => value.map(|value| value.to_string().to_sql(ty, out)),
            ValueType::Int64(value) if ty == &Type::TEXT => value.map(|value| value.to_string().to_sql(ty, out)),
            ValueType::Numeric(value) if ty == &Type::TEXT => {
                value.as_ref().map(|value| value.to_string().to_sql(ty, out))
            }
            // Float parameters otherwise use FLOAT8; preserve that widening
            // when formatting the text representation as well.
            ValueType::Float(value) if ty == &Type::TEXT => {
                value.map(|value| f64::from(value).to_string().to_sql(ty, out))
            }
            ValueType::Double(value) if ty == &Type::TEXT => value.map(|value| value.to_string().to_sql(ty, out)),
            ValueType::Int32(_) | ValueType::Int64(_)
                if matches!(
                    *ty,
                    Type::ORACLE_TINYINT
                        | Type::INT2
                        | Type::INT4
                        | Type::INT8
                        | Type::OID
                        | Type::FLOAT4
                        | Type::FLOAT8
                ) =>
            {
                let value = match &self.0.typed {
                    ValueType::Int32(value) => value.map(i64::from),
                    ValueType::Int64(value) => *value,
                    _ => unreachable!(),
                };
                value.map(|value| integer_to_sql(value, ty, out))
            }
            ValueType::Int32(value) if ty == &Type::NUMERIC => value
                .map(BigDecimal::from)
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Int64(value) if ty == &Type::NUMERIC => value
                .map(BigDecimal::from)
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Numeric(value) if ty == &Type::FLOAT4 => value.as_ref().map(|value| {
                let float = value.to_f32().filter(|value| value.is_finite()).ok_or_else(|| {
                    Error::builder(ErrorKind::conversion("Decimal value is out of range for FLOAT4")).build()
                })?;
                float.to_sql(ty, out)
            }),
            ValueType::Numeric(value) if ty == &Type::FLOAT8 => value.as_ref().map(|value| {
                let float = value.to_f64().filter(|value| value.is_finite()).ok_or_else(|| {
                    Error::builder(ErrorKind::conversion("Decimal value is out of range for FLOAT8")).build()
                })?;
                float.to_sql(ty, out)
            }),
            ValueType::Numeric(value) => value
                .as_ref()
                .map(|value| DecimalWrapper(value.clone()).to_sql(ty, out)),
            ValueType::Boolean(value) if ty == &Type::NUMERIC => value
                .map(|value| BigDecimal::from_i32(i32::from(value)).unwrap())
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Float(value) if ty == &Type::NUMERIC => value.map(|value| {
                let decimal = BigDecimal::from_f32(value).ok_or_else(|| {
                    Error::builder(ErrorKind::conversion("Non-finite FLOAT4 cannot be encoded as Decimal")).build()
                })?;
                DecimalWrapper(decimal).to_sql(ty, out)
            }),
            ValueType::Double(value) if ty == &Type::NUMERIC => value.map(|value| {
                let decimal = BigDecimal::from_f64(value).ok_or_else(|| {
                    Error::builder(ErrorKind::conversion("Non-finite FLOAT8 cannot be encoded as Decimal")).build()
                })?;
                DecimalWrapper(decimal).to_sql(ty, out)
            }),
            ValueType::Float(value) if ty == &Type::FLOAT4 => value.map(|value| value.to_sql(ty, out)),
            ValueType::Double(value) if ty == &Type::FLOAT4 => value.map(|value| {
                let float = value as f32;
                if value.is_finite() && !float.is_finite() {
                    return Err(
                        Error::builder(ErrorKind::conversion("FLOAT8 value is out of range for FLOAT4"))
                            .build()
                            .into(),
                    );
                }
                float.to_sql(ty, out)
            }),
            ValueType::Float(value) => value.map(f64::from).map(|value| value.to_sql(ty, out)),
            ValueType::Double(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::Text(value) if ty == &Type::ORACLE_ROWID => value
                .as_ref()
                .map(|value| OracleRowId::from_text(value)?.to_sql(ty, out)),
            ValueType::Text(value) if ty == &Type::UUID => value.as_ref().map(|value| {
                let uuid = uuid::Uuid::parse_str(value)
                    .map_err(|error| -> Box<dyn std::error::Error + Sync + Send> { Box::new(error) })?;
                uuid.to_sql(ty, out)
            }),
            ValueType::Text(value) => value.as_ref().map(|value| value.as_ref().to_sql(ty, out)),
            ValueType::Enum(value, _) => value.as_ref().map(|value| value.as_ref().to_sql(ty, out)),
            ValueType::Bytes(value) => value.as_ref().map(|value| value.as_ref().to_sql(ty, out)),
            ValueType::Json(value) => value.as_ref().map(|value| value.to_sql(ty, out)),
            ValueType::Xml(value) => value.as_ref().map(|value| value.as_ref().to_sql(ty, out)),
            ValueType::Uuid(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::DateTime(value) if ty == &Type::TIMESTAMPTZ => value.map(|value| value.to_sql(ty, out)),
            ValueType::DateTime(value) => value.map(|value| value.naive_utc().to_sql(ty, out)),
            ValueType::Date(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::Time(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::Char(value) => value.map(|value| value.to_string().to_sql(ty, out)),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => {
                return Err(Error::builder(ErrorKind::QueryInvalidInput(
                    "Kingbase Oracle array bind parameters are not supported.".into(),
                ))
                .build()
                .into());
            }
            ValueType::Opaque(value) => {
                return Err(Error::builder(ErrorKind::RanQueryWithOpaqueParam(value.to_string()))
                    .build()
                    .into());
            }
            ValueType::Boolean(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::Int32(value) => value.map(|value| value.to_sql(ty, out)),
            ValueType::Int64(value) => value.map(|value| value.to_sql(ty, out)),
        };

        result.unwrap_or(Ok(IsNull::Yes))
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    kingbase_postgres_types::to_sql_checked!();
}

#[cfg(test)]
mod tests {
    use super::{DecimalWrapper, OracleParam, OracleTinyInt, params_to_types};
    use crate::ast::Value;
    use bytes::BytesMut;
    use chrono::{TimeZone, Utc};
    use kingbase_postgres_types::{FromSql, IsNull, ToSql};
    use kingbase_tokio_postgres::types::Type;

    #[test]
    fn datetime_wire_types_preserve_raw_instants_and_model_wall_clock_types() {
        let instant = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();
        for (native, expected) in [
            (None, Type::TIMESTAMPTZ),
            (Some("TimestampTz"), Type::TIMESTAMPTZ),
            (Some("Date"), Type::TIMESTAMP),
            (Some("Timestamp"), Type::TIMESTAMP),
            (Some("TimestampLocalTz"), Type::TIMESTAMP),
        ] {
            let value = Value::datetime(instant).with_native_column_type(native);
            assert_eq!(params_to_types(&[value.clone()]), vec![expected.clone()], "{native:?}");
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&expected, &mut buffer).unwrap(),
                IsNull::No
            ));
            assert_eq!(
                chrono::NaiveDateTime::from_sql(&Type::TIMESTAMP, &buffer).unwrap(),
                instant.naive_utc()
            );
        }
        assert_eq!(params_to_types(&[Value::null_datetime()]), vec![Type::UNKNOWN]);
    }

    #[test]
    fn numeric_parameters_follow_the_described_binary_width() {
        for (value, ty, expected) in [
            (Value::int32(-128), Type::ORACLE_TINYINT, vec![0x80]),
            (Value::int64(127_i64), Type::ORACLE_TINYINT, vec![0x7f]),
            (Value::int32(-32768), Type::INT2, i16::MIN.to_be_bytes().to_vec()),
            (Value::int64(32767_i64), Type::INT2, i16::MAX.to_be_bytes().to_vec()),
            (
                Value::int64(i64::from(i32::MIN)),
                Type::INT4,
                i32::MIN.to_be_bytes().to_vec(),
            ),
            (
                Value::int64(i64::from(i32::MAX)),
                Type::INT4,
                i32::MAX.to_be_bytes().to_vec(),
            ),
            (Value::int32(-42), Type::INT8, (-42_i64).to_be_bytes().to_vec()),
            (Value::int64(i64::MAX), Type::INT8, i64::MAX.to_be_bytes().to_vec()),
            (
                Value::int64(i64::from(u32::MAX)),
                Type::OID,
                u32::MAX.to_be_bytes().to_vec(),
            ),
            (Value::int32(42), Type::FLOAT4, 42_f32.to_be_bytes().to_vec()),
            (Value::int64(42_i64), Type::FLOAT8, 42_f64.to_be_bytes().to_vec()),
            (
                Value::numeric("0.125".parse().unwrap()),
                Type::FLOAT4,
                0.125_f32.to_be_bytes().to_vec(),
            ),
            (
                Value::numeric("0.125".parse().unwrap()),
                Type::FLOAT8,
                0.125_f64.to_be_bytes().to_vec(),
            ),
            (Value::float(0.125), Type::FLOAT4, 0.125_f32.to_be_bytes().to_vec()),
            (Value::double(0.125), Type::FLOAT4, 0.125_f32.to_be_bytes().to_vec()),
            (
                Value::float(0.1),
                Type::FLOAT8,
                f64::from(0.1_f32).to_be_bytes().to_vec(),
            ),
        ] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&ty, &mut buffer).unwrap(),
                IsNull::No
            ));
            assert_eq!(buffer.as_ref(), expected, "{value:?} / {ty}");
        }
        for (value, ty) in [
            (Value::int32(-129), Type::ORACLE_TINYINT),
            (Value::int64(128_i64), Type::ORACLE_TINYINT),
            (Value::int32(32768), Type::INT2),
            (Value::int64(-32769_i64), Type::INT2),
            (Value::int64(i64::from(i32::MAX) + 1), Type::INT4),
            (Value::int64(i64::from(i32::MIN) - 1), Type::INT4),
            (Value::int64(-1_i64), Type::OID),
            (Value::int64(i64::from(u32::MAX) + 1), Type::OID),
            (Value::double(f64::MAX), Type::FLOAT4),
            (Value::numeric("1e100".parse().unwrap()), Type::FLOAT4),
            (Value::numeric("1e1000".parse().unwrap()), Type::FLOAT8),
        ] {
            let mut buffer = BytesMut::new();
            assert!(
                OracleParam(&value).to_sql(&ty, &mut buffer).is_err(),
                "{value:?} / {ty}"
            );
            assert!(buffer.is_empty());
        }
        for value in [Value::null_int32(), Value::null_int64()] {
            for ty in [Type::ORACLE_TINYINT, Type::INT2, Type::INT4, Type::INT8] {
                let mut buffer = BytesMut::new();
                assert!(matches!(
                    OracleParam(&value).to_sql(&ty, &mut buffer).unwrap(),
                    IsNull::Yes
                ));
                assert!(buffer.is_empty());
            }
        }
        for value in [Value::null_float(), Value::null_double()] {
            for ty in [Type::FLOAT4, Type::FLOAT8] {
                let mut buffer = BytesMut::new();
                assert!(matches!(
                    OracleParam(&value).to_sql(&ty, &mut buffer).unwrap(),
                    IsNull::Yes
                ));
                assert!(buffer.is_empty());
            }
        }
    }

    #[test]
    fn floating_parameters_use_numeric_encoding_for_numeric_targets() {
        for value in [Value::float(0.125), Value::double(0.125)] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&Type::NUMERIC, &mut buffer).unwrap(),
                IsNull::No
            ));
            assert_eq!(
                DecimalWrapper::from_sql(&Type::NUMERIC, &buffer).unwrap().0,
                "0.125".parse().unwrap()
            );
        }
        for value in [Value::float(f32::INFINITY), Value::double(f64::NAN)] {
            let mut buffer = BytesMut::new();
            assert!(OracleParam(&value).to_sql(&Type::NUMERIC, &mut buffer).is_err());
            assert!(buffer.is_empty());
        }
        for value in [Value::null_float(), Value::null_double(), Value::null_numeric()] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&Type::NUMERIC, &mut buffer).unwrap(),
                IsNull::Yes
            ));
            assert!(buffer.is_empty());
        }
    }

    #[test]
    fn boolean_parameters_default_to_the_boolean_wire_type() {
        assert_eq!(
            params_to_types(&[Value::boolean(true), Value::boolean(false)]),
            vec![Type::BOOL, Type::BOOL]
        );
        assert_eq!(params_to_types(&[Value::null_boolean()]), vec![Type::UNKNOWN]);
    }

    #[test]
    fn oracle_tinyint_uses_signed_byte_decoding_and_rejects_invalid_payloads() {
        for (raw, expected) in [(0x80, -128), (0xff, -1), (0x00, 0), (0x01, 1), (0x7f, 127)] {
            assert_eq!(
                OracleTinyInt::from_sql(&Type::ORACLE_TINYINT, &[raw]).unwrap().0,
                expected
            );
        }
        for raw in [&[][..], &[0, 1][..]] {
            assert!(OracleTinyInt::from_sql(&Type::ORACLE_TINYINT, raw).is_err());
        }
        assert!(OracleTinyInt::accepts(&Type::ORACLE_TINYINT));
        for ty in [Type::MYSQL_TINYINT, Type::CHAR, Type::INT2, Type::INT4, Type::TEXT] {
            assert!(!OracleTinyInt::accepts(&ty));
        }
    }

    #[test]
    fn numeric_parameters_use_text_when_described_as_text() {
        for (value, expected) in [
            (Value::int32(i32::MIN), i32::MIN.to_string()),
            (Value::int64(i64::MIN), i64::MIN.to_string()),
            (Value::int64(i64::MAX), i64::MAX.to_string()),
            (Value::float(0.1), f64::from(0.1_f32).to_string()),
            (Value::float(f32::MAX), f64::from(f32::MAX).to_string()),
            (Value::double(-1.25), "-1.25".to_owned()),
            (Value::double(f64::MAX), f64::MAX.to_string()),
            (Value::double(1e-31), 1e-31_f64.to_string()),
            (
                Value::numeric("12345678901234567890.123456789".parse().unwrap()),
                "12345678901234567890.123456789".to_owned(),
            ),
            (Value::numeric("-1.25".parse().unwrap()), "-1.25".to_owned()),
            (
                Value::numeric("1e-31".parse().unwrap()),
                "1e-31".parse::<bigdecimal::BigDecimal>().unwrap().to_string(),
            ),
        ] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&Type::TEXT, &mut buffer).unwrap(),
                IsNull::No
            ));
            assert_eq!(String::from_sql(&Type::TEXT, &buffer).unwrap(), expected);
        }

        for value in [
            Value::null_int32(),
            Value::null_int64(),
            Value::null_numeric(),
            Value::null_float(),
            Value::null_double(),
        ] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                OracleParam(&value).to_sql(&Type::TEXT, &mut buffer).unwrap(),
                IsNull::Yes
            ));
            assert!(buffer.is_empty());
        }
    }

    #[test]
    fn uses_oracle_native_wire_types_when_scalar_types_are_ambiguous() {
        let params = [
            Value::boolean(true).with_native_column_type(Some("Boolean")),
            Value::text("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").with_native_column_type(Some("Uuid")),
            Value::datetime(Utc.with_ymd_and_hms(2025, 1, 2, 3, 4, 5).unwrap())
                .with_native_column_type(Some("TimestampTz")),
            Value::datetime(Utc.with_ymd_and_hms(2025, 1, 2, 3, 4, 5).unwrap())
                .with_native_column_type(Some("TimestampLocalTz")),
            Value::text("large text").with_native_column_type(Some("Clob")),
            Value::text("national text").with_native_column_type(Some("NClob")),
            Value::json(serde_json::json!({ "value": 1 })),
        ];

        assert_eq!(
            params_to_types(&params),
            vec![
                Type::BOOL,
                Type::UUID,
                Type::TIMESTAMPTZ,
                Type::TIMESTAMP,
                Type::ORACLE_CLOB,
                Type::ORACLE_NCLOB,
                Type::JSONB,
            ]
        );
    }
}
