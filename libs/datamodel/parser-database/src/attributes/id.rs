use super::FieldResolutionError;
use crate::{
    ast::{self, WithName, WithSpan},
    attributes::resolve_field_array_with_args,
    context::{Arguments, Context},
    types::{FieldWithArgs, IdAttribute, ModelAttributes, SortOrder},
    DatamodelError,
};

/// @@id on models
pub(super) fn model<'ast>(
    args: &mut Arguments<'ast>,
    model_data: &mut ModelAttributes<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let fields = match args.default_arg("fields") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let resolved_fields = match resolve_field_array_with_args(&fields, args.span(), model_id, ctx) {
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
                    ctx.db.ast[model_id].name(),
                    fields.span(),
                ));
            }

            if !relation_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(&format!("The id definition refers to the relation fields {}. ID definitions must reference only scalar fields.", relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", ")), ctx.db.ast[model_id].name(), args.span()));
            }

            return;
        }
    };

    let ast_model = &ctx.db.ast[model_id];

    // ID attribute fields must reference only required fields.
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .map(|field| &ctx.db.ast[model_id][field.field_id])
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
            args.span(),
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
        let mapped_name = primary_key_mapped_name(args, ctx);
        let name = super::get_name_argument(args, ctx);

        if let Some(name) = name {
            super::validate_client_name(args.span(), &ast_model.name.name, name, "@@id", ctx);
        }

        (name, mapped_name)
    };

    model_data.primary_key = Some(IdAttribute {
        name,
        source_attribute: args.attribute().0,
        mapped_name,
        fields: resolved_fields,
        source_field: None,
    });
}
pub(super) fn field<'ast>(
    ast_model: &'ast ast::Model,
    field_id: ast::FieldId,
    model_attributes: &mut ModelAttributes<'ast>,
    args: &mut Arguments<'ast>,
    ctx: &mut Context<'ast>,
) {
    match model_attributes.primary_key {
        Some(_) => ctx.push_error(DatamodelError::new_model_validation_error(
            "At most one field must be marked as the id field with the `@id` attribute.",
            ast_model.name(),
            ast_model.span,
        )),
        None => {
            let mapped_name = primary_key_mapped_name(args, ctx);

            let length = match args.optional_arg("length").map(|length| length.as_int()) {
                Some(Ok(length)) => Some(length as u32),
                Some(Err(err)) => {
                    ctx.push_error(err);
                    None
                }
                None => None,
            };

            let sort_order = match args.optional_arg("sort").map(|sort| sort.as_constant_literal()) {
                Some(Ok("Desc")) => Some(SortOrder::Desc),
                Some(Ok("Asc")) => Some(SortOrder::Asc),
                Some(Ok(other)) => {
                    ctx.push_error(args.new_attribute_validation_error(&format!(
                        "The `sort` argument can only be `Asc` or `Desc` you provided: {}.",
                        other
                    )));
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
                source_attribute: args.attribute().0,
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
            *pk.source_attribute.span(),
        ))
    }
}

fn primary_key_mapped_name<'ast>(args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<&'ast str> {
    let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
            None
        }
        Some(Ok(name)) => Some(name),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    mapped_name
}
