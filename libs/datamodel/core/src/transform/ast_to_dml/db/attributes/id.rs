use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{context::Context, types::ModelAttributes},
};

pub(super) fn validate_id_field_arities(
    model_id: ast::ModelId,
    model_attributes: &ModelAttributes<'_>,
    ctx: &mut Context<'_>,
) {
    if model_attributes.is_ignored {
        return;
    }

    let pk = if let Some(pk) = &model_attributes.primary_key {
        pk
    } else {
        return;
    };

    let ast_field = if let Some(field_id) = pk.source_field {
        &ctx.db.ast[model_id][field_id]
    } else {
        return;
    };

    if let ast::FieldArity::List | ast::FieldArity::Optional = ast_field.arity {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "Fields that are marked as id must be required.",
            "id",
            ast_field.span,
        ))
    }
}
