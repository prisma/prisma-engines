use crate::ast::Value;
#[cfg(not(feature = "bigdecimal"))]
use crate::error::*;
#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
use std::{borrow::Cow, convert::TryFrom};
use tiberius::{ColumnData, FromSql, IntoSql, ToSql};

#[tracing::instrument(skip(params))]
pub fn conv_params<'a>(params: &'a [Value<'a>]) -> crate::Result<Vec<&'a dyn ToSql>> {
    let mut converted = Vec::with_capacity(params.len());

    for param in params.iter() {
        converted.push(param as &dyn ToSql)
    }

    Ok(converted)
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> ColumnData<'_> {
        match self {
            Value::Integer(val) => val.to_sql(),
            Value::Float(val) => val.to_sql(),
            Value::Double(val) => val.to_sql(),
            Value::Text(val) => val.to_sql(),
            Value::Bytes(val) => val.to_sql(),
            Value::Enum(val) => val.to_sql(),
            Value::Boolean(val) => val.to_sql(),
            Value::Char(val) => val.as_ref().map(|val| format!("{}", val)).into_sql(),
            Value::Xml(val) => val.to_sql(),
            Value::Array(_) => panic!("Arrays are not supported on SQL Server."),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(val) => (*val).to_sql(),
            #[cfg(feature = "json")]
            Value::Json(val) => val.as_ref().map(|val| serde_json::to_string(&val).unwrap()).into_sql(),
            #[cfg(feature = "uuid")]
            Value::Uuid(val) => val.to_sql(),
            #[cfg(feature = "chrono")]
            Value::DateTime(val) => val.to_sql(),
            #[cfg(feature = "chrono")]
            Value::Date(val) => val.to_sql(),
            #[cfg(feature = "chrono")]
            Value::Time(val) => val.to_sql(),
        }
    }
}

impl TryFrom<ColumnData<'static>> for Value<'static> {
    type Error = crate::error::Error;

    fn try_from(cd: ColumnData<'static>) -> crate::Result<Self> {
        let res = match cd {
            ColumnData::U8(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I16(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I32(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I64(num) => Value::Integer(num.map(i64::from)),
            ColumnData::F32(num) => Value::Float(num),
            ColumnData::F64(num) => Value::Double(num),
            ColumnData::Bit(b) => Value::Boolean(b),
            ColumnData::String(s) => Value::Text(s),
            ColumnData::Guid(uuid) => Value::Uuid(uuid),
            ColumnData::Binary(bytes) => Value::Bytes(bytes),
            #[cfg(feature = "bigdecimal")]
            numeric @ ColumnData::Numeric(_) => Value::Numeric(BigDecimal::from_sql(&numeric)?),
            #[cfg(not(feature = "bigdecimal"))]
            _numeric @ ColumnData::Numeric(_) => {
                let kind = ErrorKind::conversion("Please enable `bigdecimal` feature to read numeric values");
                return Err(Error::builder(kind).build());
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::DateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::SmallDateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::Time(_) => {
                use tiberius::time::chrono::NaiveTime;

                Value::Time(NaiveTime::from_sql(&dt)?)
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::Date(_) => {
                use tiberius::time::chrono::NaiveDate;
                Value::Date(NaiveDate::from_sql(&dt)?)
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::DateTime2(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));

                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono")]
            dt @ ColumnData::DateTimeOffset(_) => {
                use tiberius::time::chrono::{DateTime, Utc};

                Value::DateTime(DateTime::<Utc>::from_sql(&dt)?)
            }
            ColumnData::Xml(cow) => Value::Xml(cow.map(|xml_data| Cow::Owned(xml_data.into_owned().into_string()))),
        };

        Ok(res)
    }
}
