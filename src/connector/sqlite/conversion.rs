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
                ValueRef::Null => Value::Null,
                ValueRef::Integer(i) => match column.decl_type() {
                    Some("BOOLEAN") => {
                        if i == 0 {
                            Value::Boolean(false)
                        } else {
                            Value::Boolean(true)
                        }
                    }
                    _ => Value::Integer(i),
                },
                ValueRef::Real(f) => Value::from(f),
                ValueRef::Text(bytes) => Value::Text(String::from_utf8(bytes.to_vec())?.into()),
                ValueRef::Blob(bytes) => Value::Bytes(bytes.to_owned().into()),
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
            Value::Null => ToSqlOutput::from(Null),
            Value::Integer(integer) => ToSqlOutput::from(*integer),
            Value::Real(d) => ToSqlOutput::from((*d).to_f64().expect("Decimal is not a f64.")),
            Value::Text(cow) => ToSqlOutput::from(&**cow),
            Value::Enum(cow) => ToSqlOutput::from(&**cow),
            Value::Boolean(boo) => ToSqlOutput::from(*boo),
            Value::Char(c) => ToSqlOutput::from(*c as u8),
            Value::Bytes(bytes) => ToSqlOutput::from(bytes.as_ref()),
            #[cfg(feature = "array")]
            Value::Array(_) => unimplemented!("Arrays are not supported for sqlite."),
            #[cfg(feature = "json-1")]
            Value::Json(value) => {
                let stringified =
                    serde_json::to_string(value).map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))?;
                ToSqlOutput::from(stringified)
            }
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(value) => ToSqlOutput::from(value.to_hyphenated().to_string()),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(value) => ToSqlOutput::from(value.timestamp_millis()),
        };

        Ok(value)
    }
}
