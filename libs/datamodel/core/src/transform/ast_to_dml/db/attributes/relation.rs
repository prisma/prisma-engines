use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{context::Context, types::RelationField},
};
use dml::relation_info::ReferentialAction;
use itertools::Itertools;

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

    let has_optional_underlying_fields = relation_field
        .fields
        .iter()
        .flatten()
        .map(|field_id| &model[*field_id])
        .any(|field| field.arity.is_optional());

    if !has_optional_underlying_fields {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            &model[field_id].name.name,
            &relation_field.fields.iter().flatten().map(|field_id| &model[*field_id].name.name).join(", "),
        ),
        ast_relation_field.span
    ));
}

/// Validates usage of `onUpdate` with the `referentialIntegrity` set to
/// `prisma`.
///
/// This is temporary to the point until Query Engine supports `onUpdate`
/// actions on emulations.
pub(super) fn validate_on_update_without_foreign_keys(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    relation_field: &RelationField<'_>,
    ctx: &mut Context<'_>,
) {
    let referential_integrity = ctx
        .db
        .datasource()
        .map(|ds| ds.referential_integrity())
        .unwrap_or_default();

    if referential_integrity.uses_foreign_keys() {
        return;
    }

    if relation_field
        .on_update
        .map(|act| act != ReferentialAction::NoAction)
        .unwrap_or(false)
    {
        let ast_model = &ctx.db.ast[model_id];
        let ast_field = &ast_model[field_id];

        let span = ast_field
            .span_for_argument("relation", "onUpdate")
            .unwrap_or(ast_field.span);

        ctx.push_error(DatamodelError::new_validation_error(
            "Referential actions other than `NoAction` will not work for `onUpdate` without foreign keys. Please follow the issue: https://github.com/prisma/prisma/issues/9014",
            span
        ));
    }
}
