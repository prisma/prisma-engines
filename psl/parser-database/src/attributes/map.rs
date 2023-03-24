use crate::{
    ast::{self, WithName, WithSpan},
    coerce,
    context::Context,
    types::ModelAttributes,
    DatamodelError, ScalarFieldId, StringId,
};

pub(super) fn model(model_attributes: &mut ModelAttributes, ctx: &mut Context<'_>) {
    let mapped_name = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    model_attributes.mapped_name = Some(mapped_name);
}

pub(super) fn scalar_field(
    sfid: ScalarFieldId,
    ast_model: &ast::Model,
    ast_field: &ast::Field,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ctx: &mut Context<'_>,
) {
    let mapped_name = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    ctx.types[sfid].mapped_name = Some(mapped_name);

    if ctx
        .mapped_model_scalar_field_names
        .insert((model_id, mapped_name), field_id)
        .is_some()
    {
        ctx.push_error(DatamodelError::new_duplicate_field_error(
            ast_model.name(),
            ast_field.name(),
            if ast_model.is_view() { "view" } else { "model" },
            ast_field.span(),
        ));
    }

    if let Some(dup_field_id) = ctx.names.model_fields.get(&(model_id, mapped_name)) {
        match ctx
            .types
            .range_model_scalar_fields(model_id)
            // Do not compare field to itself
            .filter(|(_, sf)| sf.field_id != field_id)
            // Find the field with the given mapped name.
            .find(|(_, sf)| sf.field_id == *dup_field_id)
            .map(|(_, sf)| sf.mapped_name)
        {
            // @map only conflicts with _scalar_ fields
            None => return,
            Some(Some(sf_mapped_name)) if sf_mapped_name != mapped_name => return,
            Some(_) => {}
        }

        ctx.push_error(DatamodelError::new_duplicate_field_error(
            ast_model.name(),
            ast_field.name(),
            if ast_model.is_view() { "view" } else { "model" },
            ast_field.span(),
        ))
    }
}

pub(super) fn composite_type_field(
    ct: &ast::CompositeType,
    ast_field: &ast::Field,
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    ctx: &mut Context<'_>,
) {
    let mapped_name_id = match visit_map_attribute(ctx) {
        Some(name) => name,
        None => return,
    };

    {
        let mut field = ctx.types.composite_type_fields.get_mut(&(ctid, field_id)).unwrap();
        field.mapped_name = Some(mapped_name_id);
    }

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
    match ctx
        .visit_default_arg("name")
        .map(|value| coerce::string(value, ctx.diagnostics))
    {
        Ok(Some(name)) => return Some(ctx.interner.intern(name)),
        Err(err) => ctx.push_error(err), // not flattened for error handing legacy reasons
        Ok(None) => (),
    };

    None
}
