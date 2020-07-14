use super::common::*;
use crate::{flavour::PostgresFlavour, sql_schema_helpers::*};
use once_cell::sync::Lazy;
use prisma_models::PrismaValue;
use regex::Regex;
use sql_schema_describer::*;
use std::borrow::Cow;

impl super::SqlRenderer for PostgresFlavour {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str> {
        Quoted::postgres_ident(name)
    }

    fn render_column(&self, _schema_name: &str, column: ColumnRef<'_>, _add_fk_prefix: bool) -> String {
        let column_name = self.quote(column.name());
        let tpe_str = render_column_type(column.column_type());
        let nullability_str = render_nullability(&column);
        let default_str = column
            .default()
            .filter(|default| !matches!(default, DefaultValue::DBGENERATED(_)))
            .map(|default| format!("DEFAULT {}", self.render_default(default, &column.column.tpe.family)))
            .unwrap_or_else(String::new);
        let is_serial = column.auto_increment();

        if is_serial {
            format!("{} SERIAL", column_name)
        } else {
            format!("{} {} {} {}", column_name, tpe_str, nullability_str, default_str)
        }
    }

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String {
        let referenced_columns = foreign_key
            .referenced_columns
            .iter()
            .map(Quoted::postgres_ident)
            .join(",");

        format!(
            "REFERENCES {}({}) {}  ON UPDATE CASCADE",
            self.quote_with_schema(schema_name, &foreign_key.referenced_table),
            referenced_columns,
            render_on_delete(&foreign_key.on_delete_action)
        )
    }

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str> {
        match (default, family) {
            (DefaultValue::DBGENERATED(val), _) => val.as_str().into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::String)
            | (DefaultValue::VALUE(PrismaValue::Enum(val)), ColumnTypeFamily::Enum(_)) => {
                format!("E'{}'", escape_string_literal(&val)).into()
            }
            (DefaultValue::NOW, ColumnTypeFamily::DateTime) => "CURRENT_TIMESTAMP".into(),
            (DefaultValue::NOW, _) => unreachable!("NOW default on non-datetime column"),
            (DefaultValue::VALUE(val), ColumnTypeFamily::DateTime) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(PrismaValue::String(val)), ColumnTypeFamily::Json) => format!("'{}'", val).into(),
            (DefaultValue::VALUE(val), _) => val.to_string().into(),
            (DefaultValue::SEQUENCE(_), _) => todo!("rendering of sequence defaults"),
        }
    }
}

pub(crate) fn render_column_type(t: &ColumnType) -> String {
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
        ColumnTypeFamily::Enum(name) => format!("{}{}", Quoted::postgres_ident(name), array),
        ColumnTypeFamily::Json => format!("jsonb {}", array),
        x => unimplemented!("{:?} not handled yet", x),
    }
}

fn escape_string_literal(s: &str) -> Cow<'_, str> {
    static STRING_LITERAL_CHARACTER_TO_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'|\\"#).unwrap());

    STRING_LITERAL_CHARACTER_TO_ESCAPE_RE.replace_all(s, "\\$0")
}
