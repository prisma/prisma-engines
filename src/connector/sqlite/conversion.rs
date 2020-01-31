use crate::{
    ast::ParameterizedValue,
    connector::queryable::{GetRow, ToColumnNames},
};
use rusqlite::{
    types::{Null, ToSql, ToSqlOutput, ValueRef},
    Error as RusqlError, Row as SqliteRow, Rows as SqliteRows,
};
use rust_decimal::prelude::ToPrimitive;

impl<'a> GetRow for SqliteRow<'a> {
    fn get_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
        let mut row = Vec::with_capacity(self.columns().len());

        for (i, column) in self.columns().iter().enumerate() {
            let pv = match self.get_raw(i) {
                ValueRef::Null => ParameterizedValue::Null,
                ValueRef::Integer(i) => match column.decl_type() {
                    Some("BOOLEAN") => {
                        if i == 0 {
                            ParameterizedValue::Boolean(false)
                        } else {
                            ParameterizedValue::Boolean(true)
                        }
                    }
                    _ => ParameterizedValue::Integer(i),
                },
                ValueRef::Real(f) => ParameterizedValue::from(f),
                ValueRef::Text(bytes) => ParameterizedValue::Text(String::from_utf8(bytes.to_vec())?.into()),
                ValueRef::Blob(_) => panic!("Blobs not supported, yet"),
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

impl<'a> ToSql for ParameterizedValue<'a> {
    fn to_sql(&self) -> Result<ToSqlOutput, RusqlError> {
        let value = match self {
            ParameterizedValue::Null => ToSqlOutput::from(Null),
            ParameterizedValue::Integer(integer) => ToSqlOutput::from(*integer),
            ParameterizedValue::Real(d) => ToSqlOutput::from((*d).to_f64().expect("Decimal is not a f64.")),
            ParameterizedValue::Text(cow) => ToSqlOutput::from(&**cow),
            ParameterizedValue::Enum(cow) => ToSqlOutput::from(&**cow),
            ParameterizedValue::Boolean(boo) => ToSqlOutput::from(*boo),
            ParameterizedValue::Char(c) => ToSqlOutput::from(*c as u8),
            #[cfg(feature = "array")]
            ParameterizedValue::Array(_) => unimplemented!("Arrays are not supported for sqlite."),
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(value) => {
                let stringified =
                    serde_json::to_string(value).map_err(|err| RusqlError::ToSqlConversionFailure(Box::new(err)))?;
                ToSqlOutput::from(stringified)
            }
            #[cfg(feature = "uuid-0_8")]
            ParameterizedValue::Uuid(value) => ToSqlOutput::from(value.to_hyphenated().to_string()),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(value) => ToSqlOutput::from(value.timestamp_millis()),
        };

        Ok(value)
    }
}
