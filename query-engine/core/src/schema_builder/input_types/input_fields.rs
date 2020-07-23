//! Top level input fields.

use super::*;

pub(crate) fn nested_create_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> InputField {
    let input_object = create_input_objects::create_input_type(ctx, &field.related_model(), Some(field));
    let input_object = wrap_list_input_object_type(input_object, field.is_list);

    input_field("create", input_object, None)
}

pub(crate) fn nested_connect_or_create_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    create_input_objects::nested_connect_or_create_input_object(ctx, field).map(|input_object| {
        let input_type = wrap_list_input_object_type(input_object, field.is_list);
        input_field("connectOrCreate", input_type, None)
    })
}

/// Builds "upsert" field for nested updates (on relation fields).
pub(crate) fn nested_upsert_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    update_input_objects::nested_upsert_input_object(ctx, field).map(|input_object| {
        let input_type = wrap_list_input_object_type(input_object, field.is_list);
        input_field("upsert", input_type, None)
    })
}

/// Builds "deleteMany" field for nested updates (on relation fields).
pub(crate) fn nested_delete_many_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    if field.is_list {
        let input_object = filter_input_objects::scalar_filter_object_type(ctx, &field.related_model());
        let input_type = InputType::opt(InputType::list(InputType::object(input_object)));

        Some(input_field("deleteMany", input_type, None))
    } else {
        None
    }
}

/// Builds "updateMany" field for nested updates (on relation fields).
pub(crate) fn nested_update_many_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    update_input_objects::nested_update_many_input_object(ctx, field).map(|input_object| {
        let input_type = InputType::opt(InputType::null(InputType::list(InputType::object(input_object))));
        input_field("updateMany", input_type, None)
    })
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
        (false, false, false) => Some(input_field("disconnect", InputType::opt(InputType::boolean()), None)),
        (false, false, true) => None,
    }
}

/// Builds "delete" field for nested updates (on relation fields).
pub(crate) fn nested_delete_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Option<InputField> {
    match (field.is_list, field.is_required) {
        (true, _) => Some(where_input_field(ctx, "delete", field)),
        (false, false) => Some(input_field("delete", InputType::opt(InputType::boolean()), None)),
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
    let input_object = update_input_objects::input_object_type_nested_update(ctx, field);
    let input_object = wrap_list_input_object_type(input_object, field.is_list);

    input_field("update", input_object, None)
}

/// Maps relations to (filter) input fields.
pub(crate) fn map_relation_filter_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Vec<InputField> {
    let related_model = field.related_model();
    let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);

    let input_fields: Vec<_> = filter_arguments::get_field_filters(&ModelField::Relation(field.clone()))
        .into_iter()
        .map(|arg| {
            let field_name = format!("{}{}", field.name, arg.suffix);
            let obj = InputType::object(related_input_type.clone());
            let typ = if arg.suffix == "" { InputType::null(obj) } else { obj };

            input_field(field_name, InputType::opt(typ), None)
        })
        .collect();

    input_fields
}

/// Builds scalar input fields using the mapper and the given, prefiltered, scalar fields.
/// The mapper is responsible for mapping the fields to input types.
pub(crate) fn scalar_input_fields<T, F>(
    ctx: &mut BuilderContext,
    model_name: String,
    input_object_name: T,
    prefiltered_fields: Vec<ScalarFieldRef>,
    field_mapper: F,
    with_defaults: bool,
) -> Vec<InputField>
where
    T: Into<String>,
    F: Fn(ScalarFieldRef) -> InputType,
{
    let input_object_name = input_object_name.into();
    let mut non_list_fields: Vec<InputField> = prefiltered_fields
        .iter()
        .filter(|f| !f.is_list)
        .map(|f| {
            let default = if with_defaults { f.default_value.clone() } else { None };
            input_field(f.name.clone(), field_mapper(f.clone()), default)
        })
        .collect();

    let mut list_fields: Vec<InputField> = prefiltered_fields
        .into_iter()
        .filter(|f| f.is_list)
        .map(|f| {
            let name = f.name.clone();
            let set_name = format!("{}{}{}Input", model_name, input_object_name, f.name);
            let input_object = match ctx.get_input_type(&set_name) {
                Some(t) => t,
                None => {
                    let set_fields = vec![input_field("set", map_optional_input_type(&f), None)];
                    let input_object = Arc::new(input_object_type(set_name.clone(), set_fields));

                    ctx.cache_input_type(set_name, input_object.clone());
                    Arc::downgrade(&input_object)
                }
            };

            let set_input_type = InputType::opt(InputType::object(input_object));
            input_field(name, set_input_type, None)
        })
        .collect();

    non_list_fields.append(&mut list_fields);
    non_list_fields
}

fn where_input_field<T>(ctx: &mut BuilderContext, name: T, field: &RelationFieldRef) -> InputField
where
    T: Into<String>,
{
    let input_type = filter_input_objects::where_unique_object_type(ctx, &field.related_model());
    let input_type = wrap_list_input_object_type(input_type, field.is_list);

    input_field(name.into(), input_type, None)
}
