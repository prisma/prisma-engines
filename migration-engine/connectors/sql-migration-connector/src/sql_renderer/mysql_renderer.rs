use super::{common::*, SqlRenderer};
use crate::{sql_schema_helpers::ColumnRef, SqlFamily};
use once_cell::sync::Lazy;
use prisma_models::PrismaValue;
use regex::Regex;
use sql_schema_describer::*;
use std::borrow::Cow;

const VARCHAR_LENGTH_PREFIX: &str = "(191)";

pub struct MySqlRenderer {}

impl SqlRenderer for MySqlRenderer {
    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }

    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Backticks(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(&column).unwrap();
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .map(|default| format!("DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
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

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("'{}'", escape_string_literal(&val)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => format!("{}", val).into(),
            (DefaultValue::SEQUENCE(_), _) => todo!("rendering of sequence defaults"),
        }
    }
}

pub(crate) fn render_column_type(column: &ColumnRef<'_>) -> anyhow::Result<Cow<'static, str>> {
    match &column.column_type().family {
        ColumnTypeFamily::Boolean => Ok("boolean".into()),
        ColumnTypeFamily::DateTime => {
            // CURRENT_TIMESTAMP has up to second precision, not more.
            if let Some(DefaultValue::NOW) = column.default() {
                return Ok("datetime".into());
            } else {
                Ok("datetime(3)".into())
            }
        }
        ColumnTypeFamily::Float => Ok("Decimal(65,30)".into()),
        ColumnTypeFamily::Int => Ok("int".into()),
        // we use varchar right now as mediumtext doesn't allow default values
        // a bigger length would not allow to use such a column as primary key
        ColumnTypeFamily::String => Ok(format!("varchar{}", VARCHAR_LENGTH_PREFIX).into()),
        ColumnTypeFamily::Enum(enum_name) => {
            let r#enum = column
                .schema()
                .get_enum(&enum_name)
                .ok_or_else(|| anyhow::anyhow!("Could not render the variants of enum `{}`", enum_name))?;

            let variants: String = r#enum.values.iter().map(Quoted::mysql_string).join(", ");

            Ok(format!("ENUM({})", variants).into())
        }
        ColumnTypeFamily::Json => Ok("json".into()),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    const STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}
