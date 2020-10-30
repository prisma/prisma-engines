use super::*;

/// Builds "<x>UpdateManyMutationInput" input object type.
pub(crate) fn update_many_input_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let object_name = format!("{}UpdateManyMutationInput", model.name);
    return_cached_input!(ctx, &object_name);

    let input_fields = update_one_objects::scalar_input_fields_for_checked_update(ctx, model);
    let input_object = Arc::new(input_object_type(object_name.clone(), input_fields));

    ctx.cache_input_type(object_name, input_object.clone());
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyWithWhereNestedInput" input object type.
pub(crate) fn nested_update_many_input_object(
    ctx: &mut BuilderContext,
    field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    if field.is_list {
        let related_model = field.related_model();
        let type_name = format!("{}UpdateManyWithWhereNestedInput", related_model.name);

        match ctx.get_input_type(&type_name) {
            None => {
                let data_input_object = nested_update_many_data(ctx, field);
                let input_object = Arc::new(init_input_object_type(type_name.clone()));
                ctx.cache_input_type(type_name, input_object.clone());

                let where_input_object = filter_objects::scalar_filter_object_type(ctx, &related_model);

                input_object.set_fields(vec![
                    input_field("where", InputType::object(where_input_object), None),
                    input_field("data", InputType::object(data_input_object), None),
                ]);

                Some(Arc::downgrade(&input_object))
            }
            x => x,
        }
    } else {
        None
    }
}

/// Builds "<x>UpdateManyDataInput" input object type.
fn nested_update_many_data(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let type_name = format!("{}UpdateManyDataInput", related_model.name);

    return_cached_input!(ctx, &type_name);

    let input_object = Arc::new(init_input_object_type(type_name.clone()));
    ctx.cache_input_type(type_name, input_object.clone());

    let fields = update_one_objects::scalar_input_fields_for_checked_update(ctx, &related_model);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}
