use super::common::*;
use crate::{sql_schema_helpers::*, SqlFamily};
use sql_schema_describer::*;
use std::fmt::Write as _;

pub struct SqliteRenderer;

impl super::SqlRenderer for SqliteRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }

    fn quote(&self, name: &str) -> String {
        format!("{}", quoted(name))
    }

    fn write_quoted(&self, buf: &mut String, name: &str) -> std::fmt::Result {
        write!(buf, "{}", quoted(name))
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = quoted(column.name());
        let tpe_str = self.render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let auto_increment_str = if column.auto_increment() {
            "PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{} {} {} {} {}",
            column_name, tpe_str, nullability_str, default_str, auto_increment_str
        )
    }

    fn render_references(&self, _schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_fields = foreign_key.referenced_columns.iter().map(SqliteQuoted).join(",");

        format!(
            "REFERENCES {referenced_table}({referenced_fields}) {on_delete_action} ON UPDATE CASCADE",
            referenced_table = quoted(&foreign_key.referenced_table),
            referenced_fields = referenced_fields,
            on_delete_action = render_on_delete(&foreign_key.on_delete_action)
        )
    }
}

impl SqliteRenderer {
    fn render_column_type(&self, t: &ColumnType) -> String {
        match &t.family {
            ColumnTypeFamily::Boolean => format!("BOOLEAN"),
            ColumnTypeFamily::DateTime => format!("DATE"),
            ColumnTypeFamily::Float => format!("REAL"),
            ColumnTypeFamily::Int => format!("INTEGER"),
            ColumnTypeFamily::String => format!("TEXT"),
            x => unimplemented!("{:?} not handled yet", x),
        }
    }
}

pub(crate) fn quoted<T>(t: T) -> SqliteQuoted<T>
where
    T: std::fmt::Display,
{
    SqliteQuoted(t)
}

#[derive(Debug)]
pub(crate) struct SqliteQuoted<T>(T);

impl<T> std::fmt::Display for SqliteQuoted<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#""{}""#, self.0)
    }
}
