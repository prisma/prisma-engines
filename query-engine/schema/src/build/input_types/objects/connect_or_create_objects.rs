use super::*;
use constants::args;
use mutations::create_one;

/// Builds "<x>CreateOrConnectNestedInput" input object types.
pub(crate) fn nested_connect_or_create_input_object(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputObjectType<'_>> {
    let related_model = parent_field.related_model();
    let ident = Identifier::new_prisma(format!(
        "{}CreateOrConnectWithout{}Input",
        related_model.name(),
        capitalize(parent_field.related_field().name())
    ));

    let where_object = filter_objects::where_unique_object_type(ctx, related_model.clone());

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(related_model.clone().into());
    input_object.set_fields(move || {
        let create_types = create_one::create_one_input_types(ctx, related_model.clone(), Some(parent_field.clone()));
        vec![
            input_field(args::WHERE, vec![InputType::object(where_object.clone())], None),
            input_field(args::CREATE, create_types, None),
        ]
    });

    Some(input_object)
}
