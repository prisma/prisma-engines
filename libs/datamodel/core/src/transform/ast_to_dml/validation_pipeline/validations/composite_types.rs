use super::default_value;
use crate::transform::ast_to_dml::{db::walkers::CompositeTypeWalker, validation_pipeline::context::Context};
use diagnostics::DatamodelError;
use parser_database::walkers::CompositeTypeFieldWalker;

/// Does the connector support composite types.
pub(crate) fn composite_types_support(composite_type: CompositeTypeWalker<'_, '_>, ctx: &mut Context<'_>) {
    if ctx.connector.supports_composite_types() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        format!("Composite types are not supported on {}.", ctx.connector.name()),
        composite_type.ast_composite_type().span,
    ));
}

/// A composite type must have at least one visible field.
pub(crate) fn more_than_one_field(composite_type: CompositeTypeWalker<'_, '_>, ctx: &mut Context<'_>) {
    let num_of_fields = composite_type.fields().filter(|f| f.is_visible()).count();

    if num_of_fields > 0 {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        String::from("A type must have at least one field defined."),
        composite_type.ast_composite_type().span,
    ));
}

/// Validates the @default attribute of a composite scalar field
pub(super) fn validate_default_value(field: CompositeTypeFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    let default_value = field.default_value();
    let default_attribute = field.default_attribute();

    if field.default_mapped_name().is_some() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "A `map` argument for the default value of a field on a composite type is not allowed. Consider removing it.",
            "default",
            default_attribute.unwrap().span,
        ));
    }

    let scalar_type = field.r#type().as_builtin_scalar();

    default_value::validate_default_value(default_value, scalar_type, ctx);
}
