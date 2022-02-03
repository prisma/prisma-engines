use crate::{
    ast::{self, WithName},
    context::{Arguments, AttributeContext, Context},
    types::{CompositeTypeField, ModelAttributes, ScalarField},
    DatamodelError,
};

pub(super) fn model(
    model_attributes: &mut ModelAttributes,
    model_id: ast::ModelId,
    map: &mut AttributeContext<'_, '_>,
) {
    let mapped_name = match visit_map_attribute(map) {
        Some(name) => name,
        None => return,
    };

    model_attributes.mapped_name = Some(mapped_name);

    if let Some(existing_model_id) = map.ctx.mapped_model_names.insert(mapped_name, model_id) {
        let existing_model_name = map.ctx.db.ast[existing_model_id].name();
        map.ctx
            .push_error(DatamodelError::new_duplicate_model_database_name_error(
                mapped_name.to_owned(),
                existing_model_name.to_owned(),
                map.ctx.db.ast[model_id].span,
            ));
    }

    if let Some(existing_model_id) = map.ctx.db.names.tops.get(mapped_name).and_then(|id| id.as_model_id()) {
        let existing_model_name = map.ctx.db.ast[existing_model_id].name();
        map.ctx
            .push_error(DatamodelError::new_duplicate_model_database_name_error(
                mapped_name.to_owned(),
                existing_model_name.to_owned(),
                map.span(),
            ));
    }
}

pub(super) fn scalar_field(
    ast_model: &ast::Model,
    ast_field: &ast::Field,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    scalar_field_data: &mut ScalarField,
    map: &mut AttributeContext<'_, '_>,
) {
    let mapped_name = match visit_map_attribute(map) {
        Some(name) => name,
        None => return,
    };

    scalar_field_data.mapped_name = Some(mapped_name);

    if map
        .ctx
        .mapped_model_scalar_field_names
        .insert((model_id, mapped_name), field_id)
        .is_some()
    {
        map.ctx.push_error(DatamodelError::new_duplicate_field_error(
            ast_model.name(),
            &ast_field.name.name,
            ast_field.span,
        ));
    }

    if let Some(field_id) = map.ctx.db.names.model_fields.get(&(model_id, mapped_name)) {
        // @map only conflicts with _scalar_ fields
        if !map.ctx.db.types.scalar_fields.contains_key(&(model_id, *field_id)) {
            return;
        }

        match map
            .ctx
            .db
            .types
            .scalar_fields
            .get(&(model_id, *field_id))
            .and_then(|sf| sf.mapped_name)
        {
            Some(name) if name != mapped_name => {}
            _ => map.ctx.push_error(DatamodelError::new_duplicate_field_error(
                &ast_model.name.name,
                &ast_field.name.name,
                ast_field.span,
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
    map: &mut AttributeContext<'_, '_>,
) {
    let mapped_name = match visit_map_attribute(map) {
        Some(name) => name,
        None => return,
    };

    field.mapped_name = Some(mapped_name);

    if map
        .ctx
        .mapped_composite_type_names
        .insert((ctid, mapped_name), field_id)
        .is_some()
    {
        map.ctx.push_error(DatamodelError::new_duplicate_field_error(
            &ct.name.name,
            &ast_field.name.name,
            ast_field.span,
        ));
    }

    if map
        .ctx
        .db
        .names
        .composite_type_fields
        .contains_key(&(ctid, mapped_name))
    {
        map.ctx.push_error(DatamodelError::new_duplicate_field_error(
            &ct.name.name,
            &ast_field.name.name,
            ast_field.span,
        ));
    }
}

pub(super) fn visit_map_attribute<'db>(map: &mut AttributeContext<'_, 'db>) -> Option<&'db str> {
    match map.default_arg("name").map(|value| value.as_str()) {
        Ok(Ok(name)) => return Some(name),
        Err(err) => map.ctx.push_error(err), // not flattened for error handing legacy reasons
        Ok(Err(err)) => map.ctx.push_error(map.new_attribute_validation_error(&err.to_string())),
    };

    None
}
