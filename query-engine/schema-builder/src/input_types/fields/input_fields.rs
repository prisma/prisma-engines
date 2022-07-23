use super::objects::*;
use super::*;
use crate::mutations::create_one;
use constants::{args, operations};
use datamodel_connector::ConnectorCapability;

pub(crate) fn filter_input_field(ctx: &mut BuilderContext, field: &ModelField, include_aggregates: bool) -> InputField {
    let types = field_filter_types::get_field_filter_types(ctx, field, include_aggregates);
    let nullable = !field.is_required()
        && !field.is_list()
        && match field {
            ModelField::Scalar(sf) => sf.type_identifier != TypeIdentifier::Json,
            _ => true,
        };

    input_field(field.name().to_owned(), types, None)
        .optional()
        .nullable_if(nullable)
}

pub(crate) fn nested_create_one_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputField {
    let create_types = create_one::create_one_input_types(ctx, &parent_field.related_model(), Some(parent_field));

    let types: Vec<InputType> = create_types
        .into_iter()
        .flat_map(|typ| list_union_type(typ, parent_field.is_list()))
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
        && parent_field.is_list()
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
            list_union_object_type(input_object_type, parent_field.is_list()),
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
            list_union_object_type(input_object_type, parent_field.is_list()),
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
    if parent_field.is_list() {
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
    if parent_field.is_list() {
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
    if parent_field.is_list() {
        Some(where_input_field(ctx, operations::SET, parent_field))
    } else {
        None
    }
}

/// Builds "disconnect" field for nested updates (on relation fields).
pub(crate) fn nested_disconnect_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    match (parent_field.is_list(), parent_field.is_required()) {
        (true, _) => Some(where_input_field(ctx, operations::DISCONNECT, parent_field)),
        (false, false) => Some(input_field(operations::DISCONNECT, InputType::boolean(), None).optional()),
        (false, true) => None,
    }
}

/// Builds "delete" field for nested updates (on relation fields).
pub(crate) fn nested_delete_input_field(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputField> {
    match (parent_field.is_list(), parent_field.is_required()) {
        (true, _) => Some(where_input_field(ctx, operations::DELETE, parent_field)),
        (false, false) => Some(input_field(operations::DELETE, InputType::boolean(), None).optional()),
        (false, true) => None,
    }
}

/// Builds the "connect" input field for a relation.
pub(crate) fn nested_connect_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputField {
    where_input_field(ctx, operations::CONNECT, parent_field)
}

pub(crate) fn nested_update_input_field(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputField {
    let update_one_types =
        update_one_objects::update_one_input_types(ctx, &parent_field.related_model(), Some(parent_field));

    let update_types = if parent_field.is_list() {
        let list_object_type =
            update_one_objects::update_one_where_combination_object(ctx, update_one_types, parent_field);
        list_union_object_type(list_object_type, true)
    } else {
        update_one_types
    };

    input_field(operations::UPDATE, update_types, None).optional()
}

fn where_input_field<T>(ctx: &mut BuilderContext, name: T, field: &RelationFieldRef) -> InputField
where
    T: Into<String>,
{
    let input_object_type = filter_objects::where_unique_object_type(ctx, &field.related_model());
    input_field(
        name.into(),
        list_union_object_type(input_object_type, field.is_list()),
        None,
    )
    .optional()
}
