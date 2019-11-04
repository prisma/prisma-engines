use crate::{
    ast::ParameterizedValue,
    connector::queryable::{ToColumnNames, ToRow},
};
use rusqlite::{types::ValueRef, Row as SqliteRow, Rows as SqliteRows};

impl<'a> ToRow for SqliteRow<'a> {
    fn to_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>> {
        let mut row = Vec::new();

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
                ValueRef::Real(f) => ParameterizedValue::Real(f),
                ValueRef::Text(bytes) => {
                    ParameterizedValue::Text(String::from_utf8(bytes.to_vec())?.into())
                }
                ValueRef::Blob(_) => panic!("Blobs not supprted, yet"),
            };

            row.push(pv);
        }

        Ok(row)
    }
}

impl<'a> ToColumnNames for SqliteRows<'a> {
    fn to_column_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        if let Some(columns) = self.column_names() {
            for column in columns {
                names.push(String::from(column));
            }
        }

        names
    }
}
