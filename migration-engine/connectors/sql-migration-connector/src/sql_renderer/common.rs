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
        Some(value) => {
            let default = match column.tpe.family {
                ColumnTypeFamily::String | ColumnTypeFamily::DateTime => {
                    // TODO: find a better solution for this amazing hack. the default value must not be a String
                    if value.starts_with("'") {
                        format!("DEFAULT {}", value)
                    } else {
                        format!("DEFAULT '{}'", value)
                    }
                }
                _ => format!("DEFAULT {}", value),
            };
            // we use the default value right now only to smoothen migrations. So we only use it when absolutely needed.
            if column.is_required() {
                default
            } else {
                "".to_string()
            }
        }
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

// TODO: this returns None for expressions
// TODO: bring back once values for columns are not untyped Strings anymore
//fn render_value(value: &Value) -> Option<String> {
//    match value {
//        Value::Boolean(x) => Some(if *x { "true".to_string() } else { "false".to_string() }),
//        Value::Int(x) => Some(format!("{}", x)),
//        Value::Float(x) => Some(format!("{}", x)),
//        Value::Decimal(x) => Some(format!("{}", x)),
//        Value::String(x) => Some(format!("'{}'", x)),
//
//        Value::DateTime(x) => {
//            let mut raw = format!("{}", x); // this will produce a String 1970-01-01 00:00:00 UTC
//            raw.truncate(raw.len() - 4); // strip the UTC suffix
//            Some(format!("'{}'", raw)) // add quotes
//        }
//        Value::ConstantLiteral(x) => Some(format!("'{}'", x)), // this represents enum values
//        _ => None,
//    }
//}
