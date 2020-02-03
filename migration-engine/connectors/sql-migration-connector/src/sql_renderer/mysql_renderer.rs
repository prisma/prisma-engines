use super::common::*;
use crate::SqlFamily;
use sql_schema_describer::*;
use std::fmt::Write as _;

const VARCHAR_LENGTH_PREFIX: &str = "(191)";

pub struct MySqlRenderer {}

impl super::SqlRenderer for MySqlRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }

    fn write_quoted(&self, buf: &mut String, name: &str) -> std::fmt::Result {
        write!(buf, "`{}`", name)
    }

    fn quote(&self, name: &str) -> String {
        format!("`{}`", name)
    }

    fn render_column(&self, _schema_name: &str, table: &Table, column: &Column, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let tpe_str = self.render_column_type(&column.tpe);
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let foreign_key = table.foreign_key_for_column(&column.name);
        let auto_increment_str = if column.auto_increment { "AUTO_INCREMENT" } else { "" };

        match foreign_key {
            Some(_) => format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str),
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
            ColumnTypeFamily::String => format!("varchar{}", VARCHAR_LENGTH_PREFIX),
            x => unimplemented!("{:?} not handled yet", x),
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_columns = foreign_key
            .referenced_columns
            .iter()
            .map(|col| self.quote(col))
            .join(",");

        format!(
            "REFERENCES `{}`.`{}`({}) {}",
            schema_name,
            foreign_key.referenced_table,
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}
