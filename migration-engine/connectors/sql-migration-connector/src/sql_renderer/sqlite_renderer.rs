use super::common::*;
use crate::{sql_schema_helpers::*, SqlFamily};
use sql_schema_describer::*;

pub struct SqliteRenderer;

impl super::SqlRenderer for SqliteRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }

    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Double(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = self.render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .map(|default| format!("DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
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
        let referenced_fields = foreign_key
            .referenced_columns
            .iter()
            .map(Quoted::sqlite_ident)
            .join(",");

        format!(
            "REFERENCES {referenced_table}({referenced_fields}) {on_delete_action} ON UPDATE CASCADE",
            referenced_table = self.quote(&foreign_key.referenced_table),
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
