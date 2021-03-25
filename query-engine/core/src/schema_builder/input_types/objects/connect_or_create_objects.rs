use super::*;
use constants::inputs::args;

/// Builds "<x>CreateOrConnectNestedInput" input object types.
#[tracing::instrument(skip(ctx, parent_field))]
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
            capitalize(parent_field.related_field().name.as_str())
        ),
        PRISMA_NAMESPACE,
    );

    let create_types = create_one_objects::create_one_input_types(ctx, &related_model, Some(parent_field));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = Arc::new(init_input_object_type(ident.clone()));
            ctx.cache_input_type(ident, input_object.clone());

            let fields = vec![
                input_field(args::WHERE, InputType::object(where_object), None),
                input_field(args::CREATE, create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}
