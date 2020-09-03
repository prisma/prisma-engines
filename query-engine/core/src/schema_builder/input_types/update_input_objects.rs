use super::*;

/// Builds "<x>UpdateInput" input object type.
pub(crate) fn update_input_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let name = format!("{}UpdateInput", model.name);
    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    // Compute input fields for scalar fields.
    let mut fields = scalar_input_fields_for_update(ctx, model);

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_update(ctx, model, None);
    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyMutationInput" input object type.
pub(crate) fn update_many_input_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let object_name = format!("{}UpdateManyMutationInput", model.name);
    return_cached_input!(ctx, &object_name);

    let input_fields = scalar_input_fields_for_update(ctx, model);
    let input_object = Arc::new(input_object_type(object_name.clone(), input_fields));

    ctx.cache_input_type(object_name, input_object.clone());
    Arc::downgrade(&input_object)
}

fn scalar_input_fields_for_update(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Update",
        model
            .fields()
            .scalar_writable()
            .filter(field_should_be_kept_for_update_input_type)
            .collect(),
        |ctx, f: ScalarFieldRef| scalar_update_field_type_mapper(ctx, &f),
        false,
    )
}

fn scalar_update_field_type_mapper(ctx: &mut BuilderContext, field: &ScalarFieldRef) -> InputType {
    if field.is_list {
        map_optional_input_type(field)
    } else {
        let typ = match &field.type_identifier {
            TypeIdentifier::Float => operations_object_type(ctx, "Float", field, true),
            TypeIdentifier::Int => operations_object_type(ctx, "Int", field, true),
            TypeIdentifier::String => operations_object_type(ctx, "String", field, false),
            TypeIdentifier::Boolean => operations_object_type(ctx, "Bool", field, false),
            TypeIdentifier::Enum(e) => operations_object_type(ctx, &format!("Enum{}", e), field, false),
            TypeIdentifier::Json => operations_object_type(ctx, "Json", field, false),
            TypeIdentifier::DateTime => operations_object_type(ctx, "DateTime", field, false),
            TypeIdentifier::UUID => operations_object_type(ctx, "Uuid", field, false),
        };

        wrap_opt_input_object(typ)
    }
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
    obj.set_one_of(true);

    let obj = Arc::new(obj);
    let nullable_field_type = map_optional_input_type(field);
    let mapped = map_required_input_type(field);

    let non_nullable_field_type = if let InputType::Null(typ) = mapped {
        InputType::opt(*typ)
    } else {
        InputType::opt(mapped)
    };

    ctx.cache_input_type(name, obj.clone());

    let mut fields = vec![input_field("set", nullable_field_type, None)];

    if with_number_operators && feature_flags::get().atomicNumberOperations {
        fields.push(input_field("increment", non_nullable_field_type.clone(), None));
        fields.push(input_field("decrement", non_nullable_field_type.clone(), None));
        fields.push(input_field("multiply", non_nullable_field_type.clone(), None));
        fields.push(input_field("divide", non_nullable_field_type, None));
    }

    obj.set_fields(fields);

    Arc::downgrade(&obj)
}

/// For update input types only. Compute input fields for relational fields.
/// This recurses into create_input_type (via nested_create_input_field).
/// Todo: This code is fairly similar to "create" relation computation. Let's see if we can dry it up.
fn relation_input_fields_for_update(
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

                let field_type = InputType::opt(InputType::object(input_object));

                Some(input_field(rf.name.clone(), field_type, None))
            }
        })
        .collect()
}

pub(crate) fn nested_upsert_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let nested_update_data_object = nested_update_data(ctx, parent_field);

    if parent_field.is_list {
        nested_upsert_list_input_object(ctx, parent_field, nested_update_data_object)
    } else {
        nested_upsert_nonlist_input_object(ctx, parent_field, nested_update_data_object)
    }
}

