use super::common::*;
use database_introspection::*;

pub struct SqliteRenderer {}
impl super::SqlRenderer for SqliteRenderer {
    fn quote(&self, name: &str) -> String {
        format!("\"{}\"", name)
    }

    fn render_column(&self, schema_name: &str, table: &Table, column: &Column, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&table, &column);
        let default_str = render_default(&column);
        let foreign_key = table.foreign_key_for_column(&column.name);
        let references_str = self.render_references(&schema_name, foreign_key);
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

    fn render_references(&self, _schema_name: &str, foreign_key: Option<&ForeignKey>) -> String {
        match foreign_key {
            Some(fk) => format!(
                "REFERENCES \"{}\"({}) {}",
                fk.referenced_table,
                fk.referenced_columns.first().unwrap(),
                render_on_delete(&fk.on_delete_action)
            ),
            None => "".to_string(),
        }
    }
}
