use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{context::Context, types::RelationField},
};

/// Validate that the arity of fields from `fields` is compatible with relation field arity.
pub(super) fn validate_relation_field_arity(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    relation_field: &mut RelationField<'_>,
    ctx: &mut Context<'_>,
) {
    let model = &ctx.db.ast[model_id];
    let ast_relation_field = &model[field_id];

    if !ast_relation_field.arity.is_required() {
        return;
    }

    let optional_underlying_fields: Vec<&str> = relation_field
        .fields
        .iter()
        .flatten()
        .map(|field_id| &model[*field_id])
        .filter(|field| field.arity.is_optional())
        .map(|field| field.name.name.as_str())
        .collect();

    if optional_underlying_fields.is_empty() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            &model[field_id].name.name,
            optional_underlying_fields.join(", "),
        ),
        ast_relation_field.span
    ));
}
