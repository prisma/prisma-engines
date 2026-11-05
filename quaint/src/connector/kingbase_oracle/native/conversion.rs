mod decimal;

use crate::{
    ast::{Value, ValueType},
    error::{Error, ErrorKind},
};
use bigdecimal::{BigDecimal, FromPrimitive};
use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Utc};
use kingbase_postgres_types::{FromSql, IsNull, ToSql};
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
                Some("TIMESTAMPTZ" | "TIMESTAMPLOCALTZ") => return Type::TIMESTAMPTZ,
                Some("BOOLEAN") => return Type::BOOL,
                Some("CLOB") => return Type::ORACLE_CLOB,
                Some("NCLOB") => return Type::ORACLE_NCLOB,
                Some("JSON") => return Type::JSONB,
                Some("UUID") => return Type::UUID,
                _ => {}
            }

            match &param.typed {
                ValueType::Int32(_) | ValueType::Int64(_) | ValueType::Numeric(_) | ValueType::Boolean(_) => {
                    Type::NUMERIC
                }
                ValueType::Float(_) | ValueType::Double(_) => Type::FLOAT8,
                ValueType::Text(_) => Type::VARCHAR,
                ValueType::Enum(_, _) => Type::UNKNOWN,
                ValueType::Bytes(_) => Type::ORACLE_BLOB,
                ValueType::Json(_) => Type::JSONB,
                ValueType::Xml(_) => Type::XML,
                ValueType::Uuid(_) => Type::UUID,
                ValueType::DateTime(_) => Type::TIMESTAMP,
                ValueType::Date(_) => Type::DATE,
                ValueType::Time(_) => Type::TIME,
                ValueType::Char(_) => Type::CHAR,
                ValueType::Array(_) | ValueType::EnumArray(_, _) | ValueType::Opaque(_) => Type::UNKNOWN,
            }
        })
        .collect()
}

impl ToSql for OracleParam<'_, '_> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let result = match &self.0.typed {
            ValueType::Int32(value) if ty == &Type::NUMERIC => value
                .map(BigDecimal::from)
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Int64(value) if ty == &Type::NUMERIC => value
                .map(BigDecimal::from)
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Numeric(value) => value
                .as_ref()
                .map(|value| DecimalWrapper(value.clone()).to_sql(ty, out)),
            ValueType::Boolean(value) if ty == &Type::NUMERIC => value
                .map(|value| BigDecimal::from_i32(i32::from(value)).unwrap())
                .map(DecimalWrapper)
                .map(|value| value.to_sql(ty, out)),
            ValueType::Float(value) => value.map(f64::from).map(|value| value.to_sql(ty, out)),
            ValueType::Double(value) => value.map(|value| value.to_sql(ty, out)),
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
    use super::params_to_types;
    use crate::ast::Value;
    use chrono::{TimeZone, Utc};
    use kingbase_tokio_postgres::types::Type;

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
                Type::TIMESTAMPTZ,
                Type::ORACLE_CLOB,
                Type::ORACLE_NCLOB,
                Type::JSONB,
            ]
        );
    }
}
