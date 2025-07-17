use std::borrow::Cow;

use diagnostics::DatamodelError;
use schema_ast::ast::{self, WithName, WithSpan};

use crate::{
    Context, ScalarFieldId,
    attributes::{FieldResolutionError, format_fields_in_error_with_leading_word, resolve_field_array_without_args},
    types::{ModelAttributes, ScalarField, ShardKeyAttribute},
};

/// `@@shardKey` on models
pub(super) fn model(model_data: &mut ModelAttributes, model_id: crate::ModelId, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    let fields = match ctx.visit_default_arg("fields") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let resolved_fields = match resolve_field_array_without_args(fields, attr.span, model_id, ctx) {
        Ok(fields) => fields,
        Err(FieldResolutionError::AlreadyDealtWith) => return,
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                let field_names = unresolvable_fields
                    .into_iter()
                    .map(|((file_id, top_id), field_name)| match top_id {
                        ast::TopId::CompositeType(ctid) => {
                            let ct_name = ctx.asts[(file_id, ctid)].name();
                            Cow::from(format!("{field_name} in type {ct_name}"))
                        }
                        ast::TopId::Model(_) => Cow::from(field_name),
                        _ => unreachable!(),
                    });

                let msg = format!(
                    "The multi field shard key declaration refers to the unknown {}.",
                    format_fields_in_error_with_leading_word(field_names),
                );

                ctx.push_error(DatamodelError::new_model_validation_error(
                    &msg,
                    "model",
                    ctx.asts[model_id].name(),
                    fields.span(),
                ));
            }

            if !relation_fields.is_empty() {
                let field_names = relation_fields.iter().map(|(f, _)| f.name());

                let msg = format!(
                    "The shard key definition refers to the relation {}. Shard key definitions must reference only scalar fields.",
                    format_fields_in_error_with_leading_word(field_names),
                );

                ctx.push_error(DatamodelError::new_model_validation_error(
                    &msg,
                    "model",
                    ctx.asts[model_id].name(),
                    attr.span,
                ));
            }

            return;
        }
    };

    let ast_model = &ctx.asts[model_id];

    // shardKey attribute fields must reference only required fields.
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .filter_map(|id| {
            let ScalarField { model_id, field_id, .. } = ctx.types[*id];
            let field = &ctx.asts[model_id][field_id];
            (!field.arity.is_required()).then_some(field.name())
        })
        .collect();

    if !fields_that_are_not_required.is_empty() && !model_data.is_ignored {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!(
                "The shard key definition refers to the optional {}. Shard key definitions must reference only required fields.",
                format_fields_in_error_with_leading_word(fields_that_are_not_required),
            ),
            "model",
            ast_model.name(),
            attr.span,
        ))
    }

    if model_data.shard_key.is_some() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "Each model must have at most one shard key. You can't have `@shardKey` and `@@shardKey` at the same time.",
            "model",
            ast_model.name(),
            ast_model.span(),
        ))
    }

    model_data.shard_key = Some(ShardKeyAttribute {
        source_attribute: ctx.current_attribute_id(),
        fields: resolved_fields,
        source_field: None,
    });
}

/// `@shardKey` on fields
pub(super) fn field<'db>(
    ast_model: &'db ast::Model,
    scalar_field_id: ScalarFieldId,
    field_id: ast::FieldId,
    model_attributes: &mut ModelAttributes,
    ctx: &mut Context<'db>,
) {
    if model_attributes.shard_key.is_some() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "At most one field must be marked as the shard key with the `@shardKey` attribute.",
            "model",
            ast_model.name(),
            ast_model.span(),
        ))
    } else {
        let source_attribute = ctx.current_attribute_id();
        model_attributes.shard_key = Some(ShardKeyAttribute {
            source_attribute,
            fields: vec![scalar_field_id],
            source_field: Some(field_id),
        })
    }
}

// This has to be a separate step because we don't have the model attributes
// (which may include `@@ignored`) collected yet when we process field attributes.
pub(super) fn validate_shard_key_field_arities(
    model_id: crate::ModelId,
    model_attributes: &ModelAttributes,
    ctx: &mut Context<'_>,
) {
    if model_attributes.is_ignored {
        return;
    }

    let Some(pk) = &model_attributes.shard_key else {
        return;
    };

    let ast_field = if let Some(field_id) = pk.source_field {
        &ctx.asts[model_id][field_id]
    } else {
        return;
    };

    if !ast_field.arity.is_required() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "Fields that are marked as shard keys must be required.",
            "@shardKey",
            ctx.asts[pk.source_attribute].span,
        ))
    }
}
