use super::common::*;
use itertools::Itertools;
use sql_schema_describer::*;

const MYSQL_TEXT_FIELD_INDEX_PREFIX: &str = "(191)";

pub struct MySqlRenderer {}
impl super::SqlRenderer for MySqlRenderer {
    fn quote(&self, name: &str) -> String {
        format!("`{}`", name)
    }

    fn render_column(&self, schema_name: &str, table: &Table, column: &Column, add_fk_prefix: bool) -> String {
        let column_name = self.quote(&column.name);
        let column_is_part_of_pk = table
            .primary_key
            .as_ref()
            .filter(|pk| pk.columns.contains(&column.name))
            .is_some();
        let nullability_str = render_nullability(&table, &column);
        let default_str = self.render_default(&column).unwrap_or_else(String::new);

        let foreign_key = table.foreign_key_for_column(&column.name);
        let references_str = self.render_references(&schema_name, foreign_key);
        let auto_increment_str = if column.auto_increment { "AUTO_INCREMENT" } else { "" };

        let tpe_str = if column_is_part_of_pk || foreign_key.is_some() {
            self.render_id_or_relation_column_type(&column.tpe)
        } else {
            self.render_column_type(&column.tpe)
        };

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

    fn render_id_or_relation_column_type(&self, t: &ColumnType) -> String {
        match &t.family {
            ColumnTypeFamily::String => format!("varchar(191)"),
            _ => self.render_column_type(t),
        }
    }

    fn render_column_type(&self, t: &ColumnType) -> String {
        match &t.family {
            ColumnTypeFamily::Boolean => format!("boolean"),
            ColumnTypeFamily::DateTime => format!("datetime(3)"),
            ColumnTypeFamily::Float => format!("Decimal(65,30)"),
            ColumnTypeFamily::Int => format!("int"),
            ColumnTypeFamily::String => format!("mediumtext"),
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

    // For String columns, we can't index the whole column, so we have to add the prefix (e.g. `ON name(191)`).
    fn render_index_columns(&self, table: &Table, columns: &[String]) -> String {
        columns
            .iter()
            .map(|name| {
                (
                    name,
                    &table
                        .columns
                        .iter()
                        .find(|col| &col.name == name)
                        .expect("Index column is in the table.")
                        .tpe
                        .family,
                )
            })
            .map(|(name, tpe)| {
                if tpe == &ColumnTypeFamily::String {
                    format!("{}{}", self.quote(&name), MYSQL_TEXT_FIELD_INDEX_PREFIX)
                } else {
                    self.quote(&name)
                }
            })
            .join(", ")
    }

    fn render_default(&self, column: &Column) -> Option<String> {
        // Before MySQL 8, mediumtext (String) columns cannot have a default.
        if column.tpe.family == ColumnTypeFamily::String {
            return None;
        }

        Some(super::common::render_default(column))
    }
}
