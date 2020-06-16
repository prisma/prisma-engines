use crate::{
    ast::Value,
    connector::queryable::{GetRow, ToColumnNames},
};
use rusqlite::{
    types::{Null, ToSql, ToSqlOutput, ValueRef},
    Error as RusqlError, Row as SqliteRow, Rows as SqliteRows,
};
use rust_decimal::prelude::ToPrimitive;

impl<'a> GetRow for SqliteRow<'a> {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<Value<'static>>> {
        let mut row = Vec::with_capacity(self.columns().len());

        for (i, column) in self.columns().iter().enumerate() {
            let pv = match self.get_raw(i) {
                ValueRef::Null => match column.decl_type() {
                    Some("INT")
                    | Some("INTEGER")
                    | Some("SERIAL")
                    | Some("TINYINT")
                    | Some("SMALLINT")
                    | Some("MEDIUMINT")
                    | Some("BIGINT")
                    | Some("UNSIGNED BIG INT")
                    | Some("INT2")
                    | Some("INT8") => Value::Integer(None),
                    Some("TEXT") | Some("CLOB") => Value::Text(None),
                    Some(n) if n.starts_with("CHARACTER") => Value::Text(None),
                    Some(n) if n.starts_with("VARCHAR") => Value::Text(None),
                    Some(n) if n.starts_with("VARYING CHARACTER") => Value::Text(None),
                    Some(n) if n.starts_with("NCHAR") => Value::Text(None),
                    Some(n) if n.starts_with("NATIVE CHARACTER") => Value::Text(None),
                    Some(n) if n.starts_with("NVARCHAR") => Value::Text(None),
                    Some(n) if n.starts_with("DECIMAL") => Value::Real(None),
                    Some("BLOB") => Value::Bytes(None),
                    Some("NUMERIC") | Some("REAL") | Some("DOUBLE") | Some("DOUBLE PRECISION") | Some("FLOAT") => {
                        Value::Real(None)
                    }
                    Some("DATE") | Some("DATETIME") => Value::DateTime(None),
                    Some("BOOLEAN") => Value::Boolean(None),
                    Some(n) => panic!("Value {} not supported", n),
                    None => Value::Integer(None),
                },
                ValueRef::Integer(i) => match column.decl_type() {
                    Some("BOOLEAN") => {
                        if i == 0 {
                            Value::boolean(false)
                        } else {
                            Value::boolean(true)
                        }
                    }
                    _ => Value::integer(i),
                },
                ValueRef::Real(f) => Value::from(f),
                ValueRef::Text(bytes) => Value::text(String::from_utf8(bytes.to_vec())?),
                ValueRef::Blob(bytes) => Value::bytes(bytes.to_owned()),
            };

            row.push(pv);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for SqliteRows<'a> {
    fn to_column_names(&self) -> Vec<String> {
        match self.column_names() {
            Some(columns) => columns.into_iter().map(|c| c.into()).collect(),
            None => vec![],
        }
    }
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> Result<ToSqlOutput, RusqlError> {
        let value = match self {
            Value::Integer(integer) => integer.map(|i| ToSqlOutput::from(i)),
            Value::Real(d) => d.map(|d| ToSqlOutput::from(d.to_f64().expect("Decimal is not a f64."))),
            Value::Text(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Enum(cow) => cow.as_ref().map(|cow| ToSqlOutput::from(cow.as_ref())),
            Value::Boolean(boo) => boo.map(|boo| ToSqlOutput::from(boo)),
            Value::Char(c) => c.map(|c| ToSqlOutput::from(c as u8)),
            Value::Bytes(bytes) => bytes.as_ref().map(|bytes| ToSqlOutput::from(bytes.as_ref())),
            #[cfg(feature = "array")]
            Value::Array(_) => unimplemented!("Arrays are not supported for sqlite."),
            #[cfg(feature = "json-1")]
            Value::Json(value) => value.as_ref().map(|value| {
                let stringified = serde_json::to_string(value)
                    .map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))
                    .unwrap();

                ToSqlOutput::from(stringified)
            }),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(value) => value.map(|value| ToSqlOutput::from(value.to_hyphenated().to_string())),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(value) => value.map(|value| ToSqlOutput::from(value.timestamp_millis())),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(date) => date.map(|date| {
                let dt = date.and_hms(0, 0, 0);
                ToSqlOutput::from(dt.timestamp_millis())
            }),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(time) => time.map(|time| {
                use chrono::{NaiveDate, Timelike};

                let dt = NaiveDate::from_ymd(1970, 1, 1).and_hms(time.hour(), time.minute(), time.second());

                ToSqlOutput::from(dt.timestamp_millis())
            }),
        };

        match value {
            Some(value) => Ok(value),
            None => Ok(ToSqlOutput::from(Null)),
        }
    }
}
