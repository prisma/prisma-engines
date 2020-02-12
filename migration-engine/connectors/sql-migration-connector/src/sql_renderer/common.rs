use crate::sql_schema_helpers::*;
use sql_schema_describer::*;
use std::fmt::Write as _;

pub(crate) fn render_nullability(column: &ColumnRef<'_>) -> &'static str {
    if column.is_required() {
        "NOT NULL"
    } else {
        ""
    }
}

pub(crate) fn render_default(column: &ColumnRef<'_>) -> String {
    match column.default() {
        Some(value) => match &column.column_type().family {
            ColumnTypeFamily::String | ColumnTypeFamily::DateTime | ColumnTypeFamily::Enum(_) => format!(
                "DEFAULT '{}'",
                // TODO: remove once sql-schema-describer does unescaping, and perform escaping again here.
                value
                    .trim_matches('\\')
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim_matches('\\')
            ),
            _ => format!("DEFAULT {}", value),
        },
        None => "".to_string(),
    }
}

pub(crate) fn render_on_delete(on_delete: &ForeignKeyAction) -> &'static str {
    match on_delete {
        ForeignKeyAction::NoAction => "",
        ForeignKeyAction::SetNull => "ON DELETE SET NULL",
        ForeignKeyAction::Cascade => "ON DELETE CASCADE",
        ForeignKeyAction::SetDefault => "ON DELETE SET DEFAULT",
        ForeignKeyAction::Restrict => "ON DELETE RESTRICT",
    }
}

pub(crate) trait IteratorJoin {
    fn join(self, sep: &str) -> String;
}

impl<T, I> IteratorJoin for T
where
    T: Iterator<Item = I>,
    I: std::fmt::Display,
{
    fn join(mut self, sep: &str) -> String {
        let (lower_bound, _) = self.size_hint();
        let mut out = String::with_capacity(sep.len() * lower_bound);

        if let Some(first_item) = self.next() {
            write!(out, "{}", first_item).unwrap();
        }

        for item in self {
            out.push_str(sep);
            write!(out, "{}", item).unwrap();
        }

        out
    }
}
