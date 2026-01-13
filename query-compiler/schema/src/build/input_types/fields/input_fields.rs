use super::*;
use constants::{args, operations};
use mutations::{create_many, create_one};
use objects::*;
use psl::datamodel_connector::ConnectorCapability;

pub(crate) fn filter_input_field(ctx: &'_ QuerySchema, field: ModelField, include_aggregates: bool) -> InputField<'_> {
    let types = field_filter_types::get_field_filter_types(ctx, field.clone(), include_aggregates);
    let nullable = !field.is_required()
        && !field.is_list()
        && match &field {
            ModelField::Scalar(sf) => sf.type_identifier() != TypeIdentifier::Json,
            _ => true,
        };

    let has_scalar_shorthand = match &field {
        ModelField::Scalar(sf) => !field.is_list() && sf.type_identifier() != TypeIdentifier::Json,
        _ => false,
    };

    input_field(field.name().to_owned(), types, None)
        .optional()
        .nullable_if(nullable)
        .parameterizable_if(has_scalar_shorthand)
}

pub(crate) fn nested_create_one_input_field(ctx: &'_ QuerySchema, parent_field: RelationFieldRef) -> InputField<'_> {
    let parent_field_is_list = parent_field.is_list();
    let create_types = create_one::create_one_input_types(ctx, parent_field.related_model(), Some(parent_field));

    let types: Vec<InputType<'_>> = create_types
        .into_iter()
        .flat_map(|typ| list_union_type(typ, parent_field_is_list))
        .collect();

    input_field(operations::CREATE, types, None).optional()
}

/// Nested create many calls can only ever be leaf operations because they can't return the ids of
/// affected rows. This means that we can't allow nested creates if the relation is inlined on the
/// parent, as this would require a flip in the order of operations that we can't do with no identifiers.
/// It also means that we can't serve implicit m:n relations, as this would require a write to the join
/// table, but we don't have the IDs.
pub(crate) fn nested_create_many_input_field(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputField<'_>> {
    if ctx.has_capability(ConnectorCapability::CreateMany)
        && parent_field.is_list()
        && !parent_field.is_inlined_on_enclosing_model()
        && !parent_field.relation().is_many_to_many()
    {
        let envelope = nested_create_many_envelope(ctx, parent_field);

        Some(input_field(operations::CREATE_MANY, vec![InputType::object(envelope)], None).optional())
    } else {
        None
    }
}

fn nested_create_many_envelope(ctx: &'_ QuerySchema, parent_field: RelationFieldRef) -> InputObjectType<'_> {
    let create_type =
        create_many::create_many_object_type(ctx, parent_field.related_model(), Some(parent_field.clone()));
    let name = format!("{}Envelope", create_type.identifier.name());
    let ident = Identifier::new_prisma(name);
    let mut input_object = init_input_object_type(ident);
    input_object.set_container(parent_field.related_model());
    input_object.set_fields(move || {
        let create_many_type = InputType::object(create_type.clone());
        let data_arg = input_field(args::DATA, list_union_type(create_many_type, true), None);

        if ctx.has_capability(ConnectorCapability::CreateSkipDuplicates) {
            let skip_arg = input_field(args::SKIP_DUPLICATES, vec![InputType::boolean()], None).optional();

            vec![data_arg, skip_arg]
        } else {
            vec![data_arg]
        }
    });
    input_object
}

pub(crate) fn nested_connect_or_create_field(
    ctx: &'_ QuerySchema,
    parent_field: RelationFieldRef,
) -> Option<InputField<'_>> {
    connect_or_create_objects::nested_connect_or_create_input_object(ctx, parent_field.clone()).map(
        |input_object_type| {
            input_field(
                operations::CONNECT_OR_CREATE,
                list_union_object_type(input_object_type, parent_field.is_list()),
                None,
            )
            .optional()
        },
    )
}

/// Builds "upsert" field for nested updates (on relation fields).
pub(crate) fn nested_upsert_field(ctx: &'_ QuerySchema, parent_field: RelationFieldRef) -> Option<InputField<'_>> {
    upsert_objects::nested_upsert_input_object(ctx, parent_field.clone()).map(|input_object_type| {
        input_field(
            operations::UPSERT,
            list_union_object_type(input_object_type, parent_field.is_list()),
            None,
        )
        .optional()
    })
}

/// Builds "deleteMany" field for nested updates (on relation fields).
pub(crate) fn nested_delete_many_field<'a>(
    ctx: &'a QuerySchema,
    parent_field: &RelationFieldRef,
) -> Option<InputField<'a>> {
    if parent_field.is_list() {
        let input_object = filter_objects::scalar_filter_object_type(ctx, parent_field.related_model(), false);
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
pub(crate) fn nested_update_many_field(ctx: &'_ QuerySchema, parent_field: RelationFieldRef) -> Option<InputField<'_>> {
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
pub(crate) fn nested_set_input_field<'a>(
    ctx: &'a QuerySchema,
    parent_field: &RelationFieldRef,
) -> Option<InputField<'a>> {
    if parent_field.is_list() {
        Some(where_unique_input_field(ctx, operations::SET, parent_field))
    } else {
        None
    }
}

/// Builds "disconnect" field for nested updates (on relation fields).
pub(crate) fn nested_disconnect_input_field<'a>(
    ctx: &'a QuerySchema,
    parent_field: &RelationFieldRef,
) -> Option<InputField<'a>> {
    match (parent_field.is_list(), parent_field.is_required()) {
        (true, _) => Some(where_unique_input_field(ctx, operations::DISCONNECT, parent_field)),
        (false, false) => {
            let mut types = vec![InputType::boolean()];

            if ctx.has_capability(ConnectorCapability::FilteredInlineChildNestedToOneDisconnect)
                       // If the disconnect happens on the inline side, then we can allow filters
                    || parent_field.related_field().is_inlined_on_enclosing_model()
            {
                types.push(InputType::object(filter_objects::where_object_type(
                    ctx,
                    parent_field.related_model().into(),
                )));
            }

            Some(input_field(operations::DISCONNECT, types, None).optional())
        }
        (false, true) => None,
    }
}

/// Builds "delete" field for nested updates (on relation fields).
pub(crate) fn nested_delete_input_field<'a>(
    ctx: &'a QuerySchema,
    parent_field: &RelationFieldRef,
) -> Option<InputField<'a>> {
    match (parent_field.is_list(), parent_field.is_required()) {
        (true, _) => Some(where_unique_input_field(ctx, operations::DELETE, parent_field)),
        (false, false) => {
            let types = vec![
                InputType::boolean(),
                InputType::object(filter_objects::where_object_type(
                    ctx,
                    parent_field.related_model().into(),
                )),
            ];

            Some(input_field(operations::DELETE, types, None).optional())
        }
        (false, true) => None,
    }
}

