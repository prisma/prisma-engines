use crate::ast::{Value, ValueType};

use bigdecimal::BigDecimal;
use std::{borrow::Cow, convert::TryFrom};

use tiberius::ToSql;
use tiberius::{ColumnData, FromSql, IntoSql};

impl<'a> IntoSql<'a> for &'a Value<'a> {
    fn into_sql(self) -> ColumnData<'a> {
        match &self.typed {
            ValueType::Int32(val) => val.into_sql(),
            ValueType::Int64(val) => val.into_sql(),
            ValueType::Float(val) => val.into_sql(),
            ValueType::Double(val) => val.into_sql(),
            ValueType::Text(val) => val.as_deref().into_sql(),
            ValueType::Bytes(val) => val.as_deref().into_sql(),
            ValueType::Enum(val, _) => val.as_deref().into_sql(),
            ValueType::Boolean(val) => val.into_sql(),
            ValueType::Char(val) => val.as_ref().map(|val| format!("{val}")).into_sql(),
            ValueType::Xml(val) => val.as_deref().into_sql(),
            ValueType::Array(_) | ValueType::EnumArray(_, _) => panic!("Arrays are not supported on SQL Server."),
            ValueType::Numeric(val) => (*val).to_sql(),
            ValueType::Json(val) => val.as_ref().map(|val| serde_json::to_string(&val).unwrap()).into_sql(),
            ValueType::Uuid(val) => val.into_sql(),
            ValueType::DateTime(val) => val.into_sql(),
            ValueType::Date(val) => val.into_sql(),
            ValueType::Time(val) => val.into_sql(),
        }
    }
}

impl TryFrom<ColumnData<'static>> for Value<'static> {
    type Error = crate::error::Error;

    fn try_from(cd: ColumnData<'static>) -> crate::Result<Self> {
        let res = match cd {
            ColumnData::U8(num) => ValueType::Int32(num.map(i32::from)),
            ColumnData::I16(num) => ValueType::Int32(num.map(i32::from)),
            ColumnData::I32(num) => ValueType::Int32(num.map(i32::from)),
            ColumnData::I64(num) => ValueType::Int64(num.map(i64::from)),
            ColumnData::F32(num) => ValueType::Float(num),
            ColumnData::F64(num) => ValueType::Double(num),
            ColumnData::Bit(b) => ValueType::Boolean(b),
            ColumnData::String(s) => ValueType::Text(s),
            ColumnData::Guid(uuid) => ValueType::Uuid(uuid),
            ColumnData::Binary(bytes) => ValueType::Bytes(bytes),
            numeric @ ColumnData::Numeric(_) => ValueType::Numeric(BigDecimal::from_sql(&numeric)?),
            dt @ ColumnData::DateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                ValueType::DateTime(dt)
            }
            dt @ ColumnData::SmallDateTime(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                ValueType::DateTime(dt)
            }
            dt @ ColumnData::Time(_) => {
                use tiberius::time::chrono::NaiveTime;

                ValueType::Time(NaiveTime::from_sql(&dt)?)
            }
            dt @ ColumnData::Date(_) => {
                use tiberius::time::chrono::NaiveDate;
                ValueType::Date(NaiveDate::from_sql(&dt)?)
            }
            dt @ ColumnData::DateTime2(_) => {
                use tiberius::time::chrono::{DateTime, NaiveDateTime, Utc};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));

                ValueType::DateTime(dt)
            }
            dt @ ColumnData::DateTimeOffset(_) => {
                use tiberius::time::chrono::{DateTime, Utc};

                ValueType::DateTime(DateTime::<Utc>::from_sql(&dt)?)
            }
            ColumnData::Xml(cow) => ValueType::Xml(cow.map(|xml_data| Cow::Owned(xml_data.into_owned().into_string()))),
        };

        Ok(Value::from(res))
    }
}
