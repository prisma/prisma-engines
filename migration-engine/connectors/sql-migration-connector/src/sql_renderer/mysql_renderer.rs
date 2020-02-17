use super::common::*;
use crate::{sql_schema_helpers::ColumnRef, SqlFamily};
use sql_schema_describer::*;
use std::fmt::Write as _;

const VARCHAR_LENGTH_PREFIX: &str = "(191)";

pub struct MySqlRenderer {}

impl super::SqlRenderer for MySqlRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }

    fn write_quoted(&self, buf: &mut String, name: &str) -> std::fmt::Result {
        write!(buf, "{}", quoted(name))
    }

    fn quote(&self, name: &str) -> String {
        quoted(name).to_string()
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = self.render_column_type(&column).unwrap();
        let nullability_str = render_nullability(&column);
        let default_str = render_default(&column);
        let foreign_key = column.table().foreign_key_for_column(column.name());
        let auto_increment_str = if column.auto_increment() { "AUTO_INCREMENT" } else { "" };

        match foreign_key {
            Some(_) => format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str),
            None => format!(
                "{} {} {} {} {}",
                column_name, tpe_str, nullability_str, default_str, auto_increment_str
            ),
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_columns = foreign_key
            .referenced_columns
            .iter()
            .map(|col| self.quote(col))
            .join(",");

        format!(
            " REFERENCES `{}`.`{}`({}) {} ON UPDATE CASCADE",
            schema_name,
            foreign_key.referenced_table,
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }
}

impl MySqlRenderer {
    fn render_column_type(&self, column: &ColumnRef<'_>) -> anyhow::Result<String> {
        match &column.column_type().family {
            ColumnTypeFamily::Boolean => Ok(format!("boolean")),
            ColumnTypeFamily::DateTime => Ok(format!("datetime(3)")),
            ColumnTypeFamily::Float => Ok(format!("Decimal(65,30)")),
            ColumnTypeFamily::Int => Ok(format!("int")),
            // we use varchar right now as mediumtext doesn't allow default values
            // a bigger length would not allow to use such a column as primary key
            ColumnTypeFamily::String => Ok(format!("varchar{}", VARCHAR_LENGTH_PREFIX)),
            ColumnTypeFamily::Enum(enum_name) => {
                let r#enum = column
                    .schema()
                    .get_enum(&enum_name)
                    .ok_or_else(|| anyhow::anyhow!("Could not render the variants of enum `{}`", enum_name))?;

                let variants: String = r#enum.values.iter().map(quoted_string).join(", ");

                Ok(format!("ENUM({})", variants))
            }
            x => unimplemented!("{:?} not handled yet", x),
        }
    }
}

pub(crate) fn quoted<T: std::fmt::Display>(t: T) -> MysqlQuoted<T> {
    MysqlQuoted(t)
}

#[derive(Debug)]
pub(crate) struct MysqlQuoted<T>(T);

impl<T> std::fmt::Display for MysqlQuoted<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

pub(crate) fn quoted_string<T: std::fmt::Display>(t: T) -> MysqlQuotedString<T> {
    MysqlQuotedString(t)
}

#[derive(Debug)]
pub(crate) struct MysqlQuotedString<T>(T);

impl<T> std::fmt::Display for MysqlQuotedString<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0)
    }
}
