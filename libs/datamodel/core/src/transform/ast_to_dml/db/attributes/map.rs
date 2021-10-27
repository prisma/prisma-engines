use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::db::{
        context::{Arguments, Context},
        types::{CompositeTypeField, ModelAttributes},
        ScalarField,
    },
};

pub(super) fn model<'ast>(
    model_attributes: &mut ModelAttributes<'ast>,
    model_id: ast::ModelId,
    args: &mut Arguments<'ast>,
    ctx: &mut Context<'ast>,
) {
    let mapped_name = match visit_map_attribute(args, ctx) {
        Some(name) => name,
        None => return,
    };

    model_attributes.mapped_name = Some(mapped_name);

    if let Some(existing_model_id) = ctx.mapped_model_names.insert(mapped_name, model_id) {
        let existing_model_name = ctx.db.ast[existing_model_id].name();
        ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
            mapped_name.to_owned(),
            existing_model_name.to_owned(),
            ctx.db.ast[model_id].span,
        ));
    }

    if let Some(existing_model_id) = ctx.db.names.tops.get(mapped_name).and_then(|id| id.as_model_id()) {
        let existing_model_name = ctx.db.ast[existing_model_id].name();
        ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
            mapped_name.to_owned(),
            existing_model_name.to_owned(),
            args.span(),
        ));
    }
}

pub(super) fn scalar_field<'ast>(
    ast_model: &ast::Model,
    ast_field: &ast::Field,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    scalar_field_data: &mut ScalarField<'ast>,
    map_args: &mut Arguments<'ast>,
    ctx: &mut Context<'ast>,
) {
    let mapped_name = match visit_map_attribute(map_args, ctx) {
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
            &ast_field.name.name,
            ast_field.span,
        ));
    }

    if let Some(field_id) = ctx.db.names.model_fields.get(&(model_id, mapped_name)) {
        // @map only conflicts with _scalar_ fields
        if !ctx.db.types.scalar_fields.contains_key(&(model_id, *field_id)) {
            return;
        }

        match ctx
            .db
            .types
            .scalar_fields
            .get(&(model_id, *field_id))
            .and_then(|sf| sf.mapped_name)
        {
            Some(name) if name != mapped_name => {}
            _ => ctx.push_error(DatamodelError::new_duplicate_field_error(
                &ast_model.name.name,
                &ast_field.name.name,
                ast_field.span,
            )),
        }
    }
}

pub(super) fn composite_type_field<'ast>(
    ct: &'ast ast::CompositeType,
    ast_field: &'ast ast::Field,
    ctid: ast::CompositeTypeId,
    field_id: ast::FieldId,
    field: &mut CompositeTypeField<'ast>,
    map_args: &mut Arguments<'ast>,
    ctx: &mut Context<'ast>,
) {
    let mapped_name = match visit_map_attribute(map_args, ctx) {
        Some(name) => name,
        None => return,
    };

    field.mapped_name = Some(mapped_name);

    if ctx
        .mapped_composite_type_names
        .insert((ctid, mapped_name), field_id)
        .is_some()
    {
        ctx.push_error(DatamodelError::new_duplicate_field_error(
            &ct.name.name,
            &ast_field.name.name,
            ast_field.span,
        ));
    }

    if ctx.db.names.composite_type_fields.contains_key(&(ctid, mapped_name)) {
        ctx.push_error(DatamodelError::new_duplicate_field_error(
            &ct.name.name,
            &ast_field.name.name,
            ast_field.span,
        ));
    }
}

pub(super) fn visit_map_attribute<'ast>(map_args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<&'ast str> {
    match map_args.default_arg("name").map(|value| value.as_str()) {
        Ok(Ok(name)) => return Some(name),
        Err(err) => ctx.push_error(err), // not flattened for error handing legacy reasons
        Ok(Err(err)) => ctx.push_error(map_args.new_attribute_validation_error(&err.to_string())),
    };

    None
}
