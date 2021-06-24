use super::Context;
use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::helpers::{Arguments, ValueValidator},
};

/// @@id on models
pub(super) fn model_id(id_args: &mut Arguments<'_>, model_id: ast::TopId, ctx: &mut Context<'_, '_>) {
    let resolved_fields = match id_args.default_arg("fields") {
        Ok(value) => match resolve_field_array(&value, model_id, ctx) {
            Ok(fields) => fields,
            Err(Some(unresolved_fields)) => {
                return ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The multi field id declaration refers to the unknown fields {}.",
                        unresolved_fields.join(", "),
                    ),
                    ctx.db.ast[model_id].name(),
                    value.span(),
                ));
            }
            Err(None) => return,
        },
        Err(err) => {
            return ctx.push_error(err);
        }
    };

    let model_name = ctx.db.ast()[model_id].name();

    // ID fields must reference only required (1) scalar (2) fields.

    // (1)
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .map(|field_id| &ctx.db.ast[model_id].as_model().unwrap()[*field_id])
        .filter(|field| !matches!(field.arity, ast::FieldArity::Required))
        .map(|field| field.name.name.as_str())
        .collect();

    if !fields_that_are_not_required.is_empty() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!(
                "The id definition refers to the optional fields {}. ID definitions must reference only required fields.",
                fields_that_are_not_required.join(", ")
            ),
            model_name,
            id_args.span(),
        ))
    }

    // (2)
    let referenced_relation_fields: Vec<&str> = resolved_fields
        .iter()
        .filter(|field_id| ctx.db.relations.relation_fields.contains_key(&(model_id, **field_id)))
        .map(|field_id| ctx.db.ast[model_id].as_model().unwrap()[*field_id].name.name.as_str())
        .collect();

    if !referenced_relation_fields.is_empty() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!(
                "The id definition refers to the relation fields {}. Id definitions must reference only scalar fields.",
                referenced_relation_fields.join(", ")
            ),
            model_name,
            id_args.span(),
        ))
    }

    ctx.db.ids.insert(model_id, resolved_fields);
}

/// Takes an attribute argument, validates it as an array of constants, then
/// resolves  the constant as field names on the model.
fn resolve_field_array(
    values: &ValueValidator,
    model_id: ast::TopId,
    ctx: &mut Context<'_, '_>,
) -> Result<Vec<ast::FieldId>, Option<Vec<String>>> {
    let constant_array = match values.as_constant_array() {
        Ok(values) => values,
        Err(err) => {
            ctx.push_error(err);
            return Err(None);
        }
    };

    let mut field_ids = Vec::with_capacity(constant_array.len());
    let mut unresolvable_fields = Vec::new();

    for field_name in constant_array {
        if let Some(field_id) = ctx.db.names.model_fields.get(&(model_id, field_name.as_str())) {
            field_ids.push(*field_id);
        } else {
            unresolvable_fields.push(field_name);
        }
    }

    if !unresolvable_fields.is_empty() {
        Err(Some(unresolvable_fields))
    } else {
        Ok(field_ids)
    }
}
