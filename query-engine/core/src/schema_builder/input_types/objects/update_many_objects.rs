use super::*;

pub(crate) fn update_many_input_types(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let checked_input = InputType::object(checked_update_many_input_type(ctx, model));

    if feature_flags::get().uncheckedScalarInputs {
        let unchecked_input = InputType::object(unchecked_update_many_input_type(ctx, model, parent_field));

        // If the inputs are equal, only use one.
        if checked_input == unchecked_input {
            vec![checked_input]
        } else {
            vec![checked_input, unchecked_input]
        }
    } else {
        vec![checked_input]
    }
}

/// Builds "<x>UpdateManyMutationInput" input object type.
pub(crate) fn checked_update_many_input_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let object_name = format!("{}UpdateManyMutationInput", model.name);
    return_cached_input!(ctx, &object_name);

    let input_fields = update_one_objects::scalar_input_fields_for_checked_update(ctx, model);
    let input_object = Arc::new(input_object_type(object_name.clone(), input_fields));

    ctx.cache_input_type(object_name, input_object.clone());
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateManyMutationInput" input object type.
pub(crate) fn unchecked_update_many_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let object_name = format!("{}UncheckedUpdateManyMutationInput", model.name);
    return_cached_input!(ctx, &object_name);

    let input_fields = update_one_objects::scalar_input_fields_for_unchecked_update(ctx, model, parent_field);
    let input_object = Arc::new(input_object_type(object_name.clone(), input_fields));

    ctx.cache_input_type(object_name, input_object.clone());
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyWithWhereNestedInput" input object type.
/// Simple combination object of "where" and "data".
pub(crate) fn update_many_where_combination_object(
    ctx: &mut BuilderContext,
    field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = field.related_model();
    let name = format!("{}UpdateManyWithWhereNestedInput", related_model.name);

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    let where_input_object = filter_objects::scalar_filter_object_type(ctx, &related_model);
    let update_types = update_many_input_types(ctx, &related_model, Some(field));

    input_object.set_fields(vec![
        input_field("where", InputType::object(where_input_object), None),
        input_field("data", update_types, None),
    ]);

    Arc::downgrade(&input_object)
}
