use crate::constants::args;
use crate::input_types::fields::arguments::where_argument;
use crate::mutations::create_one;

use super::*;

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

    if ctx.db.input_object_fields(where_object).next().is_none() || create_types.iter().all(|typ| typ.is_empty(&ctx.db))
    {
        return None;
    }

    let ident = Identifier::new_prisma(IdentifierType::NestedUpsertManyInput(parent_field.related_field()));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = init_input_object_type(ident.clone());
            let id = ctx.cache_input_type(ident, input_object);
            let where_field = input_field(ctx, args::WHERE, InputType::object(where_object), None);
            let update_field = input_field(ctx, args::UPDATE, update_types, None);
            let create_field = input_field(ctx, args::CREATE, create_types, None);
            ctx.db.push_input_field(id, where_field);
            ctx.db.push_input_field(id, update_field);
            ctx.db.push_input_field(id, create_field);
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
            let update_field = input_field(ctx, args::UPDATE, update_types, None);
            let create_field = input_field(ctx, args::CREATE, create_types, None);
            ctx.db.push_input_field(id, update_field);
            ctx.db.push_input_field(id, create_field);

            if ctx.has_feature(PreviewFeature::ExtendedWhereUnique) {
                let where_arg = where_argument(ctx, &related_model);
                ctx.db.push_input_field(id, where_arg);
            }

            Some(id)
        }
        x => x,
    }
}
