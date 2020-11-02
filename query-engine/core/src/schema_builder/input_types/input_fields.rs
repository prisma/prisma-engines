use super::*;
use prisma_models::dml::DefaultValue;

pub(crate) fn filter_input_field(ctx: &mut BuilderContext, field: &ModelField) -> InputField {
    let types = field_filter_types::get_field_filter_types(ctx, field);
    input_field(field.name().to_owned(), types, None).optional()
}

pub(crate) fn nested_create_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> InputField {
    let create_types = create_objects::create_input_types(ctx, &field.related_model(), Some(field));
    let types: Vec<InputType> = create_types
        .into_iter()
        .flat_map(|typ| list_union_type(typ, field.is_list))
        .collect();

    input_field("create", types, None).optional()
}

pub(crate) fn nested_connect_or_create_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    connect_or_create_objects::nested_connect_or_create_input_object(ctx, field).map(|input_object_type| {
        input_field(
            "connectOrCreate",
            list_union_object_type(input_object_type, field.is_list),
            None,
        )
        .optional()
    })
}

/// Builds "upsert" field for nested updates (on relation fields).
pub(crate) fn nested_upsert_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    upsert_objects::nested_upsert_input_object(ctx, field).map(|input_object_type| {
        input_field("upsert", list_union_object_type(input_object_type, field.is_list), None).optional()
    })
}

/// Builds "deleteMany" field for nested updates (on relation fields).
pub(crate) fn nested_delete_many_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    if field.is_list {
        let input_object = filter_objects::scalar_filter_object_type(ctx, &field.related_model());
        let input_type = InputType::object(input_object);

        Some(
            input_field(
                "deleteMany",
                vec![input_type.clone(), InputType::list(input_type)],
                None,
            )
            .optional(),
        )
    } else {
        None
    }
}

/// Builds "updateMany" field for nested updates (on relation fields).
pub(crate) fn nested_update_many_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    if parent_field.is_list {
        let input_type = update_many_objects::update_many_where_combination_object(ctx, parent_field);

        Some(
            input_field(
                "updateMany",
                list_union_object_type(input_type, true), //vec![input_type.clone(), InputType::list(input_type)],
                None,
            )
            .optional(),
        )
    } else {
        None
    }
}

/// Builds "set" field for nested updates (on relation fields).
pub(crate) fn nested_set_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    match (field.related_model().is_embedded, field.is_list) {
        (true, _) => None,
        (false, true) => Some(where_input_field(ctx, "set", field)),
        (false, false) => None,
    }
}

/// Builds "disconnect" field for nested updates (on relation fields).
pub(crate) fn nested_disconnect_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    match (field.related_model().is_embedded, field.is_list, field.is_required) {
        (true, _, _) => None,
        (false, true, _) => Some(where_input_field(ctx, "disconnect", field)),
        (false, false, false) => Some(input_field("disconnect", InputType::boolean(), None).optional()),
        (false, false, true) => None,
    }
}

/// Builds "delete" field for nested updates (on relation fields).
pub(crate) fn nested_delete_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    match (field.is_list, field.is_required) {
        (true, _) => Some(where_input_field(ctx, "delete", field)),
        (false, false) => Some(input_field("delete", InputType::boolean(), None).optional()),
        (false, true) => None,
    }
}

/// Builds the "connect" input field for a relation.
pub(crate) fn nested_connect_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    if field.related_model().is_embedded {
        None
    } else {
        Some(where_input_field(ctx, "connect", field))
    }
}

pub(crate) fn nested_update_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> InputField {
    let update_one_types = update_one_objects::update_one_input_types(ctx, &field.related_model(), Some(field));

    let update_types = if field.is_list {
        let list_object_type = update_one_objects::update_one_where_combination_object(ctx, update_one_types, field);
        list_union_object_type(list_object_type, true)
    } else {
        update_one_types
    };

    input_field("update", update_types, None).optional()
}

/// Builds scalar input fields using the mapper and the given, prefiltered, scalar fields.
/// The mapper is responsible for mapping the fields to input types.
pub(crate) fn scalar_input_fields<T, F>(
    ctx: &mut BuilderContext,
    model_name: String,
    input_object_name: T,
    prefiltered_fields: Vec<ScalarFieldRef>,
    non_list_field_mapper: F,
    with_defaults: bool,
) -> Vec<InputField>
where
    T: Into<String>,
    F: Fn(&mut BuilderContext, ScalarFieldRef, Option<DefaultValue>) -> InputField,
{
    let input_object_name = input_object_name.into();
    let mut non_list_fields: Vec<InputField> = prefiltered_fields
        .iter()
        .filter(|f| !f.is_list)
        .map(|f| {
            let default = if with_defaults { f.default_value.clone() } else { None };
            non_list_field_mapper(ctx, f.clone(), default)
        })
        .collect();

    let mut list_fields: Vec<InputField> = prefiltered_fields
        .into_iter()
        .filter(|f| f.is_list)
        .map(|f| {
            let name = f.name.clone();
            let list_input_type = map_scalar_input_type(&f);
            let set_object_name = format!("{}{}{}Input", model_name, input_object_name, f.name);
            let input_object = match ctx.get_input_type(&set_object_name) {
                Some(t) => t,
                None => {
                    let set_fields = vec![input_field("set", list_input_type.clone(), None)];
                    let input_object = Arc::new(input_object_type(set_object_name.clone(), set_fields));

                    ctx.cache_input_type(set_object_name, input_object.clone());
                    Arc::downgrade(&input_object)
                }
            };

            let set_input_type = InputType::object(input_object);
            input_field(name, vec![set_input_type, list_input_type], None).optional()
        })
        .collect();

    non_list_fields.append(&mut list_fields);
    non_list_fields
}

fn where_input_field<T>(ctx: &mut BuilderContext, name: T, field: &RelationFieldRef) -> InputField
where
    T: Into<String>,
{
    let input_object_type = filter_objects::where_unique_object_type(ctx, &field.related_model());
    input_field(
        name.into(),
        list_union_object_type(input_object_type, field.is_list),
        None,
    )
    .optional()
}
