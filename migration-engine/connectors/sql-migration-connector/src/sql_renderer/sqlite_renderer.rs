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

    fn render_column(&self, _schema_name: &str, _table: &Table, column: &Column, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let auto_increment_str = if column.auto_increment {
            "PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{} {} {} {} {}",
            column_name, tpe_str, nullability_str, default_str, auto_increment_str
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
        use itertools::Itertools;

        let referenced_fields = foreign_key
            .referenced_columns
            .iter()
            .map(|col| format!(r#""{}""#, col))
            .join(",");

        format!(
            "REFERENCES \"{}\"({}) {}",
            foreign_key.referenced_table,
            referenced_fields,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}
