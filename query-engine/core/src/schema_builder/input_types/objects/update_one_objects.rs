use super::*;
use constants::inputs::{args, operations};
use datamodel_connector::ConnectorCapability;
use prisma_models::{dml::DefaultValue, ModelProjection};

#[tracing::instrument(skip(ctx, model, parent_field))]
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
#[tracing::instrument(skip(ctx, model, parent_field))]
fn checked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}UpdateWithout{}Input", model.name, capitalize(f.name.as_str())),
        _ => format!("{}UpdateInput", model.name),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    // Compute input fields for scalar fields.
    let mut fields = scalar_input_fields_for_checked_update(ctx, model);

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_checked_update_one(ctx, model, parent_field);
    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateInput" input object type.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn unchecked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!(
            "{}UncheckedUpdateWithout{}Input",
            model.name,
            capitalize(f.name.as_str())
        ),
        _ => format!("{}UncheckedUpdateInput", model.name),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    // Compute input fields for scalar fields.
    let mut fields = scalar_input_fields_for_unchecked_update(ctx, model, parent_field);

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_unchecked_update_one(ctx, model, parent_field);
    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

#[tracing::instrument(skip(ctx, model))]
pub(super) fn scalar_input_fields_for_checked_update(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Update",
        model
            .fields()
            .scalar_writable()
            .filter(|sf| field_should_be_kept_for_checked_update_input_type(ctx, sf))
            .collect(),
        |ctx, f: ScalarFieldRef, default| non_list_scalar_update_field_mapper(ctx, &f, default),
        false,
    )
}

