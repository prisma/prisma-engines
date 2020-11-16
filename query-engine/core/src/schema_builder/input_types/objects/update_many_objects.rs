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
    let ident = Identifier::new(format!("{}UpdateManyMutationInput", model.name), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_fields = update_one_objects::scalar_input_fields_for_checked_update(ctx, model);
    let input_object = Arc::new(input_object_type(ident.clone(), input_fields));

    ctx.cache_input_type(ident, input_object.clone());
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateManyWithout<y>MutationInput" input object type.
pub(crate) fn unchecked_update_many_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!(
            "{}UncheckedUpdateManyWithout{}Input",
            model.name,
            capitalize(f.related_field().name.as_str())
        ),
        _ => format!("{}UncheckedUpdateManyInput", model.name),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_fields = update_one_objects::scalar_input_fields_for_unchecked_update(ctx, model, parent_field);
    let input_object = Arc::new(input_object_type(ident.clone(), input_fields));

    ctx.cache_input_type(ident, input_object.clone());
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyWithWhereWithout<y>Input" input object type.
/// Simple combination object of "where" and "data".
pub(crate) fn update_many_where_combination_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let name = format!(
        "{}UpdateManyWithWhereWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let where_input_object = filter_objects::scalar_filter_object_type(ctx, &related_model);
    let update_types = update_many_input_types(ctx, &related_model, Some(parent_field));

    input_object.set_fields(vec![
        input_field("where", InputType::object(where_input_object), None),
        input_field("data", update_types, None),
    ]);

    Arc::downgrade(&input_object)
}
