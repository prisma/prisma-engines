use crate::transform::ast_to_dml::{db::walkers::CompositeTypeWalker, validation_pipeline::context::Context};
use diagnostics::DatamodelError;

pub(crate) fn composite_types_support(composite_type: CompositeTypeWalker<'_, '_>, ctx: &mut Context<'_>) {
    if ctx.connector.supports_composite_types() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        format!("Composite types are not supported on {}.", ctx.connector.name()),
        composite_type.ast_composite_type().span,
    ));
}
