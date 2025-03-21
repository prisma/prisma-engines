use super::fields::data_input_mapper::*;
use super::*;
use constants::args;

pub(crate) fn update_many_input_types(
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> Vec<InputType<'_>> {
    let checked_input = InputType::object(checked_update_many_input_type(ctx, model.clone()));
    let unchecked_input = InputType::object(unchecked_update_many_input_type(ctx, model, parent_field));

    vec![checked_input, unchecked_input]
}

/// Builds "<x>UpdateManyMutationInput" input object type.
pub(crate) fn checked_update_many_input_type(ctx: &'_ QuerySchema, model: Model) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CheckedUpdateManyInput(model.clone()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(model.clone().into());
    input_object.set_fields(move || {
        let mut filtered_fields = update_one_objects::filter_checked_update_fields(ctx, &model, None)
            .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)));

        let field_mapper = UpdateDataInputFieldMapper::new_checked();
        field_mapper.map_all(ctx, &mut filtered_fields)
    });
    input_object
}

/// Builds "<x>UncheckedUpdateManyWithout<y>MutationInput" input object type
pub(crate) fn unchecked_update_many_input_type(
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::UncheckedUpdateManyInput(
        model.clone(),
        parent_field.clone().map(|pf| pf.related_field()),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(model.clone().into());
    input_object.set_fields(move || {
        let mut filtered_fields =
            update_one_objects::filter_unchecked_update_fields(ctx, &model, parent_field.as_ref())
                .filter(|field| matches!(field, ModelField::Scalar(_) | ModelField::Composite(_)));

        let field_mapper = UpdateDataInputFieldMapper::new_unchecked();
        field_mapper.map_all(ctx, &mut filtered_fields)
    });
    input_object
}

/// Builds "<x>UpdateManyWithWhereWithout<y>Input" input object type.
/// Simple combination object of "where" and "data"
pub(crate) fn update_many_where_combination_object(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::UpdateManyWhereCombinationInput(
        parent_field.related_field(),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(parent_field.related_model().into());
    input_object.set_fields(move || {
        let related_model = parent_field.related_model();
        let where_input_object = filter_objects::scalar_filter_object_type(ctx, related_model.clone(), false);
        let update_types = update_many_input_types(ctx, related_model, Some(parent_field));

        vec![
            input_field(args::WHERE, vec![InputType::object(where_input_object)], None),
            input_field(args::DATA, update_types, None),
        ]
    });
    input_object
}
