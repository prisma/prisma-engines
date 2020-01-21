use sql_schema_describer::*;

pub fn render_nullability(column: &Column) -> &'static str {
    if column.is_required() {
        "NOT NULL"
    } else {
        ""
    }
}

pub fn render_default(column: &Column) -> String {
    match &column.default {
        Some(value) => match column.tpe.family {
            ColumnTypeFamily::String | ColumnTypeFamily::DateTime => format!(
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

pub fn render_on_delete(on_delete: &ForeignKeyAction) -> &'static str {
    match on_delete {
        ForeignKeyAction::NoAction => "",
        ForeignKeyAction::SetNull => "ON DELETE SET NULL",
        ForeignKeyAction::Cascade => "ON DELETE CASCADE",
        ForeignKeyAction::SetDefault => "ON DELETE SET DEFAULT",
        ForeignKeyAction::Restrict => "ON DELETE RESTRICT",
    }
}
