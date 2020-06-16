use crate::ast::Value;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::convert::TryFrom;
use tiberius::{ColumnData, FromSql, IntoSql, ToSql};

pub fn conv_params<'a>(params: &'a [Value<'a>]) -> Vec<&'a dyn ToSql> {
    params.iter().map(|x| x as &dyn ToSql).collect::<Vec<_>>()
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> ColumnData<'_> {
        match self {
            Value::Integer(val) => val.to_sql(),
            Value::Real(val) => val.to_sql(),
            Value::Text(val) => val.to_sql(),
            Value::Bytes(val) => val.to_sql(),
            Value::Enum(val) => val.to_sql(),
            Value::Boolean(val) => val.to_sql(),
            Value::Char(val) => val.as_ref().map(|val| format!("{}", val)).into_sql(),
            #[cfg(feature = "array")]
            Value::Array(_) => panic!("Arrays not supported in MSSQL"),
            #[cfg(feature = "json-1")]
            Value::Json(val) => val.as_ref().map(|val| serde_json::to_string(&val).unwrap()).into_sql(),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(val) => val.to_sql(),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(val) => val.to_sql(),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(val) => val.to_sql(),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(val) => val.to_sql(),
        }
    }
}

impl TryFrom<ColumnData<'static>> for Value<'static> {
    type Error = crate::error::Error;

    fn try_from(cd: ColumnData<'static>) -> crate::Result<Self> {
        let res = match cd {
            ColumnData::I8(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I16(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I32(num) => Value::Integer(num.map(i64::from)),
            ColumnData::I64(num) => Value::Integer(num.map(i64::from)),
            ColumnData::F32(num) => Value::Real(num.and_then(Decimal::from_f32)),
            ColumnData::F64(num) => Value::Real(num.and_then(Decimal::from_f64)),
            ColumnData::Bit(b) => Value::Boolean(b),
            ColumnData::String(s) => Value::Text(s),
            ColumnData::Guid(uuid) => Value::Uuid(uuid),
            ColumnData::Binary(bytes) => Value::Bytes(bytes),
            numeric @ ColumnData::Numeric(_) => Value::Real(Decimal::from_sql(&numeric)?),
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::DateTime(_) => {
                use chrono::{offset::Utc, DateTime, NaiveDateTime};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::SmallDateTime(_) => {
                use chrono::{offset::Utc, DateTime, NaiveDateTime};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::Time(_) => {
                use chrono::NaiveTime;
                Value::Time(NaiveTime::from_sql(&dt)?)
            }
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::Date(_) => {
                use chrono::NaiveDate;
                Value::Date(NaiveDate::from_sql(&dt)?)
            }
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::DateTime2(_) => {
                use chrono::{offset::Utc, DateTime, NaiveDateTime};

                let dt = NaiveDateTime::from_sql(&dt)?.map(|dt| DateTime::<Utc>::from_utc(dt, Utc));

                Value::DateTime(dt)
            }
            #[cfg(feature = "chrono-0_4")]
            dt @ ColumnData::DateTimeOffset(_) => {
                use chrono::{offset::Utc, DateTime};
                Value::DateTime(DateTime::<Utc>::from_sql(&dt)?)
            }
            ColumnData::Xml(_) => panic!("XML not supprted yet"),
        };

        Ok(res)
    }
}