#[tracing::instrument(skip(ctx, model, parent_field))]
pub(super) fn scalar_input_fields_for_unchecked_update(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputField> {
    let linking_fields = if let Some(parent_field) = parent_field {
        let child_field = parent_field.related_field();
        if child_field.is_inlined_on_enclosing_model() {
            child_field.linking_fields().scalar_fields().collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|sf| !linking_fields.contains(sf))
        .collect();

    input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Update",
        scalar_fields,
        |ctx, f: ScalarFieldRef, default| non_list_scalar_update_field_mapper(ctx, &f, default),
        false,
    )
}

#[tracing::instrument(skip(ctx, field, default))]
fn non_list_scalar_update_field_mapper(
    ctx: &mut BuilderContext,
    field: &ScalarFieldRef,
    default: Option<DefaultValue>,
) -> InputField {
    let base_update_type = match &field.type_identifier {
        TypeIdentifier::Float => InputType::object(operations_object_type(ctx, "Float", field, true)),
        TypeIdentifier::Decimal => InputType::object(operations_object_type(ctx, "Decimal", field, true)),
        TypeIdentifier::Int => InputType::object(operations_object_type(ctx, "Int", field, true)),
        TypeIdentifier::BigInt => InputType::object(operations_object_type(ctx, "BigInt", field, true)),
        TypeIdentifier::String => InputType::object(operations_object_type(ctx, "String", field, false)),
        TypeIdentifier::Boolean => InputType::object(operations_object_type(ctx, "Bool", field, false)),
        TypeIdentifier::Enum(e) => InputType::object(operations_object_type(ctx, &format!("Enum{}", e), field, false)),
        TypeIdentifier::Json => map_scalar_input_type_for_field(ctx, field),
        TypeIdentifier::DateTime => InputType::object(operations_object_type(ctx, "DateTime", field, false)),
        TypeIdentifier::UUID => InputType::object(operations_object_type(ctx, "Uuid", field, false)),
        TypeIdentifier::Xml => InputType::object(operations_object_type(ctx, "Xml", field, false)),
        TypeIdentifier::Bytes => InputType::object(operations_object_type(ctx, "Bytes", field, false)),
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    };

    let input_field = if field.type_identifier != TypeIdentifier::Json {
        let types = vec![map_scalar_input_type_for_field(ctx, field), base_update_type];
        input_field(field.name.clone(), types, default)
    } else {
        input_field(field.name.clone(), base_update_type, default)
    };

    input_field.optional().nullable_if(!field.is_required)
}

#[tracing::instrument(skip(ctx, prefix, field, with_number_operators))]
fn operations_object_type(
    ctx: &mut BuilderContext,
    prefix: &str,
    field: &ScalarFieldRef,
    with_number_operators: bool,
) -> InputObjectTypeWeakRef {
    // Nullability is important for the `set` operation, so we need to
    // construct and cache different objects to reflect that.
    let nullable = if field.is_required { "" } else { "Nullable" };
    let ident = Identifier::new(
        format!("{}{}FieldUpdateOperationsInput", nullable, prefix),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut obj = init_input_object_type(ident.clone());
    obj.require_exactly_one_field();

    let obj = Arc::new(obj);
    ctx.cache_input_type(ident, obj.clone());

    let typ = map_scalar_input_type_for_field(ctx, field);
    let mut fields = vec![input_field(operations::SET, typ.clone(), None)
        .optional()
        .nullable_if(!field.is_required)];

    if with_number_operators {
        fields.push(input_field(operations::INCREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::DECREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::MULTIPLY, typ.clone(), None).optional());
        fields.push(input_field(operations::DIVIDE, typ, None).optional());
    }

    obj.set_fields(fields);

    Arc::downgrade(&obj)
}

/// For update input types only. Compute input fields for checked relational fields.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn relation_input_fields_for_checked_update_one(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputField> {
    model
        .fields()
        .relation()
        .into_iter()
        .filter_map(|rf| {
            let related_model = rf.related_model();
            let related_field = rf.related_field();

            // Compute input object name
            let arity_part = match (rf.is_list, rf.is_required) {
                (true, _) => "Many",
                (false, true) => "OneRequired",
                (false, false) => "One",
            };

            let without_part = format!("Without{}", capitalize(&related_field.name));
            let ident = Identifier::new(
                format!("{}Update{}{}Input", related_model.name, arity_part, without_part),
                PRISMA_NAMESPACE,
            );

            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            if field_is_opposite_relation_field {
                None
            } else {
                let input_object = match ctx.get_input_type(&ident) {
                    Some(t) => t,
                    None => {
                        let input_object = Arc::new(init_input_object_type(ident.clone()));
                        ctx.cache_input_type(ident, input_object.clone());

                        // Enqueue the nested update input for its fields to be
                        // created at a later point, to avoid recursing too deep
                        // (that has caused stack overflows on large schemas in
                        // the past).
                        ctx.nested_update_inputs_queue
                            .push((Arc::clone(&input_object), Arc::clone(&rf)));

                        Arc::downgrade(&input_object)
                    }
                };

                Some(input_field(rf.name.clone(), InputType::object(input_object), None).optional())
            }
        })
        .collect()
}

/// For unchecked update input types only. Compute input fields for checked relational fields.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn relation_input_fields_for_unchecked_update_one(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputField> {
    model
        .fields()
        .relation()
        .into_iter()
        .filter_map(|rf| {
            let related_model = rf.related_model();
            let related_field = rf.related_field();

            // Compute input object name
            let arity_part = match (rf.is_list, rf.is_required) {
                (true, _) => "Many",
                (false, true) => "OneRequired",
                (false, false) => "One",
            };

            let without_part = format!("Without{}", capitalize(&related_field.name));
            let ident = Identifier::new(
                format!(
                    "{}UncheckedUpdate{}{}Input",
                    related_model.name, arity_part, without_part
                ),
                PRISMA_NAMESPACE,
            );

            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            // Filter out all inlined relations on `related_model`.
            // -> Only relations that point to other models are allowed in the unchecked input.
            if field_is_opposite_relation_field || !related_field.is_inlined_on_enclosing_model() {
                None
            } else {
                let input_object = match ctx.get_input_type(&ident) {
                    Some(t) => t,
                    None => {
                        let input_object = Arc::new(init_input_object_type(ident.clone()));
                        ctx.cache_input_type(ident, input_object.clone());

                        // Enqueue the nested update input for its fields to be
                        // created at a later point, to avoid recursing too deep
                        // (that has caused stack overflows on large schemas in
                        // the past).
                        ctx.nested_update_inputs_queue
                            .push((Arc::clone(&input_object), Arc::clone(&rf)));

                        Arc::downgrade(&input_object)
                    }
                };

                Some(input_field(rf.name.clone(), InputType::object(input_object), None).optional())
            }
        })
        .collect()
}

/// Builds "<x>UpdateWithWhereUniqueNestedInput" / "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
/// Simple combination object of "where" and "data".
#[tracing::instrument(skip(ctx, update_types, parent_field))]
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
            related_model.name,
            capitalize(&parent_field.related_field().name)
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

fn field_should_be_kept_for_checked_update_input_type(ctx: &BuilderContext, field: &ScalarFieldRef) -> bool {
    // We forbid updating auto-increment integer unique fields as this can create problems with the
    // underlying sequences (checked inputs only).
    let is_not_autoinc = !field.is_auto_generated_int_id
        && !matches!(
            (&field.type_identifier, field.unique(), field.is_autoincrement),
            (TypeIdentifier::Int, true, true)
        );

    let model_id: ModelProjection = field.model().primary_identifier();
    let is_not_disallowed_id = if model_id.contains(field.clone()) {
        // Is part of the id, connector must allow updating ID fields.
        ctx.capabilities.contains(ConnectorCapability::UpdateableId)
    } else {
        true
    };

    is_not_autoinc && is_not_disallowed_id
}
