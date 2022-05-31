use bigdecimal::BigDecimal;
use datamodel_connector::ConnectorCapability;
use parser_database::ast::Expression;

use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::{db::ScalarType, validation_pipeline::context::Context},
};
use std::str::FromStr;

/// Function `auto()` works for now only with MongoDB.
pub(super) fn validate_auto_param(default_value: Option<&ast::Expression>, ctx: &mut Context<'_>) {
    if ctx.connector.has_capability(ConnectorCapability::DefaultValueAuto) {
        return;
    }

    let expression = match default_value {
        Some(default_value) => default_value,
        None => return,
    };

    match expression {
        Expression::Function(name, _, span) if name == "auto" => {
            let message = "The current connector does not support the `auto()` function.";

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                message, "@default", *span,
            ));
        }
        _ => (),
    }
}

/// Validates the @default attribute of a scalar field
pub(super) fn validate_default_value(
    default_value: Option<&ast::Expression>,
    scalar_type: Option<ScalarType>,
    ctx: &mut Context<'_>,
) {
    use chrono::{DateTime, FixedOffset};

    let scalar_type = match scalar_type {
        Some(scalar_type) => scalar_type,
        None => return,
    };

    let expression = match default_value {
        Some(default_value) => default_value,
        None => return,
    };

    // For array expressions, validate each element in the array.
    if let ast::Expression::Array(items, _) = expression {
        for item in items {
            validate_default_value(Some(item), Some(scalar_type), ctx);
        }

        return;
    }

    // Scalar type specific validations.
    match (scalar_type, expression) {
        (ScalarType::Json, ast::Expression::StringValue(value, span)) => {
            let details = match serde_json::from_str::<serde_json::Value>(value) {
                Ok(_) => return,
                Err(details) => details,
            };

            let message = format!("Parse error: \"{value}\" is not a valid JSON string. ({details})",);

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message, "@default", *span,
            ));
        }
        (ScalarType::Bytes, ast::Expression::StringValue(value, span)) => {
            let details = match dml::prisma_value::decode_bytes(value) {
                Ok(_) => return,
                Err(details) => details,
            };

            let message = format!("Parse error: \"{value}\" is not a valid base64 string. ({details})",);

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message, "@default", *span,
            ));
        }
        (ScalarType::DateTime, ast::Expression::StringValue(value, span)) => {
            let details = match DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                Ok(_) => return,
                Err(details) => details,
            };

            let message = format!(
                "Parse error: \"{bad_value}\" is not a valid rfc3339 datetime string. ({details})",
                details = details,
                bad_value = value,
            );

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message, "@default", *span,
            ));
        }
        (ScalarType::BigInt | ScalarType::Int, ast::Expression::NumericValue(value, span)) => {
            let details = match i64::from_str(value) {
                Ok(_) => return,
                Err(details) => details,
            };

            let message = format!("Parse error: \"{value}\" is not a valid integer. ({details})");

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message, "@default", *span,
            ));
        }
        (ScalarType::Decimal, ast::Expression::StringValue(value, span)) => {
            let details = match BigDecimal::from_str(value) {
                Ok(_) => return,
                Err(details) => details,
            };

            let message = format!("Parse error: \"{value}\" is not a valid decimal. ({details})");

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                &message, "default", *span,
            ));
        }
        _ => (),
    }
}
