use super::*;
use constants::args;
use input_types::fields::arguments::where_argument;
use mutations::create_one;

pub(crate) fn nested_upsert_input_object(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputObjectType<'_>> {
    if parent_field.is_list() {
        nested_upsert_list_input_object(ctx, parent_field)
    } else {
        nested_upsert_nonlist_input_object(ctx, parent_field)
    }
}

/// Builds "<x>UpsertWithWhereUniqueNestedInput" / "<x>UpsertWithWhereUniqueWithout<y>Input" input object types.
fn nested_upsert_list_input_object(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputObjectType<'_>> {
    let related_model = parent_field.related_model();
    let where_object = filter_objects::where_unique_object_type(ctx, related_model.clone());
    let create_types = create_one::create_one_input_types(ctx, related_model.clone(), Some(parent_field.clone()));
    let update_types = update_one_objects::update_one_input_types(ctx, related_model, Some(parent_field.clone()));

    let ident = Identifier::new_prisma(IdentifierType::NestedUpsertManyInput(parent_field.related_field()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(parent_field.related_model().into());
    input_object.set_fields(move || {
        vec![
            input_field(args::WHERE, vec![InputType::object(where_object.clone())], None),
            input_field(args::UPDATE, update_types.clone(), None),
            input_field(args::CREATE, create_types.clone(), None),
        ]
    });

    Some(input_object)
}

/// Builds "<x>UpsertNestedInput" / "<x>UpsertWithout<y>Input" input object types.
fn nested_upsert_nonlist_input_object(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputObjectType<'_>> {
    let related_model = parent_field.related_model();
    let create_types = create_one::create_one_input_types(ctx, related_model.clone(), Some(parent_field.clone()));

    let ident = Identifier::new_prisma(IdentifierType::NestedUpsertOneInput(parent_field.related_field()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(related_model.clone().into());
    input_object.set_fields(move || {
        let update_types =
            update_one_objects::update_one_input_types(ctx, related_model.clone(), Some(parent_field.clone()));

        let fields = vec![
            input_field(args::UPDATE, update_types, None),
            input_field(args::CREATE, create_types.clone(), None),
            where_argument(ctx, &related_model),
        ];

        fields
    });
    Some(input_object)
}
