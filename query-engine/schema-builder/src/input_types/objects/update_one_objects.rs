use super::{arguments, fields::data_input_mapper::*, *};
use constants::args;
use psl::datamodel_connector::ConnectorCapability;

pub(crate) fn update_one_input_types(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let checked_input = InputType::object(checked_update_one_input_type(ctx, model, parent_field));
    let unchecked_input = InputType::object(unchecked_update_one_input_type(ctx, model, parent_field));

    // If the inputs are equal, only use one.
    if checked_input == unchecked_input {
        vec![checked_input]
    } else {
        vec![checked_input, unchecked_input]
    }
}

/// Builds "<x>UpdateInput" input object type.
fn checked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}UpdateWithout{}Input", model.name(), capitalize(f.name())),
        _ => format!("{}UpdateInput", model.name()),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields = filter_checked_update_fields(ctx, model, parent_field);
    let field_mapper = UpdateDataInputFieldMapper::new_checked();
    let input_fields = field_mapper.map_all(ctx, &filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateInput" input object type.
fn unchecked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}UncheckedUpdateWithout{}Input", model.name(), capitalize(f.name())),
        _ => format!("{}UncheckedUpdateInput", model.name()),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields = filter_unchecked_update_fields(ctx, model, parent_field);
    let field_mapper = UpdateDataInputFieldMapper::new_unchecked();
    let input_fields = field_mapper.map_all(ctx, &filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Filters the given model's fields down to the allowed ones for checked update.
pub(super) fn filter_checked_update_fields(
    ctx: &BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<ModelField> {
    model
        .fields()
        .all
        .iter()
        .filter(|field| {
            match field {
                ModelField::Scalar(sf) => {
                    // We forbid updating auto-increment integer unique fields as this can create problems with the
                    // underlying sequences (checked inputs only).
                    let is_not_autoinc = !sf.is_auto_generated_int_id()
                        && !matches!(
                            (&sf.type_identifier(), sf.unique(), sf.is_autoincrement()),
                            (TypeIdentifier::Int, true, true)
                        );

                    let model_id = sf.container().as_model().unwrap().primary_identifier();
                    let is_not_disallowed_id = if model_id.contains(sf.name()) {
                        // Is part of the id, connector must allow updating ID fields.
                        ctx.has_capability(ConnectorCapability::UpdateableId)
                    } else {
                        true
                    };

                    !sf.is_read_only() && is_not_autoinc && is_not_disallowed_id
                }

                // If the relation field `rf` is the one that was traversed to by the parent relation field `parent_field`,
                // then exclude it for checked inputs - this prevents endless nested type circles that are useless to offer as API.
                ModelField::Relation(rf) => {
                    let field_was_traversed_to = parent_field
                        .filter(|pf| pf.related_field().name() == rf.name())
                        .is_some();
                    !field_was_traversed_to
                }

                // Always keep composites
                ModelField::Composite(_) => true,
            }
        })
        .map(Clone::clone)
        .collect()
}

/// Filters the given model's fields down to the allowed ones for unchecked update.
pub(super) fn filter_unchecked_update_fields(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<ModelField> {
    let linking_fields = if let Some(parent_field) = parent_field {
        let child_field = parent_field.related_field();
        if child_field.is_inlined_on_enclosing_model() {
            child_field
                .linking_fields()
                .as_scalar_fields()
                .expect("Expected linking fields to be scalar.")
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let id_fields = model.fields().id().map(|pk| pk.fields());
    model
        .fields()
        .all
        .iter()
        .filter(|field| match field {
            // 1) In principle, all scalars are writable for unchecked inputs. However, it still doesn't make any sense to be able to write the scalars that
            // link the model to the parent record in case of a nested unchecked create, as this would introduce complexities we don't want to deal with right now.
            // 2) Exclude @@id or @id fields if not updatable
            ModelField::Scalar(sf) => {
                !linking_fields.contains(sf)
                    && if let Some(ref id_fields) = &id_fields {
                        // Exclude @@id or @id fields if not updatable
                        if id_fields.contains(sf) {
                            ctx.has_capability(ConnectorCapability::UpdateableId)
                        } else {
                            true
                        }
                    } else {
                        true
                    }
            }

            // If the relation field `rf` is the one that was traversed to by the parent relation field `parent_field`,
            // then exclude it for checked inputs - this prevents endless nested type circles that are useless to offer as API.
            //
            // Additionally, only relations that point to other models and are NOT inlined on the currently in scope model are allowed in the unchecked input, because if they are
            // inlined, they are written only as scalars for unchecked, not via the relation API (`connect`, nested `create`, etc.).
            ModelField::Relation(rf) => {
                let is_not_inlined = !rf.is_inlined_on_enclosing_model();
                let field_was_not_traversed_to = parent_field
                    .filter(|pf| pf.related_field().name() == rf.name())
                    .is_none();

                field_was_not_traversed_to && is_not_inlined
            }

            // Always keep composites
            ModelField::Composite(_) => true,
        })
        .map(Clone::clone)
        .collect()
}

/// Builds "<x>UpdateWithWhereUniqueNestedInput" / "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
/// Simple combination object of "where" and "data".
pub(crate) fn update_one_where_combination_object(
    ctx: &mut BuilderContext,
    update_types: Vec<InputType>,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let where_input_object = filter_objects::where_unique_object_type(ctx, &related_model);
    let ident = Identifier::new(
        format!(
            "{}UpdateWithWhereUniqueWithout{}Input",
            related_model.name(),
            capitalize(parent_field.related_field().name())
        ),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let fields = vec![
        input_field(args::WHERE, InputType::object(where_input_object), None),
        input_field(args::DATA, update_types, None),
    ];

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
/// Simple combination object of "where" and "data" for to-one relations.
pub(crate) fn update_to_one_rel_where_combination_object(
    ctx: &mut BuilderContext,
    update_types: Vec<InputType>,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let ident = Identifier::new(
        format!(
            "{}UpdateToOneWithWhereWithout{}Input",
            related_model.name(),
            capitalize(parent_field.related_field().name())
        ),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::NestedToOneUpdateEnvelope);
    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = vec![
        arguments::where_argument(ctx, &related_model),
        input_field(args::DATA, update_types, None),
    ];

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}
