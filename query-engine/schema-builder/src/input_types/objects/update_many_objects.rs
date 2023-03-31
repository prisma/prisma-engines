use super::fields::data_input_mapper::*;
use super::*;
use constants::args;

pub(crate) fn update_many_input_types(
    ctx: &mut BuilderContext<'_>,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let checked_input = InputType::object(checked_update_many_input_type(ctx, model));
    let unchecked_input = InputType::object(unchecked_update_many_input_type(ctx, model, parent_field));

    // If the inputs are equal, only use one.
    if checked_input == unchecked_input {
        vec![checked_input]
    } else {
        vec![checked_input, unchecked_input]
    }
}

/// Builds "<x>UpdateManyMutationInput" input object type.
pub(crate) fn checked_update_many_input_type(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(IdentifierType::CheckedUpdateManyInput(model.clone()));

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields: Vec<_> = update_one_objects::filter_checked_update_fields(ctx, model, None)
        .into_iter()
        .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)))
        .collect();

    let field_mapper = UpdateDataInputFieldMapper::new_checked();
    let input_fields = field_mapper.map_all(ctx, &filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateManyWithout<y>MutationInput" input object type
pub(crate) fn unchecked_update_many_input_type(
    ctx: &mut BuilderContext<'_>,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    // TODO: This leads to conflicting type names.
    // TODO: See https://github.com/prisma/prisma/issues/18534 for further details.
    let name = match parent_field {
        Some(pf) => format!(
            "{}UncheckedUpdateManyWithout{}Input",
            model.name(),
            capitalize(pf.name())
        ),
        _ => format!("{}UncheckedUpdateManyInput", model.name()),
    };

    let ident = Identifier::new_prisma(name);

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields: Vec<_> = update_one_objects::filter_unchecked_update_fields(ctx, model, parent_field)
        .into_iter()
        .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)))
        .collect();

    let field_mapper = UpdateDataInputFieldMapper::new_unchecked();
    let input_fields = field_mapper.map_all(ctx, &filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyWithWhereWithout<y>Input" input object type.
/// Simple combination object of "where" and "data"
pub(crate) fn update_many_where_combination_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(IdentifierType::UpdateManyWhereCombinationInput(
        parent_field.related_field(),
    ));

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let related_model = parent_field.related_model();
    let where_input_object = filter_objects::scalar_filter_object_type(ctx, &related_model, false);
    let update_types = update_many_input_types(ctx, &related_model, Some(parent_field));

    input_object.set_fields(vec![
        input_field(ctx, args::WHERE, InputType::object(where_input_object), None),
        input_field(ctx, args::DATA, update_types, None),
    ]);

    Arc::downgrade(&input_object)
}
