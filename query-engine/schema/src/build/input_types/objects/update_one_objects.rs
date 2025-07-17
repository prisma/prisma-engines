use super::{arguments, fields::data_input_mapper::*, *};
use constants::args;
use psl::datamodel_connector::ConnectorCapability;

pub(crate) fn update_one_input_types(
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> Vec<InputType<'_>> {
    let checked_input = InputType::object(checked_update_one_input_type(ctx, model.clone(), parent_field.clone()));
    let unchecked_input = InputType::object(unchecked_update_one_input_type(ctx, model, parent_field));

    vec![checked_input, unchecked_input]
}

/// Builds "<x>UpdateInput" input object type.
fn checked_update_one_input_type(
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CheckedUpdateOneInput(
        model.clone(),
        parent_field.as_ref().map(|pf| pf.related_field()),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(model.clone());
    input_object.set_fields(move || {
        let mut filtered_fields = filter_checked_update_fields(ctx, &model, parent_field.as_ref());
        let field_mapper = UpdateDataInputFieldMapper::new_checked();
        field_mapper.map_all(ctx, &mut filtered_fields)
    });
    input_object
}

/// Builds "<x>UncheckedUpdateInput" input object type.
fn unchecked_update_one_input_type(
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::UncheckedUpdateOneInput(
        model.clone(),
        parent_field.as_ref().map(|pf| pf.related_field()),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(model.clone());
    input_object.set_fields(move || {
        let mut filtered_fields = filter_unchecked_update_fields(ctx, &model, parent_field.as_ref());
        let field_mapper = UpdateDataInputFieldMapper::new_unchecked();
        field_mapper.map_all(ctx, &mut filtered_fields)
    });

    input_object
}

/// Filters the given model's fields down to the allowed ones for checked update.
pub(super) fn filter_checked_update_fields<'a>(
    ctx: &'a QuerySchema,
    model: &'a Model,
    parent_field: Option<&'a RelationFieldRef>,
) -> impl Iterator<Item = ModelField> + 'a {
    model.fields().filter_all(move |field| {
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
}

/// Filters the given model's fields down to the allowed ones for unchecked update.
pub(super) fn filter_unchecked_update_fields<'a>(
    ctx: &'a QuerySchema,
    model: &'a Model,
    parent_field: Option<&'a RelationFieldRef>,
) -> impl Iterator<Item = ModelField> + 'a {
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

    let fields = model.fields();
    let id_fields = fields.id_fields();
    model.fields().filter_all(move |field| match field {
        // 1) In principle, all scalars are writable for unchecked inputs. However, it still doesn't make any sense to be able to write the scalars that
        // link the model to the parent record in case of a nested unchecked create, as this would introduce complexities we don't want to deal with right now.
        // 2) Exclude @@id or @id fields if not updatable
        ModelField::Scalar(sf) => {
            !linking_fields.contains(sf)
                && if let Some(id_fields) = &id_fields {
                    // Exclude @@id or @id fields if not updatable
                    if id_fields.clone().any(|f| f.id == sf.id) {
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
}

/// Builds "<x>UpdateWithWhereUniqueNestedInput" / "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
/// Simple combination object of "where" and "data".
pub(crate) fn update_one_where_combination_object<'a>(
    ctx: &'a QuerySchema,
    update_types: Vec<InputType<'a>>,
    parent_field: &RelationFieldRef,
) -> InputObjectType<'a> {
    let ident = Identifier::new_prisma(IdentifierType::UpdateOneWhereCombinationInput(
        parent_field.related_field(),
    ));

    let related_model = parent_field.related_model();
    let where_input_object = filter_objects::where_unique_object_type(ctx, related_model);

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(parent_field.related_model());
    input_object.set_fields(move || {
        vec![
            simple_input_field(args::WHERE, InputType::object(where_input_object.clone()), None),
            input_field(args::DATA, update_types.clone(), None),
        ]
    });
    input_object
}

/// Builds "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
/// Simple combination object of "where" and "data" for to-one relations.
pub(crate) fn update_to_one_rel_where_combination_object<'a>(
    ctx: &'a QuerySchema,
    update_types: Vec<InputType<'a>>,
    parent_field: RelationFieldRef,
) -> InputObjectType<'a> {
    let ident = Identifier::new_prisma(IdentifierType::UpdateToOneRelWhereCombinationInput(
        parent_field.related_field(),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(parent_field.related_model());
    input_object.set_tag(ObjectTag::NestedToOneUpdateEnvelope);
    input_object.set_fields(move || {
        let related_model = parent_field.related_model();
        vec![
            arguments::where_argument(ctx, &related_model),
            input_field(args::DATA, update_types.clone(), None),
        ]
    });
    input_object
}
