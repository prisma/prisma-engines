use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::{db::ScalarType, validation_pipeline::context::Context},
};
use std::str::FromStr;

pub(super) fn validate_default_value(
    default_value: Option<&ast::Expression>,
    scalar_type: Option<ScalarType>,
    ctx: &mut Context<'_>,
) {
    use chrono::{DateTime, FixedOffset};

    let scalar_type = if let Some(scalar_type) = scalar_type {
        scalar_type
    } else {
        return;
    };

    // Scalar type specific validations.
    match (scalar_type, default_value) {
        (ScalarType::Json, Some(attribute)) => {
            if let Some((value, span)) = attribute.as_string_value() {
                if let Err(details) = serde_json::from_str::<serde_json::Value>(value) {
                    return ctx.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "Parse error: \"{bad_value}\" is not a valid JSON string. ({details})",
                            details = details,
                            bad_value = value,
                        ),
                        "default",
                        span,
                    ));
                }
            }
        }
        (ScalarType::Bytes, Some(attribute)) => {
            if let Some((value, span)) = attribute.as_string_value() {
                if let Err(details) = dml::prisma_value::decode_bytes(value) {
                    return ctx.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "Parse error: \"{bad_value}\" is not a valid base64 string. ({details})",
                            details = details,
                            bad_value = value,
                        ),
                        "default",
                        span,
                    ));
                }
            }
        }
        (ScalarType::DateTime, Some(attribute)) => {
            if let Some((value, span)) = attribute.as_string_value() {
                if let Err(details) = DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                    return ctx.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "Parse error: \"{bad_value}\" is not a valid rfc3339 datetime string. ({details})",
                            details = details,
                            bad_value = value,
                        ),
                        "default",
                        span,
                    ));
                }
            }
        }
        (ScalarType::BigInt | ScalarType::Int, Some(attribute)) => {
            if let Some((value, span)) = attribute.as_numeric_value() {
                if let Err(details) = i64::from_str(value) {
                    return ctx.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "Parse error: \"{bad_value}\" is not a valid integer. ({details})",
                            details = details,
                            bad_value = value,
                        ),
                        "default",
                        span,
                    ));
                }
            }
        }
        _ => (),
    }
}
