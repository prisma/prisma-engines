use crate::{
    ast::{self, WithName, WithSpan},
    context::Context,
    types::{CompositeTypeField, ModelAttributes, ScalarField},
    DatamodelError, StringId,
};

pub(super) fn model(model_attributes: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
    let mapped_name = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    model_attributes.mapped_name = Some(mapped_name);

    if let Some(existing_model_id) = ctx.mapped_model_names.insert(mapped_name, model_id) {
        let existing_model_name = ctx.ast[existing_model_id].name();
        ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
            &ctx[mapped_name],
            existing_model_name,
            ctx.ast[model_id].span(),
        ));
    }

    if let Some(existing_model_id) = ctx.names.tops.get(&mapped_name).and_then(|id| id.as_model_id()) {
        let existing_model_name = ctx.ast[existing_model_id].name();
        ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
            &ctx[mapped_name],
            existing_model_name,
            ctx.current_attribute().span,
        ));
    }
}

pub(super) fn scalar_field(
    ast_model: &ast::Model,
    ast_field: &ast::Field,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    scalar_field_data: &mut ScalarField,
    ctx: &mut Context<'_>,
) {
    let mapped_name = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    scalar_field_data.mapped_name = Some(mapped_name);

    if ctx
        .mapped_model_scalar_field_names
        .insert((model_id, mapped_name), field_id)
        .is_some()
    {
        ctx.push_error(DatamodelError::new_duplicate_field_error(
            ast_model.name(),
            ast_field.name(),
            ast_field.span(),
        ));
    }

    if let Some(field_id) = ctx.names.model_fields.get(&(model_id, mapped_name)) {
        // @map only conflicts with _scalar_ fields
        if !ctx.types.scalar_fields.contains_key(&(model_id, *field_id)) {
            return;
        }

        match ctx
            .types
            .scalar_fields
            .get(&(model_id, *field_id))
            .and_then(|sf| sf.mapped_name)
        {
            Some(name) if name != mapped_name => {}
            _ => ctx.push_error(DatamodelError::new_duplicate_field_error(
                ast_model.name(),
                ast_field.name(),
                ast_field.span(),
            )),
        }
    }
}

pub(super) fn composite_type_field(
    ct: &ast::CompositeType,
    ast_field: &ast::Field,
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &mut CompositeTypeField,
    ctx: &mut Context<'_>,
) {
    let mapped_name_id = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    field.mapped_name = Some(mapped_name_id);

    if ctx
        .mapped_composite_type_names
        .insert((ctid, mapped_name_id), field_id)
        .is_some()
    {
        ctx.push_error(DatamodelError::new_composite_type_duplicate_field_error(
            ct.name(),
            &ctx[mapped_name_id],
            ast_field.span(),
        ));
    }

    if let Some(f) = ctx.names.composite_type_fields.get(&(ctid, mapped_name_id)) {
        let other_field = &ctx.types.composite_type_fields[&(ctid, *f)];

        // We check mapped name collisions above. In this part, if the other
        // field has a mapped name, they cannot collide.
        if other_field.mapped_name.is_some() {
            return;
        }

        ctx.push_error(DatamodelError::new_composite_type_duplicate_field_error(
            ct.name(),
            ast_field.name(),
            ast_field.span(),
        ));
    }
}

pub(super) fn visit_map_attribute(ctx: &mut Context<'_>) -> Option<StringId> {
    match ctx.visit_default_arg("name").map(|value| value.as_str()) {
        Ok(Ok(name)) => return Some(ctx.interner.intern(name)),
        Err(err) => ctx.push_error(err), // not flattened for error handing legacy reasons
        Ok(Err(err)) => ctx.push_error(err),
    };

    None
}