/// Builds the "connect" input field for a relation.
pub(crate) fn nested_connect_input_field<'a>(ctx: &'a QuerySchema, parent_field: &RelationFieldRef) -> InputField<'a> {
    where_unique_input_field(ctx, operations::CONNECT, parent_field)
}

pub(crate) fn nested_update_input_field(ctx: &'_ QuerySchema, parent_field: RelationFieldRef) -> InputField<'_> {
    let mut update_shorthand_types =
        update_one_objects::update_one_input_types(ctx, parent_field.related_model(), Some(parent_field.clone()));

    let update_types = if parent_field.is_list() {
        let to_many_update_full_type =
            update_one_objects::update_one_where_combination_object(ctx, update_shorthand_types.clone(), &parent_field);

        list_union_object_type(to_many_update_full_type, true)
    } else {
        let to_one_update_full_type = update_one_objects::update_to_one_rel_where_combination_object(
            ctx,
            update_shorthand_types.clone(),
            parent_field,
        );

        let mut to_one_types = vec![InputType::object(to_one_update_full_type)];
        to_one_types.append(&mut update_shorthand_types);

        to_one_types
    };

    input_field(operations::UPDATE, update_types, None).optional()
}

fn where_unique_input_field<'a, T>(ctx: &'a QuerySchema, name: T, field: &RelationFieldRef) -> InputField<'a>
where
    T: Into<String>,
{
    let input_object_type = filter_objects::where_unique_object_type(ctx, field.related_model());

    input_field(
        name.into(),
        list_union_object_type(input_object_type, field.is_list()),
        None,
    )
    .optional()
}
