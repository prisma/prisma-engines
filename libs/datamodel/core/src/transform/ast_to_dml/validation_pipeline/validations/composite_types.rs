use super::default_value;
use crate::transform::ast_to_dml::{db::walkers::CompositeTypeWalker, validation_pipeline::context::Context};
use diagnostics::DatamodelError;
use parser_database::walkers::CompositeTypeFieldWalker;

pub(crate) fn composite_types_support(composite_type: CompositeTypeWalker<'_, '_>, ctx: &mut Context<'_>) {
    if ctx.connector.supports_composite_types() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        format!("Composite types are not supported on {}.", ctx.connector.name()),
        composite_type.ast_composite_type().span,
    ));
}

pub(super) fn validate_default_value(field: CompositeTypeFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    let default_value = field.default_value();
    let scalar_type = field.r#type().as_builtin_scalar();

    default_value::validate_default_value(default_value, scalar_type, ctx);
}
