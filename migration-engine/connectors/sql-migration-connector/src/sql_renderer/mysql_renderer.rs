use super::common::*;
use database_introspection::*;

pub struct MySqlRenderer {}
impl super::SqlRenderer for MySqlRenderer {
    fn quote(&self, name: &str) -> String {
        format!("`{}`", name)
    }

    fn render_column(&self, schema_name: &str, table: &Table, column: &Column, add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&table, &column);
        let default_str = render_default(&column);
        let foreign_key = table.foreign_key_for_column(&column.name);
        let references_str = self.render_references(&schema_name, foreign_key);
        let auto_increment_str = if column.auto_increment { "AUTO_INCREMENT" } else { "" };

        match foreign_key {
            Some(_) => {
                let add = if add_fk_prefix { "ADD" } else { "" };
                let fk_line = format!("{} FOREIGN KEY ({}) {}", add, column_name, references_str);
                format!(
                    "{} {} {} {},\n{}",
                    column_name, tpe_str, nullability_str, default_str, fk_line
                )
            }
            None => format!(
                "{} {} {} {} {}",
                column_name, tpe_str, nullability_str, default_str, auto_increment_str
            ),
        }
    }

    fn render_column_type(&self, t: &ColumnType) -> String {
        match &t.family {
            ColumnTypeFamily::Boolean => format!("boolean"),
            ColumnTypeFamily::DateTime => format!("datetime(3)"),
            ColumnTypeFamily::Float => format!("Decimal(65,30)"),
            ColumnTypeFamily::Int => format!("int"),
            // we use varchar right now as mediumtext doesn't allow default values
            // a bigger length would not allow to use such a column as primary key
            ColumnTypeFamily::String => format!("varchar(191)"),
            x => unimplemented!("{:?} not handled yet", x),
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: Option<&ForeignKey>) -> String {
        match foreign_key {
            Some(fk) => format!(
                "REFERENCES `{}`.`{}`(`{}`) {}",
                schema_name,
                fk.referenced_table,
                fk.referenced_columns.first().unwrap(),
                render_on_delete(&fk.on_delete_action)
            ),
            None => "".to_string(),
        }
    }
}
