use introspection_connector::Warning;
use serde_json::json;

pub(crate) fn unsupported_type(affected: &[(String, String, &str)]) -> Warning {
    let affected = serde_json::Value::Array({
        affected
            .iter()
            .map(|(model, field, tpe)| {
                json!({
                    "model": model,
                    "field": field,
                    "tpe": tpe
                })
            })
            .collect()
    });

    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected,
    }
}

pub(crate) fn undecided_field_type(affected: &[(String, String, String)]) -> Warning {
    let affected = serde_json::Value::Array({
        affected
            .iter()
            .map(|(model, field, tpe)| {
                json!({
                    "model": model,
                    "field": field,
                    "tpe": tpe
                })
            })
            .collect()
    });

    Warning {
        code: 101,
        message: "The following fields had data stored in multiple types. The most common type was chosen. If loading data with a type that does not match the one in the data model, the client will crash. Please see the issue: https://github.com/prisma/prisma/issues/9654".into(),
        affected: json![{"name": affected}],
    }
}
