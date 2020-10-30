use super::*;

pub(crate) fn nested_upsert_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    if parent_field.is_list {
        nested_upsert_list_input_object(ctx, parent_field)
    } else {
        nested_upsert_nonlist_input_object(ctx, parent_field)
    }
}

/// Builds "<x>UpsertWithWhereUniqueNestedInput" / "<x>UpsertWithWhereUniqueWithout<y>Input" input object types.
fn nested_upsert_list_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let where_object = filter_objects::where_unique_object_type(ctx, &related_model);
    let create_types = create_objects::create_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if where_object.into_arc().is_empty() || create_types.iter().all(|typ| typ.is_empty()) {
        return None;
    }

    let type_name = format!(
        "{}UpsertWithWhereUniqueWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("where", InputType::object(where_object), None),
                input_field("update", update_types, None),
                input_field("create", create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}

/// Builds "<x>UpsertNestedInput" / "<x>UpsertWithout<y>Input" input object types.
fn nested_upsert_nonlist_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let create_types = create_objects::create_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if create_types.iter().all(|typ| typ.is_empty()) {
        return None;
    }

    let type_name = format!(
        "{}UpsertWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("update", update_types, None),
                input_field("create", create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}
