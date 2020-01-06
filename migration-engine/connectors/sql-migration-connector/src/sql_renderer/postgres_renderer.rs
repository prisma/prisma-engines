use super::common::*;
use crate::SqlFamily;
use sql_schema_describer::*;

pub struct PostgresRenderer {}
impl super::SqlRenderer for PostgresRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Postgres
    }

    fn quote(&self, name: &str) -> String {
        format!("\"{}\"", name)
    }

    fn render_column(&self, _schema_name: &str, _table: &Table, column: &Column, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let is_serial = column.auto_increment;

        if is_serial {
            format!("{} SERIAL", column_name)
        } else {
            format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str)
        }
    }

    //Render the Arity here
    fn render_column_type(&self, t: &ColumnType) -> String {
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
            x => unimplemented!("{:?} not handled yet", x),
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String {
        format!(
            "REFERENCES \"{}\".\"{}\"(\"{}\") {}",
            schema_name,
            foreign_key.referenced_table,
            foreign_key.referenced_columns.first().unwrap(),
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}
