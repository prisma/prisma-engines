use super::*;
use prisma_models::dml::DefaultValue;

pub(crate) fn update_one_input_types(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let checked_input = InputType::object(checked_update_one_input_type(ctx, model, parent_field));

    if feature_flags::get().uncheckedScalarInputs {
        let unchecked_input = InputType::object(unchecked_update_one_input_type(ctx, model, parent_field));

        // If the inputs are equal, only use one.
        if checked_input == unchecked_input {
            vec![checked_input]
        } else {
            vec![checked_input, unchecked_input]
        }
    } else {
        vec![checked_input]
    }
}

/// Builds "<x>UpdateInput" input object type.
fn checked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!(
            "{}UpdateWithout{}Input",
            model.name,
            capitalize(f.related_field().name.as_str())
        ),
        _ => format!("{}UpdateInput", model.name),
    };

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    // Compute input fields for scalar fields.
    let mut fields = scalar_input_fields_for_checked_update(ctx, model);

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_checked_update_one(ctx, model, parent_field);
    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UncheckedUpdateInput" input object type.
fn unchecked_update_one_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!(
            "{}UncheckedUpdateWithout{}Input",
            model.name,
            capitalize(f.related_field().name.as_str())
        ),
        _ => format!("{}UncheckedUpdateInput", model.name),
    };

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    // Compute input fields for scalar fields.
    let mut fields = scalar_input_fields_for_unchecked_update(ctx, model, parent_field);

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_unchecked_update_one(ctx, model, parent_field);
    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

pub(super) fn scalar_input_fields_for_checked_update(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Update",
        model
            .fields()
            .scalar_writable()
            .filter(field_should_be_kept_for_update_input_type)
            .collect(),
        |ctx, f: ScalarFieldRef, default| non_list_scalar_update_field_mapper(ctx, &f, default),
        false,
    )
}

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

fn non_list_scalar_update_field_mapper(
    ctx: &mut BuilderContext,
    field: &ScalarFieldRef,
    default: Option<DefaultValue>,
) -> InputField {
    let base_update_type = match &field.type_identifier {
        TypeIdentifier::Float => InputType::object(operations_object_type(ctx, "Float", field, true)),
        TypeIdentifier::Decimal => InputType::object(operations_object_type(ctx, "Decimal", field, true)),
        TypeIdentifier::Int => InputType::object(operations_object_type(ctx, "Int", field, true)),
        TypeIdentifier::String => InputType::object(operations_object_type(ctx, "String", field, false)),
        TypeIdentifier::Boolean => InputType::object(operations_object_type(ctx, "Bool", field, false)),
        TypeIdentifier::Enum(e) => InputType::object(operations_object_type(ctx, &format!("Enum{}", e), field, false)),
        TypeIdentifier::Json => map_scalar_input_type(field),
        TypeIdentifier::DateTime => InputType::object(operations_object_type(ctx, "DateTime", field, false)),
        TypeIdentifier::UUID => InputType::object(operations_object_type(ctx, "Uuid", field, false)),
        TypeIdentifier::Xml => InputType::object(operations_object_type(ctx, "Xml", field, false)),
        TypeIdentifier::Bytes => InputType::object(operations_object_type(ctx, "Bytes", field, false)),
    };

    let input_field = if field.type_identifier != TypeIdentifier::Json {
        let types = vec![map_scalar_input_type(field), base_update_type];
        input_field(field.name.clone(), types, default)
    } else {
        input_field(field.name.clone(), base_update_type, default)
    };

    input_field.optional().nullable_if(!field.is_required)
}

fn operations_object_type(
    ctx: &mut BuilderContext,
    prefix: &str,
    field: &ScalarFieldRef,
    with_number_operators: bool,
) -> InputObjectTypeWeakRef {
    // Nullability is important for the `set` operation, so we need to
    // construct and cache different objects to reflect that.
    let nullable = if field.is_required { "" } else { "Nullable" };
    let name = format!("{}{}FieldUpdateOperationsInput", nullable, prefix);
    return_cached_input!(ctx, &name);

    let mut obj = init_input_object_type(&name);
    obj.require_exactly_one_field();

    let obj = Arc::new(obj);
    ctx.cache_input_type(name, obj.clone());

    let typ = map_scalar_input_type(field);
    let mut fields = vec![input_field("set", typ.clone(), None)
        .optional()
        .nullable_if(!field.is_required)];

    if with_number_operators {
        fields.push(input_field("increment", typ.clone(), None).optional());
        fields.push(input_field("decrement", typ.clone(), None).optional());
        fields.push(input_field("multiply", typ.clone(), None).optional());
        fields.push(input_field("divide", typ, None).optional());
    }

    obj.set_fields(fields);

    Arc::downgrade(&obj)
}

/// For update input types only. Compute input fields for checked relational fields.
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

            let input_name = format!("{}Update{}{}Input", related_model.name, arity_part, without_part);
            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            if field_is_opposite_relation_field {
                None
            } else {
                let input_object = match ctx.get_input_type(&input_name) {
                    Some(t) => t,
                    None => {
                        let input_object = Arc::new(init_input_object_type(input_name.clone()));
                        ctx.cache_input_type(input_name, input_object.clone());

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
            let input_name = format!(
                "{}UncheckedUpdate{}{}Input",
                related_model.name, arity_part, without_part
            );
            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            // Filter out all inlined relations on `related_model`.
            // -> Only relations that point to other models are allowed in the unchecked input.
            if field_is_opposite_relation_field || !related_field.is_inlined_on_enclosing_model() {
                None
            } else {
                let input_object = match ctx.get_input_type(&input_name) {
                    Some(t) => t,
                    None => {
                        let input_object = Arc::new(init_input_object_type(input_name.clone()));
                        ctx.cache_input_type(input_name, input_object.clone());

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
pub(crate) fn update_one_where_combination_object(
    ctx: &mut BuilderContext,
    update_types: Vec<InputType>,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let where_input_object = filter_objects::where_unique_object_type(ctx, &related_model);
    let type_name = format!(
        "{}UpdateWithWhereUniqueWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    return_cached_input!(ctx, &type_name);
    let input_object = Arc::new(init_input_object_type(type_name.clone()));
    ctx.cache_input_type(type_name, input_object.clone());

    let fields = vec![
        input_field("where", InputType::object(where_input_object), None),
        input_field("data", update_types, None),
    ];

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn field_should_be_kept_for_update_input_type(field: &ScalarFieldRef) -> bool {
    // We forbid updating auto-increment integer unique fields as this can create problems with the
    // underlying sequences (checked inputs only).
    !field.is_auto_generated_int_id
        && !matches!(
            (&field.type_identifier, field.unique(), field.is_autoincrement),
            (TypeIdentifier::Int, true, true)
        )
}
