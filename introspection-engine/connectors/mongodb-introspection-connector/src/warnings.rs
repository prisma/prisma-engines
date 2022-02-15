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

pub(crate) fn fields_with_empty_names(fields_with_empty_names: &[(Name, String)]) -> Warning {
    let affected = serde_json::Value::Array({
        fields_with_empty_names
            .iter()
            .map(|(container, field)| match container {
                Name::Model(name) => json!({
                    "model": name,
                    "field": field
                }),
                Name::CompositeType(name) => json!({
                    "type": name,
                    "field": field
                }),
            })
            .collect()
    });

    Warning {
        code: 4,
        message: "These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute.".into(),
        affected,
    }
}

pub(crate) fn fields_with_unknown_types(unknown_types: &[(Name, String)]) -> Warning {
    let affected = serde_json::Value::Array({
        unknown_types
            .iter()
            .map(|(name, field)| match name {
                Name::Model(name) => json!({
                    "model": name,
                    "field": field,
                }),
                Name::CompositeType(name) => json!({
                    "compositeType": name,
                    "field": field,
                }),
            })
            .collect()
    });

    Warning {
        code: 103,
        message: "Could not determine the types for the following fields.".into(),
        affected,
    }
}
