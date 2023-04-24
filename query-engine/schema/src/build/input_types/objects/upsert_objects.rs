use super::*;
use constants::args;
use input_types::fields::arguments::where_argument;
use mutations::create_one;

pub(crate) fn nested_upsert_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeId> {
    if parent_field.is_list() {
        nested_upsert_list_input_object(ctx, parent_field)
    } else {
        nested_upsert_nonlist_input_object(ctx, parent_field)
    }
}

/// Builds "<x>UpsertWithWhereUniqueNestedInput" / "<x>UpsertWithWhereUniqueWithout<y>Input" input object types.
fn nested_upsert_list_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeId> {
    let related_model = parent_field.related_model();
    let where_object = filter_objects::where_unique_object_type(ctx, &related_model);
    let create_types = create_one::create_one_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if ctx.db[where_object].is_empty() || create_types.iter().all(|typ| typ.is_empty(&ctx.db)) {
        return None;
    }

    let ident = Identifier::new_prisma(IdentifierType::NestedUpsertManyInput(parent_field.related_field()));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = init_input_object_type(ident.clone());
            let id = ctx.cache_input_type(ident, input_object);

            let fields = vec![
                input_field(ctx, args::WHERE, InputType::object(where_object), None),
                input_field(ctx, args::UPDATE, update_types, None),
                input_field(ctx, args::CREATE, create_types, None),
            ];

            ctx.db[id].set_fields(fields);
            Some(id)
        }
        x => x,
    }
}

/// Builds "<x>UpsertNestedInput" / "<x>UpsertWithout<y>Input" input object types.
fn nested_upsert_nonlist_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeId> {
    let related_model = parent_field.related_model();
    let create_types = create_one::create_one_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if create_types.iter().all(|typ| typ.is_empty(&ctx.db)) {
        return None;
    }

    let ident = Identifier::new_prisma(IdentifierType::NestedUpsertOneInput(parent_field.related_field()));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = init_input_object_type(ident.clone());
            let id = ctx.cache_input_type(ident, input_object);

            let mut fields = vec![
                input_field(ctx, args::UPDATE, update_types, None),
                input_field(ctx, args::CREATE, create_types, None),
            ];

            if ctx.has_feature(PreviewFeature::ExtendedWhereUnique) {
                fields.push(where_argument(ctx, &related_model));
            }

            ctx.db[id].set_fields(fields);
            Some(id)
        }
        x => x,
    }
}