/// Builds "<x>UpsertWithWhereUniqueNestedInput" / "<x>UpsertWithWhereUniqueWithout<y>Input" input object types.
fn nested_upsert_list_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
    update_object: InputObjectTypeWeakRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let where_object = filter_input_objects::where_unique_object_type(ctx, &related_model);
    let create_object = create_input_objects::create_input_type(ctx, &related_model, Some(parent_field));

    if where_object.into_arc().is_empty() || create_object.into_arc().is_empty() {
        return None;
    }

    let type_name = format!(
        "{}UpsertWithWhereUniqueWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("where", InputType::object(where_object), None),
                input_field("update", InputType::object(update_object), None),
                input_field("create", InputType::object(create_object), None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}

/// Builds "<x>UpsertNestedInput" / "<x>UpsertWithout<y>Input" input object types.
fn nested_upsert_nonlist_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
    update_object: InputObjectTypeWeakRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let create_object = create_input_objects::create_input_type(ctx, &related_model, Some(parent_field));

    if create_object.into_arc().is_empty() {
        return None;
    }

    let type_name = format!(
        "{}UpsertWithout{}Input",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("update", InputType::object(update_object), None),
                input_field("create", InputType::object(create_object), None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}

/// Builds "<x>UpdateManyWithWhereNestedInput" input object type.
pub(crate) fn nested_update_many_input_object(
    ctx: &mut BuilderContext,
    field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    if field.is_list {
        let related_model = field.related_model();
        let type_name = format!("{}UpdateManyWithWhereNestedInput", related_model.name);

        match ctx.get_input_type(&type_name) {
            None => {
                let data_input_object = nested_update_many_data(ctx, field);
                let input_object = Arc::new(init_input_object_type(type_name.clone()));
                ctx.cache_input_type(type_name, input_object.clone());

                let where_input_object = filter_input_objects::scalar_filter_object_type(ctx, &related_model);

                input_object.set_fields(vec![
                    input_field("where", InputType::object(where_input_object), None),
                    input_field("data", InputType::object(data_input_object), None),
                ]);

                Some(Arc::downgrade(&input_object))
            }
            x => x,
        }
    } else {
        None
    }
}

/// Builds "<x>UpdateWithWhereUniqueNestedInput" / "<x>UpdateWithWhereUniqueWithout<y>Input" input object types.
pub(crate) fn input_object_type_nested_update(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let nested_input_object = nested_update_data(ctx, parent_field);

    if parent_field.is_list {
        let where_input_object = filter_input_objects::where_unique_object_type(ctx, &related_model);
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
            input_field("data", InputType::object(nested_input_object), None),
        ];

        input_object.set_fields(fields);
        Arc::downgrade(&input_object)
    } else {
        nested_input_object
    }
}

/// Builds "<x>UpdateDataInput" / "<x>UpdateWithout<y>DataInput" ubout input object types.
fn nested_update_data(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let type_name = format!(
        "{}UpdateWithout{}DataInput",
        related_model.name,
        capitalize(&parent_field.related_field().name)
    );

    return_cached_input!(ctx, &type_name);

    let input_object = Arc::new(init_input_object_type(&type_name));
    ctx.cache_input_type(type_name, input_object.clone());

    let mut fields = scalar_input_fields_for_update(ctx, &related_model);
    let mut relational_input_fields = relation_input_fields_for_update(ctx, &related_model, Some(parent_field));

    fields.append(&mut relational_input_fields);
    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

/// Builds "<x>UpdateManyDataInput" input object type.
fn nested_update_many_data(ctx: &mut BuilderContext, parent_field: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = parent_field.related_model();
    let type_name = format!("{}UpdateManyDataInput", related_model.name);

    return_cached_input!(ctx, &type_name);

    let input_object = Arc::new(init_input_object_type(type_name.clone()));
    ctx.cache_input_type(type_name, input_object.clone());

    let fields = scalar_input_fields_for_update(ctx, &related_model);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn field_should_be_kept_for_update_input_type(field: &ScalarFieldRef) -> bool {
    // We forbid updating auto-increment integer unique fields as this can create problems with the
    // underlying sequences.
    !field.is_auto_generated_int_id
        && !matches!(
            (&field.type_identifier, field.unique(), field.is_autoincrement),
            (TypeIdentifier::Int, true, true)
        )
}
