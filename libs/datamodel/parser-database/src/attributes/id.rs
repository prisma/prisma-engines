use super::FieldResolutionError;
use crate::{
    ast::{self, WithName},
    attributes::resolve_field_array_with_args,
    context::Context,
    types::{FieldWithArgs, IdAttribute, ModelAttributes, SortOrder},
    DatamodelError, StringId,
};

/// @@id on models
pub(super) fn model(model_data: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    let fields = match ctx.visit_default_arg("fields") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let resolved_fields = match resolve_field_array_with_args(&fields, attr.span, model_id, ctx) {
        Ok(fields) => fields,
        Err(FieldResolutionError::AlreadyDealtWith) => return,
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The multi field id declaration refers to the unknown fields {}.",
                        unresolvable_fields.join(", "),
                    ),
                    ctx.ast[model_id].name(),
                    fields.span(),
                ));
            }

            if !relation_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(&format!("The id definition refers to the relation fields {}. ID definitions must reference only scalar fields.", relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", ")), ctx.ast[model_id].name(), attr.span));
            }

            return;
        }
    };

    let ast_model = &ctx.ast[model_id];

    // ID attribute fields must reference only required fields.
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .map(|field| &ctx.ast[model_id][field.field_id])
        .filter(|field| !field.arity.is_required())
        .map(|field| field.name())
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

    model_data.primary_key = Some(IdAttribute {
        name,
        source_attribute: ctx.current_attribute_id(),
        mapped_name,
        fields: resolved_fields,
        source_field: None,
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

            model_attributes.primary_key = Some(IdAttribute {
                name: None,
                mapped_name,
                source_attribute: ctx.current_attribute_id(),
                fields: vec![FieldWithArgs {
                    field_id,
                    sort_order,
                    length,
                }],
                source_field: Some(field_id),
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
