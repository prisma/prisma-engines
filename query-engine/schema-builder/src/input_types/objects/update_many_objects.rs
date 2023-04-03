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
pub(crate) fn checked_update_many_input_type(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CheckedUpdateManyInput(model.clone()));

    return_cached_input!(ctx, &ident);

    let input_object = init_input_object_type(ident.clone());
    let id = ctx.cache_input_type(ident, input_object);

    let filtered_fields: Vec<_> = update_one_objects::filter_checked_update_fields(ctx, model, None)
        .into_iter()
        .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)))
        .collect();

    let field_mapper = UpdateDataInputFieldMapper::new_checked();
    field_mapper.map_all(ctx, id, &mut filtered_fields.iter());
    id
}

/// Builds "<x>UncheckedUpdateManyWithout<y>MutationInput" input object type
pub(crate) fn unchecked_update_many_input_type(
    ctx: &mut BuilderContext<'_>,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeId {
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

    let input_object = init_input_object_type(ident.clone());
    let id = ctx.cache_input_type(ident, input_object);

    let filtered_fields = update_one_objects::filter_unchecked_update_fields(ctx, model, parent_field)
        .into_iter()
        .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)));

    let field_mapper = UpdateDataInputFieldMapper::new_unchecked();
    for field in filtered_fields {
        field_mapper.map_field(ctx, id, &field);
    }
    id
}

/// Builds "<x>UpdateManyWithWhereWithout<y>Input" input object type.
/// Simple combination object of "where" and "data"
pub(crate) fn update_many_where_combination_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::UpdateManyWhereCombinationInput(
        parent_field.related_field(),
    ));

    return_cached_input!(ctx, &ident);

    let input_object = init_input_object_type(ident.clone());
    let id = ctx.cache_input_type(ident, input_object);

    let related_model = parent_field.related_model();
    let where_input_object = filter_objects::scalar_filter_object_type(ctx, &related_model, false);
    let update_types = update_many_input_types(ctx, &related_model, Some(parent_field));
    let where_field = input_field(ctx, args::WHERE, InputType::object(where_input_object), None);
    let data_field = input_field(ctx, args::DATA, update_types, None);
    ctx.db.push_input_field(id, where_field);
    ctx.db.push_input_field(id, data_field);
    id
}
