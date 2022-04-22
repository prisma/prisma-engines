use std::borrow::Cow;

use super::{FieldResolutionError, FieldResolvingSetup};
use crate::{
    ast::{self, WithName},
    attributes::resolve_field_array_with_args,
    context::Context,
    types::{FieldWithArgs, IdAttribute, IndexFieldPath, ModelAttributes, SortOrder},
    DatamodelError, StringId,
};

/// @@id on models
pub(super) fn model(model_data: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    let fields = match ctx.visit_default_arg("fields") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let resolving = FieldResolvingSetup::OnlyTopLevel;

    let resolved_fields = match resolve_field_array_with_args(&fields, attr.span, model_id, resolving, ctx) {
        Ok(fields) => fields,
        Err(FieldResolutionError::AlreadyDealtWith) => return,
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                let fields_str = unresolvable_fields
                    .into_iter()
                    .map(|(top_id, field_name)| match top_id {
                        ast::TopId::CompositeType(ctid) => {
                            let ct_name = &ctx.ast[ctid].name.name;

                            Cow::from(format!("{field_name} in type {ct_name}"))
                        }
                        ast::TopId::Model(_) => Cow::from(field_name),
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let msg = format!("The multi field id declaration refers to the unknown fields {fields_str}.");
                let error = DatamodelError::new_model_validation_error(&msg, ctx.ast[model_id].name(), fields.span());

                ctx.push_error(error);
            }

            if !relation_fields.is_empty() {
                let field_names = relation_fields
                    .iter()
                    .map(|(f, _)| f.name())
                    .collect::<Vec<_>>()
                    .join(", ");

                let msg = format!("The id definition refers to the relation fields {field_names}. ID definitions must reference only scalar fields.");

                ctx.push_error(DatamodelError::new_model_validation_error(
                    &msg,
                    ctx.ast[model_id].name(),
                    attr.span,
                ));
            }

            return;
        }
    };

    let ast_model = &ctx.ast[model_id];

    // ID attribute fields must reference only required fields.
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .filter_map(|field| {
            let field_id = field.path.field_in_index();

            match field.path.type_holding_the_indexed_field() {
                None => {
                    let field = &ctx.ast[model_id][field_id];

                    if field.arity.is_required() {
                        None
                    } else {
                        Some(field.name())
                    }
                }
                Some(ctid) => {
                    let field = &ctx.ast[ctid][field_id];

                    if field.arity.is_required() {
                        None
                    } else {
                        Some(field.name())
                    }
                }
            }
        })
        .collect();

    if !fields_that_are_not_required.is_empty() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!(
                "The id definition refers to the optional fields {}. ID definitions must reference only required fields.",
                fields_that_are_not_required.join(", ")
            ),
            &ast_model.name.name,
            attr.span,
        ))
    }

    if model_data.primary_key.is_some() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "Each model must have at most one id criteria. You can't have `@id` and `@@id` at the same time.",
            ast_model.name(),
            ast_model.span,
        ))
    }

    let (name, mapped_name) = {
        let mapped_name = primary_key_mapped_name(ctx);
        let name = super::get_name_argument(ctx);

        if let Some(name) = name {
            super::validate_client_name(attr.span, &ast_model.name.name, name, "@@id", ctx);
        }

        (name, mapped_name)
    };

    let clustered = super::validate_clustering_setting(ctx);

    model_data.primary_key = Some(IdAttribute {
        name,
        source_attribute: ctx.current_attribute_id(),
        mapped_name,
        fields: resolved_fields,
        source_field: None,
        clustered,
    });
}

pub(super) fn field<'db>(
    ast_model: &'db ast::Model,
    field_id: ast::FieldId,
    model_attributes: &mut ModelAttributes,
    ctx: &mut Context<'db>,
) {
    match model_attributes.primary_key {
        Some(_) => ctx.push_error(DatamodelError::new_model_validation_error(
            "At most one field must be marked as the id field with the `@id` attribute.",
            ast_model.name(),
            ast_model.span,
        )),
        None => {
            let mapped_name = primary_key_mapped_name(ctx);

            let length = match ctx.visit_optional_arg("length").map(|length| length.as_int()) {
                Some(Ok(length)) => Some(length as u32),
                Some(Err(err)) => {
                    ctx.push_error(err);
                    None
                }
                None => None,
            };

            let sort_order = match ctx.visit_optional_arg("sort").map(|sort| sort.as_constant_literal()) {
                Some(Ok("Desc")) => Some(SortOrder::Desc),
                Some(Ok("Asc")) => Some(SortOrder::Asc),
                Some(Ok(other)) => {
                    ctx.push_attribute_validation_error(&format!(
                        "The `sort` argument can only be `Asc` or `Desc` you provided: {}.",
                        other
                    ));
                    None
                }
                Some(Err(err)) => {
                    ctx.push_error(err);
                    None
                }
                None => None,
            };

            let clustered = super::validate_clustering_setting(ctx);

            model_attributes.primary_key = Some(IdAttribute {
                name: None,
                mapped_name,
                source_attribute: ctx.current_attribute_id(),
                fields: vec![FieldWithArgs {
                    path: IndexFieldPath::new(field_id),
                    sort_order,
                    length,
                }],
                source_field: Some(field_id),
                clustered,
            })
        }
    }
}

pub(super) fn validate_id_field_arities(
    model_id: ast::ModelId,
    model_attributes: &ModelAttributes,
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
        &ctx.ast[model_id][field_id]
    } else {
        return;
    };

    if let ast::FieldArity::List | ast::FieldArity::Optional = ast_field.arity {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "Fields that are marked as id must be required.",
            "id",
            ctx.ast[pk.source_attribute].span,
        ))
    }
}

fn primary_key_mapped_name(ctx: &mut Context<'_>) -> Option<StringId> {
    let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(Ok(name)) => Some(ctx.interner.intern(name)),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    mapped_name
}
