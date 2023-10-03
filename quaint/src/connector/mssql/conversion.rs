use crate::ast::{Value, ValueInner};
#[cfg(not(feature = "bigdecimal"))]
use crate::error::*;
#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
use std::{borrow::Cow, convert::TryFrom};
#[cfg(feature = "bigdecimal")]
use tiberius::ToSql;
use tiberius::{ColumnData, FromSql, IntoSql};

impl<'a> IntoSql<'a> for &'a Value<'a> {
    fn into_sql(self) -> ColumnData<'a> {
        match &self.inner {
            ValueInner::Int32(val) => val.into_sql(),
            ValueInner::Int64(val) => val.into_sql(),
            ValueInner::Float(val) => val.into_sql(),
            ValueInner::Double(val) => val.into_sql(),
            ValueInner::Text(val) => val.as_deref().into_sql(),
            ValueInner::Bytes(val) => val.as_deref().into_sql(),
            ValueInner::Enum(val, _) => val.as_deref().into_sql(),
            ValueInner::Boolean(val) => val.into_sql(),
            ValueInner::Char(val) => val.as_ref().map(|val| format!("{val}")).into_sql(),
            ValueInner::Xml(val) => val.as_deref().into_sql(),
            ValueInner::Array(_) | ValueInner::EnumArray(_, _) => panic!("Arrays are not supported on SQL Server."),
            #[cfg(feature = "bigdecimal")]
            ValueInner::Numeric(val) => (*val).to_sql(),
            ValueInner::Json(val) => val.as_ref().map(|val| serde_json::to_string(&val).unwrap()).into_sql(),
            #[cfg(feature = "uuid")]
            ValueInner::Uuid(val) => val.into_sql(),
            ValueInner::DateTime(val) => val.into_sql(),
            ValueInner::Date(val) => val.into_sql(),
            ValueInner::Time(val) => val.into_sql(),
        }
    }
}

impl TryFrom<ColumnData<'static>> for Value<'static> {
    type Error = crate::error::Error;

    fn try_from(cd: ColumnData<'static>) -> crate::Result<Self> {
        let res = match cd {
            ColumnData::U8(num) => ValueInner::Int32(num.map(i32::from)),
            ColumnData::I16(num) => ValueInner::Int32(num.map(i32::from)),
            ColumnData::I32(num) => ValueInner::Int32(num.map(i32::from)),
            ColumnData::I64(num) => ValueInner::Int64(num.map(i64::from)),
            ColumnData::F32(num) => ValueInner::Float(num),
            ColumnData::F64(num) => ValueInner::Double(num),
            ColumnData::Bit(b) => ValueInner::Boolean(b),
            ColumnData::String(s) => ValueInner::Text(s),
            ColumnData::Guid(uuid) => ValueInner::Uuid(uuid),
            ColumnData::Binary(bytes) => ValueInner::Bytes(bytes),
            #[cfg(feature = "bigdecimal")]
            numeric @ ColumnData::Numeric(_) => ValueInner::Numeric(BigDecimal::from_sql(&numeric)?),
            #[cfg(not(feature = "bigdecimal"))]
            _numeric @ ColumnData::Numeric(_) => {
                let kind = ErrorKind::conversion("Please enable `bigdecimal` feature to read numeric values");
                return Err(Error::builder(kind).build());
            }
            dt @ ColumnData::DateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                ValueInner::DateTime(dt)
            }
            dt @ ColumnData::SmallDateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                ValueInner::DateTime(dt)
            }
            dt @ ColumnData::Time(_) => {
                use tiberius::time::chrono::NaiveTime;

                ValueInner::Time(NaiveTime::from_sql(&dt)?)
            }
            dt @ ColumnData::Date(_) => {
                use tiberius::time::chrono::NaiveDate;
                ValueInner::Date(NaiveDate::from_sql(&dt)?)
            }
            dt @ ColumnData::DateTime2(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));

                ValueInner::DateTime(dt)
            }
            dt @ ColumnData::DateTimeOffset(_) => {
                use tiberius::time::chrono::{DateTime, Utc};

                ValueInner::DateTime(DateTime::<Utc>::from_sql(&dt)?)
            }
            ColumnData::Xml(cow) => {
                ValueInner::Xml(cow.map(|xml_data| Cow::Owned(xml_data.into_owned().into_string())))
            }
        };

        Ok(Value::from(res))
    }
}
