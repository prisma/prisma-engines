use super::common::*;
use crate::{sql_schema_helpers::*, SqlFamily};
use sql_schema_describer::*;
use std::fmt::Write as _;

pub struct PostgresRenderer {}
impl super::SqlRenderer for PostgresRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Postgres
    }

    fn write_quoted(&self, buf: &mut String, name: &str) -> std::fmt::Result {
        write!(buf, "{}", quoted(name))
    }

    fn quote(&self, name: &str) -> String {
        quoted(name).to_string()
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .map(|default| format!("DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
        let is_serial = column.auto_increment();

        if is_serial {
            format!("{} SERIAL", column_name)
        } else {
            format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str)
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_columns = foreign_key.referenced_columns.iter().map(quoted).join(",");

        format!(
            "REFERENCES {}.{}({}) {}  ON UPDATE CASCADE",
            quoted(schema_name),
            quoted(&foreign_key.referenced_table),
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}

pub(crate) fn render_column_type(t: &ColumnType) -> String {
    let array = match t.arity {
        ColumnArity::List => "[]",
        _ => "",
    };

    match &t.family {
        ColumnTypeFamily::Boolean => format!("boolean {}", array),
        ColumnTypeFamily::DateTime => format!("timestamp(3) {}", array),
        ColumnTypeFamily::Float => format!("Decimal(65,30) {}", array),
        ColumnTypeFamily::Int => format!("integer {}", array),
        ColumnTypeFamily::String => format!("text {}", array),
        ColumnTypeFamily::Enum(name) => format!("{}{}", quoted(name), array),
        ColumnTypeFamily::Json => format!("jsonb {}", array),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

pub(crate) fn quoted_string<T>(t: T) -> PostgresQuotedString<T> {
    PostgresQuotedString(t)
}

#[derive(Debug)]
pub(crate) struct PostgresQuotedString<T>(T);

impl<T> std::fmt::Display for PostgresQuotedString<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0)
    }
}

pub(crate) fn quoted<T>(t: T) -> PostgresQuoted<T> {
    PostgresQuoted(t)
}

#[derive(Debug)]
pub(crate) struct PostgresQuoted<T>(T);

impl<T> std::fmt::Display for PostgresQuoted<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#""{}""#, self.0)
    }
}
