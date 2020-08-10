use super::*;

/// Builds "<x>CreateOrConnectNestedInput" input object types.
pub(crate) fn nested_connect_or_create_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();

    let where_object = filter_input_objects::where_unique_object_type(ctx, &related_model);
    let create_object = create_input_type(ctx, &related_model, Some(parent_field));

    if where_object.into_arc().is_empty() {
        return None;
    }

    let type_name = format!(
        "{}CreateOrConnectWithout{}Input",
        related_model.name,
        parent_field.model().name
    );

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("where", InputType::object(where_object), None),
                input_field("create", InputType::object(create_object), None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}

/// Builds the create input type (<x>CreateInput / <x>CreateWithout<y>Input)
pub(crate) fn create_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}CreateWithout{}Input", model.name, capitalize(f.name.as_str())),
        _ => format!("{}CreateInput", model.name),
    };

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));

    // Cache empty object for circuit breaking
    ctx.cache_input_type(name, input_object.clone());

    // Compute input fields for scalar fields.
    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar_writable()
        .into_iter()
        .filter(|f| field_should_be_kept_for_create_input_type(&f))
        .collect();

    let mut fields = input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Create",
        scalar_fields,
        |f: ScalarFieldRef| {
            if f.is_required && f.default_value.is_none() && (f.is_created_at() || f.is_updated_at()) {
                //todo shouldnt these also be Default Value expressions at some point?
                map_optional_input_type(&f)
            } else if f.is_required && f.default_value.is_none() {
                map_required_input_type(&f)
            } else {
                map_optional_input_type(&f)
            }
        },
        true,
    );

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_create(ctx, model, parent_field);

    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// For create input types only. Compute input fields for relational fields.
/// This recurses into create_input_type (via nested_create_input_field).
fn relation_input_fields_for_create(
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
            let arity_part = if rf.is_list { "Many" } else { "One" };
            let without_part = format!("Without{}", capitalize(&related_field.name));
            let input_name = format!("{}Create{}{}Input", related_model.name, arity_part, without_part);
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

                        // Enqueue the nested create input for its fields to be
                        // created at a later point, to avoid recursing too deep
                        // (that has caused stack overflows on large schemas in
                        // the past).
                        ctx.nested_create_inputs_queue
                            .push((Arc::clone(&input_object), Arc::clone(&rf)));

                        Arc::downgrade(&input_object)
                    }
                };

                let all_required_scalar_fields_have_defaults = rf
                    .linking_fields()
                    .scalar_fields()
                    .all(|scalar_field| scalar_field.default_value.is_some());

                let input_type = InputType::object(input_object);
                let input_field = if rf.is_required && !all_required_scalar_fields_have_defaults {
                    input_field(rf.name.clone(), input_type, None)
                } else {
                    input_field(rf.name.clone(), InputType::opt(input_type), None)
                };

                Some(input_field)
            }
        })
        .collect()
}

fn field_should_be_kept_for_create_input_type(field: &ScalarFieldRef) -> bool {
    !field.is_auto_generated_int_id
}
