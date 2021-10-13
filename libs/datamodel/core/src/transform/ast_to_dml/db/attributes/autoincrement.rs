use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{context::Context, types::ModelAttributes},
};

pub(super) fn validate_auto_increment(
    model_id: ast::ModelId,
    model_attributes: &ModelAttributes<'_>,
    ctx: &mut Context<'_>,
) {
    let autoincrement_fields = || {
        ctx.db
            .types
            .range_model_scalar_fields(model_id)
            .filter(|(_, f)| f.is_autoincrement())
    };

    if ctx.db.datasource().is_none() || autoincrement_fields().next().is_none() {
        return; // just don't try to validate without a datasource or autoincrement fields
    }

    let mut errors = Vec::new();

    // First check if the provider supports autoincrement at all. If yes, proceed with the detailed checks.
    if !ctx.db.active_connector().supports_auto_increment() {
        for (field_id, _) in autoincrement_fields() {
            // Add an error for all autoincrement fields on the model.
            errors.push(DatamodelError::new_attribute_validation_error(
                "The `autoincrement()` default value is used with a datasource that does not support it.",
                "default",
                ctx.db.ast[model_id][field_id].span,
            ));
        }

        return;
    }

    if !ctx.db.active_connector().supports_multiple_auto_increment() && autoincrement_fields().count() > 1 {
        errors.push(
            DatamodelError::new_attribute_validation_error(
            "The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.",
            "default",
            ctx.db.ast[model_id].span,
        ))
    }

    // go over all fields
    for (field_id, _scalar_field) in autoincrement_fields() {
        let field_is_indexed = || model_attributes.field_is_indexed_for_autoincrement(field_id);

        if !ctx.db.active_connector().supports_non_id_auto_increment() && !model_attributes.field_is_single_pk(field_id)
        {
            errors.push(DatamodelError::new_attribute_validation_error(
                        "The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.",
                        "default",
                        ctx.db.ast[model_id][field_id].span,
                    ))
        }

        if !ctx.db.active_connector().supports_non_indexed_auto_increment() && !field_is_indexed() {
            errors.push(DatamodelError::new_attribute_validation_error(
                        "The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.",
                        "default",
                        ctx.db.ast[model_id][field_id].span,
                    ))
        }
    }

    errors.into_iter().for_each(|err| ctx.diagnostics.push_error(err));
}
