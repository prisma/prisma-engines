use super::{common::*, RenderedAlterColumn, SqlRenderer};
use crate::{
    database_info::DatabaseInfo,
    expanded_alter_column::{expand_mysql_alter_column, MysqlAlterColumn},
    flavour::{MysqlFlavour, SqlFlavour},
    sql_schema_differ::{ColumnChanges, ColumnDiffer, SqlSchemaDiffer},
};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use sql_schema_describer::walkers::ColumnWalker;
use sql_schema_describer::*;
use std::borrow::Cow;

const VARCHAR_LENGTH_PREFIX: &str = "(191)";

impl SqlRenderer for MysqlFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Backticks(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnWalker<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(&column);
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| {
                !matches!(default, DefaultValue::DBGENERATED(_) | DefaultValue::SEQUENCE(_))
                    // We do not want to render JSON defaults because they are not supported by MySQL.
                    && !matches!(column.column_type_family(), ColumnTypeFamily::Json)
            })
            .map(|default| format!("DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
        let foreign_key = column.table().foreign_key_for_column(column.name());
        let auto_increment_str = if column.is_autoincrement() {
            " AUTO_INCREMENT"
        } else {
            ""
        };

        match foreign_key {
            Some(_) => format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str),
            None => format!(
                "{} {} {} {}{}",
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
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP(3)".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => format!("{}", val).into(),
            (DefaultValue::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_alter_column<'a>(&self, differ: &ColumnDiffer<'_>) -> Option<RenderedAlterColumn> {
        let expanded = expand_mysql_alter_column(differ);

        let sql = match expanded {
            MysqlAlterColumn::DropDefault => vec![format!(
                "ALTER COLUMN {column} DROP DEFAULT",
                column = Quoted::mysql_ident(differ.previous.name())
            )],
            MysqlAlterColumn::Modify { new_default, changes } => {
                vec![render_mysql_modify(&changes, new_default.as_ref(), differ.next, self)]
            }
        };

        Some(RenderedAlterColumn {
            alter_columns: sql,
            before: None,
            after: None,
        })
    }

    fn render_create_enum(&self, _create_enum: &crate::CreateEnum) -> Vec<String> {
        Vec::new() // enums are defined on each column that uses them on MySQL
    }

    fn render_drop_enum(&self, _drop_enum: &crate::DropEnum) -> Vec<String> {
        Vec::new()
    }

    fn render_redefine_tables(
        &self,
        _names: &[String],
        _differ: SqlSchemaDiffer<'_>,
        _database_info: &DatabaseInfo,
    ) -> Vec<String> {
        unreachable!("render_redefine_table on MySQL")
    }
}

fn render_mysql_modify(
    changes: &ColumnChanges,
    new_default: Option<&sql_schema_describer::DefaultValue>,
    next_column: ColumnWalker<'_>,
    renderer: &dyn SqlFlavour,
) -> String {
    let column_type: Option<String> = if changes.type_changed() {
        Some(next_column.column_type().full_data_type.clone()).filter(|r| !r.is_empty() || r.contains("datetime"))
    // @default(now()) does not work with datetimes of certain sizes
    } else {
        Some(next_column.column.tpe.full_data_type.clone()).filter(|r| !r.is_empty())
    };

    let column_type = column_type
        .map(Cow::Owned)
        .unwrap_or_else(|| render_column_type(&next_column));

    let default = new_default
        .map(|default| renderer.render_default(&default, &next_column.column_type().family))
        .filter(|expr| !expr.is_empty())
        .map(|expression| format!(" DEFAULT {}", expression))
        .unwrap_or_else(String::new);

    format!(
        "MODIFY {column_name} {column_type}{nullability}{default}{sequence}",
        column_name = Quoted::mysql_ident(&next_column.name()),
        column_type = column_type,
        nullability = if next_column.arity().is_required() {
            " NOT NULL"
        } else {
            ""
        },
        default = default,
        sequence = if next_column.is_autoincrement() {
            " AUTO_INCREMENT"
        } else {
            ""
        },
    )
}

pub(crate) fn render_column_type(column: &ColumnWalker<'_>) -> Cow<'static, str> {
    match &column.column_type().family {
        ColumnTypeFamily::Boolean => "boolean".into(),
        ColumnTypeFamily::DateTime => "datetime(3)".into(),
        ColumnTypeFamily::Float => "decimal(65,30)".into(),
        ColumnTypeFamily::Int => "int".into(),
        // we use varchar right now as mediumtext doesn't allow default values
        // a bigger length would not allow to use such a column as primary key
        ColumnTypeFamily::String => format!("varchar{}", VARCHAR_LENGTH_PREFIX).into(),
        ColumnTypeFamily::Enum(enum_name) => {
            let r#enum = column
                .schema()
                .get_enum(&enum_name)
                .unwrap_or_else(|| panic!("Could not render the variants of enum `{}`", enum_name));

            let variants: String = r#enum.values.iter().map(Quoted::mysql_string).join(", ");

            format!("ENUM({})", variants).into()
        }
        ColumnTypeFamily::Json => "json".into(),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}
