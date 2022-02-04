use introspection_connector::Warning;
use serde_json::json;

use crate::sampler::Name;

pub(crate) fn unsupported_type(affected: &[(Name, String, &str)]) -> Warning {
    let affected = serde_json::Value::Array({
        affected
            .iter()
            .map(|(name, field, tpe)| match name {
                Name::Model(name) => json!({
                    "model": name,
                    "field": field,
                    "tpe": tpe
                }),
                Name::CompositeType(name) => json!({
                    "compositeType": name,
                    "field": field,
                    "tpe": tpe
                }),
            })
            .collect()
    });

    Warning {
        code: 3,
        message: "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.".into(),
        affected,
    }
}

pub(crate) fn undecided_field_type(affected: &[(Name, String, String)]) -> Warning {
    let affected = serde_json::Value::Array({
        affected
            .iter()
            .map(|(name, field, tpe)| match name {
                Name::Model(name) => json!({
                    "model": name,
                    "field": field,
                    "tpe": tpe
                }),
                Name::CompositeType(name) => json!({
                    "compositeType": name,
                    "field": field,
                    "tpe": tpe
                }),
            })
            .collect()
    });

    Warning {
        code: 101,
        message: "The following fields had data stored in multiple types. The most common type was chosen. If loading data with a type that does not match the one in the data model, the client will crash. Please see the issue: https://github.com/prisma/prisma/issues/9654".into(),
        affected,
    }
}
