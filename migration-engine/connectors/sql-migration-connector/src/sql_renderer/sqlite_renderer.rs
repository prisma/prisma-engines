use super::{common::*, RenderedAlterColumn, SqlRenderer};
use crate::{flavour::SqliteFlavour, sql_schema_helpers::*};
use once_cell::sync::Lazy;
use prisma_models::PrismaValue;
use regex::Regex;
use sql_schema_describer::*;
use std::borrow::Cow;

impl SqlRenderer for SqliteFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::Double(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| !matches!(default, DefaultValue::DBGENERATED(_) | DefaultValue::SEQUENCE(_)))
            .map(|default| format!(" DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
        let auto_increment_str = if column.is_autoincrement() && column.is_single_primary_key() {
            " PRIMARY KEY AUTOINCREMENT"
        } else {
            ""
        };

        format!(
            "{column_name} {tpe_str} {nullability_str}{default_str}{auto_increment}",
            column_name = column_name,
            tpe_str = tpe_str,
            nullability_str = nullability_str,
            default_str = default_str,
            auto_increment = auto_increment_str
        )
    }

    fn render_references(&self, _schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_fields = foreign_key
            .referenced_columns
            .iter()
            .map(Quoted::sqlite_ident)
            .join(",");

        format!(
            "REFERENCES {referenced_table}({referenced_fields}) {on_delete_action} ON UPDATE CASCADE",
            referenced_table = self.quote(&foreign_key.referenced_table),
            referenced_fields = referenced_fields,
            on_delete_action = render_on_delete(&foreign_key.on_delete_action)
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("'{}'", escape_quotes(&val)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => format!("{}", val).into(),
            (DefaultValue::SEQUENCE(_), _) => "".into(),
        }
    }

    fn render_alter_column(&self, _differ: &crate::sql_schema_differ::ColumnDiffer<'_>) -> Option<RenderedAlterColumn> {
        None
    }
}

fn render_column_type(t: &ColumnType) -> String {
    match &t.family {
        ColumnTypeFamily::Boolean => "BOOLEAN".to_string(),
        ColumnTypeFamily::DateTime => "DATE".to_string(),
        ColumnTypeFamily::Float => "REAL".to_string(),
        ColumnTypeFamily::Int => "INTEGER".to_string(),
        ColumnTypeFamily::String => "TEXT".to_string(),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

fn escape_quotes(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "'$0")
}
