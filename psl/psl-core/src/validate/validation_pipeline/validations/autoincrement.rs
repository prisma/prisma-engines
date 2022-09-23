use crate::{
    diagnostics::DatamodelError,
    parser_database::{ast::WithSpan, walkers::ModelWalker},
    validate::validation_pipeline::context::Context,
};

pub(super) fn validate_auto_increment(model: ModelWalker<'_>, ctx: &mut Context<'_>) {
    let autoincrement_fields = || model.scalar_fields().filter(|f| f.is_autoincrement());

    if autoincrement_fields().next().is_none() {
        return; // just don't try to validate without autoincrement fields
    }

    // First check if the provider supports autoincrement at all. If yes, proceed with the detailed checks.
    if !ctx.connector.supports_auto_increment() {
        for field in autoincrement_fields() {
            let msg = "The `autoincrement()` default value is used with a datasource that does not support it.";

            // Add an error for all autoincrement fields on the model.
            ctx.push_error(DatamodelError::new_attribute_validation_error(
                msg,
                "@default",
                field.default_attribute().unwrap().span,
            ));
        }

        return;
    }

    if !ctx.connector.supports_multiple_auto_increment() && autoincrement_fields().count() > 1 {
        let msg = "The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.";

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            msg,
            "@default",
            model.ast_model().span(),
        ))
    }

    // go over all fields
    for field in autoincrement_fields() {
        let field_is_indexed = || model.field_is_indexed_for_autoincrement(field.field_id());

        if !ctx.connector.supports_non_id_auto_increment() && !model.field_is_single_pk(field.field_id()) {
            let msg = "The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.";

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                msg,
                "@default",
                field.ast_field().span(),
            ))
        }

        if !ctx.connector.supports_non_indexed_auto_increment() && !field_is_indexed() {
            let msg = "The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.";

            ctx.push_error(DatamodelError::new_attribute_validation_error(
                msg,
                "@default",
                field.ast_field().span(),
            ))
        }
    }
}
