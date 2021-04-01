use crate::constants::inputs::args;

use super::*;
use constants::inputs::operations;
use datamodel_connector::ConnectorCapability;
use prisma_models::dml::DefaultValue;

pub(crate) fn filter_input_field(ctx: &mut BuilderContext, field: &ModelField, include_aggregates: bool) -> InputField {
    let types = field_filter_types::get_field_filter_types(ctx, field, include_aggregates);
    input_field(field.name().to_owned(), types, None).optional()
}

pub(crate) fn nested_create_one_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputField {
    let create_types =
        create_one_objects::create_one_input_types(ctx, &parent_field.related_model(), Some(parent_field));

    let types: Vec<InputType> = create_types
        .into_iter()
        .flat_map(|typ| list_union_type(typ, parent_field.is_list))
        .collect();

    input_field(operations::CREATE, types, None).optional()
}

/// Nested create many calls can only ever be leaf operations because they can't return the ids of
/// affected rows. This means that we can't allow nested creates if the relation is inlined on the
/// parent, as this would require a flip in the order of operations that we can't do with no identifiers.
/// It also means that we can't serve implicit m:n relations, as this would require a write to the join
/// table, but we don't have the IDs.
pub(crate) fn nested_create_many_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    if ctx.capabilities.contains(ConnectorCapability::CreateMany)
        && parent_field.is_list
        && !parent_field.is_inlined_on_enclosing_model()
        && !parent_field.relation().is_many_to_many()
    {
        let envelope = nested_create_many_envelope(ctx, parent_field);
        Some(input_field("createMany", InputType::object(envelope), None).optional())
    } else {
        None
    }
}

fn nested_create_many_envelope(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let create_type =
        create_many_objects::create_many_object_type(ctx, &parent_field.related_model(), Some(parent_field));

    let nested_ident = &create_type.into_arc().identifier;
    let name = format!("{}Envelope", nested_ident.name());

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let create_many_type = InputType::object(create_type);
    let data_arg = input_field("data", InputType::list(create_many_type), None);

    let fields = if ctx.capabilities.contains(ConnectorCapability::CreateSkipDuplicates) {
        let skip_arg = input_field(args::SKIP_DUPLICATES, InputType::boolean(), None).optional();

        vec![data_arg, skip_arg]
    } else {
        vec![data_arg]
    };

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

pub(crate) fn nested_connect_or_create_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    connect_or_create_objects::nested_connect_or_create_input_object(ctx, parent_field).map(|input_object_type| {
        input_field(
            operations::CONNECT_OR_CREATE,
            list_union_object_type(input_object_type, parent_field.is_list),
            None,
        )
        .optional()
    })
}

/// Builds "upsert" field for nested updates (on relation fields).
pub(crate) fn nested_upsert_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> Option<InputField> {
    upsert_objects::nested_upsert_input_object(ctx, parent_field).map(|input_object_type| {
        input_field(
            operations::UPSERT,
            list_union_object_type(input_object_type, parent_field.is_list),
            None,
        )
        .optional()
    })
}

/// Builds "deleteMany" field for nested updates (on relation fields).
pub(crate) fn nested_delete_many_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    if parent_field.is_list {
        let input_object = filter_objects::scalar_filter_object_type(ctx, &parent_field.related_model(), false);
        let input_type = InputType::object(input_object);

        Some(
            input_field(
                operations::DELETE_MANY,
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
                operations::UPDATE_MANY,
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
pub(crate) fn nested_set_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> Option<InputField> {
    match (parent_field.related_model().is_embedded, parent_field.is_list) {
        (true, _) => None,
        (false, true) => Some(where_input_field(ctx, operations::SET, parent_field)),
        (false, false) => None,
    }
}

/// Builds "disconnect" field for nested updates (on relation fields).
pub(crate) fn nested_disconnect_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    match (
        parent_field.related_model().is_embedded,
        parent_field.is_list,
        parent_field.is_required,
    ) {
        (true, _, _) => None,
        (false, true, _) => Some(where_input_field(ctx, operations::DISCONNECT, parent_field)),
        (false, false, false) => Some(input_field(operations::DISCONNECT, InputType::boolean(), None).optional()),
        (false, false, true) => None,
    }
}

/// Builds "delete" field for nested updates (on relation fields).
pub(crate) fn nested_delete_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    match (parent_field.is_list, parent_field.is_required) {
        (true, _) => Some(where_input_field(ctx, operations::DELETE, parent_field)),
        (false, false) => Some(input_field(operations::DELETE, InputType::boolean(), None).optional()),
        (false, true) => None,
    }
}

/// Builds the "connect" input field for a relation.
pub(crate) fn nested_connect_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    if parent_field.related_model().is_embedded {
        None
    } else {
        Some(where_input_field(ctx, operations::CONNECT, parent_field))
    }
}

pub(crate) fn nested_update_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputField {
    let update_one_types =
        update_one_objects::update_one_input_types(ctx, &parent_field.related_model(), Some(parent_field));

    let update_types = if parent_field.is_list {
        let list_object_type =
            update_one_objects::update_one_where_combination_object(ctx, update_one_types, parent_field);
        list_union_object_type(list_object_type, true)
    } else {
        update_one_types
    };

    input_field(operations::UPDATE, update_types, None).optional()
}

/// Builds scalar input fields using the mapper and the given, prefiltered, scalar fields.
/// The mapper is responsible for mapping the fields to input types.
pub(crate) fn scalar_input_fields<F, G>(
    ctx: &mut BuilderContext,
    prefiltered_fields: Vec<ScalarFieldRef>,
    non_list_field_mapper: F,
    list_field_mapper: G,
    with_defaults: bool,
) -> Vec<InputField>
where
    F: Fn(&mut BuilderContext, ScalarFieldRef, Option<DefaultValue>) -> InputField,
    G: Fn(&mut BuilderContext, ScalarFieldRef, Option<DefaultValue>) -> InputField,
{
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
            let default = if with_defaults { f.default_value.clone() } else { None };
            list_field_mapper(ctx, f.clone(), default)
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

pub(crate) fn scalar_list_input_field_mapper<T>(
    ctx: &mut BuilderContext,
    model_name: String,
    input_object_name: T,
    f: ScalarFieldRef,
    is_create: bool,
) -> InputField
where
    T: Into<String>,
{
    let list_input_type = map_scalar_input_type(ctx, &f.type_identifier, f.is_list);
    let ident = Identifier::new(
        format!("{}{}{}Input", model_name, input_object_name.into(), f.name),
        PRISMA_NAMESPACE,
    );

    let input_object = match ctx.get_input_type(&ident) {
        Some(t) => t,
        None => {
            let mut object_fields =
                vec![input_field(operations::SET, list_input_type.clone(), None).optional_if(!is_create)];

            if !is_create {
                object_fields.push(
                    input_field(
                        operations::PUSH,
                        vec![
                            map_scalar_input_type(ctx, &f.type_identifier, false),
                            list_input_type.clone(),
                        ],
                        None,
                    )
                    .optional(),
                )
            }

            let mut input_object = input_object_type(ident.clone(), object_fields);
            input_object.require_exactly_one_field();

            let input_object = Arc::new(input_object);
            ctx.cache_input_type(ident, input_object.clone());

            Arc::downgrade(&input_object)
        }
    };

    let input_type = InputType::object(input_object);
    input_field(f.name.clone(), vec![input_type, list_input_type], None).optional()
}
