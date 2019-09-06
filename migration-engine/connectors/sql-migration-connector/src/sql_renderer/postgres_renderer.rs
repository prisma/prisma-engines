use super::common::*;
use sql_schema_describer::*;

pub struct PostgresRenderer {}
impl super::SqlRenderer for PostgresRenderer {
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

        let is_serial = column.auto_increment;

        if is_serial {
            format!("{} SERIAL", column_name)
        } else {
            format!(
                "{} {} {} {} {}",
                column_name, tpe_str, nullability_str, default_str, references_str
            )
        }
    }

    fn render_column_type(&self, t: &ColumnType) -> String {
        match &t.family {
            ColumnTypeFamily::Boolean => format!("boolean"),
            ColumnTypeFamily::DateTime => format!("timestamp(3)"),
            ColumnTypeFamily::Float => format!("Decimal(65,30)"),
            ColumnTypeFamily::Int => format!("integer"),
            ColumnTypeFamily::String => format!("text"),
            x => unimplemented!("{:?} not handled yet", x),
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: Option<&ForeignKey>) -> String {
        match foreign_key {
            Some(fk) => format!(
                "REFERENCES \"{}\".\"{}\"(\"{}\") {}",
                schema_name,
                fk.referenced_table,
                fk.referenced_columns.first().unwrap(),
                render_on_delete(&fk.on_delete_action)
            ),
            None => "".to_string(),
        }
    }
}
