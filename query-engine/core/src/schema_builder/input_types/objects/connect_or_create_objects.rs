use super::*;

/// Builds "<x>CreateOrConnectNestedInput" input object types.
pub(crate) fn nested_connect_or_create_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let where_object = filter_objects::where_unique_object_type(ctx, &related_model);

    if where_object.into_arc().is_empty() {
        return None;
    }

    let ident = Identifier::new(
        format!(
            "{}CreateOrConnectWithout{}Input",
            related_model.name,
            parent_field.related_field().name
        ),
        PRISMA_NAMESPACE,
    );

    let create_types = create_objects::create_input_types(ctx, &related_model, Some(parent_field));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = Arc::new(init_input_object_type(ident.clone()));
            ctx.cache_input_type(ident, input_object.clone());

            let fields = vec![
                input_field("where", InputType::object(where_object), None),
                input_field("create", create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}
