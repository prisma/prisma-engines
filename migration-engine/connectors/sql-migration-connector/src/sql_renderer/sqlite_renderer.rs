use super::common::*;
use crate::SqlFamily;
use sql_schema_describer::*;

pub struct SqliteRenderer {}

impl super::SqlRenderer for SqliteRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }

    fn quote(&self, name: &str) -> String {
        format!("\"{}\"", name)
    }

    fn render_column(&self, schema_name: &str, table: &Table, column: &Column, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let foreign_key = table.foreign_key_for_column(&column.name);
        let references_str: String = if let Some(foreign_key) = foreign_key {
            self.render_references(&schema_name, foreign_key)
        } else {
            String::new()
        };
        let auto_increment_str = if column.auto_increment {
            "PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{} {} {} {} {} {}",
            column_name, tpe_str, nullability_str, default_str, auto_increment_str, references_str
        )
    }

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

    fn render_references(&self, _schema_name: &str, foreign_key: &ForeignKey) -> String {
        format!(
            "REFERENCES \"{}\"(\"{}\") {}",
            foreign_key.referenced_table,
            foreign_key.referenced_columns.first().unwrap(),
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}
