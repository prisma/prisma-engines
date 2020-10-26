use super::*;
use prisma_models::dml::DefaultValue;

pub(crate) fn create_input_types(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let safe_input = InputType::object(safe_create_input_type(ctx, model, parent_field));

    if feature_flags::get().unsafeScalarInputs {
        let unsafe_input = InputType::object(unsafe_create_input_type(ctx, model, parent_field));

        vec![safe_input, unsafe_input]
    } else {
        vec![safe_input]
    }
}

/// Builds the create input type (<x>CreateInput / <x>CreateWithout<y>Input)
/// Also valid for nested inputs. A nested input is constructed if the `parent_field` is provided.
/// "Safe" input refers to disallowing writing relation scalars directly, as it can lead to unintended
/// data integrity violations if used incorrectly.
fn safe_create_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    // We allow creation from both sides of the relation - which would lead to an endless loop of input types
    // if we would allow to create the parent from a child create that is already a nested create.
    // To solve it, we remove the parent relation from the input ("Without<Parent>").
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}CreateWithout{}Input", model.name, capitalize(f.name.as_str())),
        _ => format!("{}CreateInput", model.name),
    };

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

    // Compute input fields for scalar fields.
    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar_writable()
        .into_iter()
        .filter(|f| field_should_be_kept_for_safe_create_input_type(&f))
        .collect();

    let mut fields = input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Create",
        scalar_fields,
        |_, f: ScalarFieldRef, default: Option<DefaultValue>| {
            let typ = map_scalar_input_type(&f);
            if f.is_required && f.default_value.is_none() && (f.is_created_at() || f.is_updated_at()) {
                input_field(f.name.clone(), typ, default)
                    .optional()
                    .nullable_if(!f.is_required)
            } else if f.is_required && f.default_value.is_none() {
                input_field(f.name.clone(), typ, default)
            } else {
                input_field(f.name.clone(), typ, default)
                    .optional()
                    .nullable_if(!f.is_required)
            }
        },
        true,
    );

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_safe_create(ctx, model, parent_field);

    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// For safe create input types only. Compute input fields for relational fields.
fn relation_input_fields_for_safe_create(
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

                let input_field = input_field(rf.name.clone(), InputType::object(input_object), None);

                if rf.is_required && !all_required_scalar_fields_have_defaults {
                    Some(input_field)
                } else {
                    Some(input_field.optional())
                }
            }
        })
        .collect()
}

fn field_should_be_kept_for_safe_create_input_type(field: &ScalarFieldRef) -> bool {
    !field.is_auto_generated_int_id
}

/// Builds the create input type (<x>UnsafeCreateInput / <x>UnsafeCreateWithout<y>Input)
/// Also valid for nested inputs. A nested input is constructed if the `parent_field` is provided.
/// "Safe" input refers to disallowing writing relation scalars directly, as it can lead to unintended
/// data integrity violations if used incorrectly.
fn unsafe_create_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    // We allow creation from both sides of the relation - which would lead to an endless loop of input types
    // if we would allow to create the parent from a child create that is already a nested create.
    // To solve it, we remove the parent relation from the input ("Without<Parent>").
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}UnsafeCreateWithout{}Input", model.name, capitalize(f.name.as_str())),
        _ => format!("{}UnsafeCreateInput", model.name),
    };

    return_cached_input!(ctx, &name);

    let input_object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, input_object.clone());

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

    let mut fields = input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "Create",
        scalar_fields,
        |_, f: ScalarFieldRef, default: Option<DefaultValue>| {
            let typ = map_scalar_input_type(&f);
            if f.is_required && f.default_value.is_none() && (f.is_created_at() || f.is_updated_at()) {
                input_field(f.name.clone(), typ, default)
                    .optional()
                    .nullable_if(!f.is_required)
            } else if f.is_required && f.default_value.is_none() {
                input_field(f.name.clone(), typ, default)
            } else {
                input_field(f.name.clone(), typ, default)
                    .optional()
                    .nullable_if(!f.is_required)
            }
        },
        true,
    );

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_unsafe_create(ctx, model, parent_field);

    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// For safe create input types only. Compute input fields for relational fields.
fn relation_input_fields_for_unsafe_create(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputField> {
    model
        .fields()
        .relation()
        .into_iter()
        .filter_map(|rf| {
            let child_model = rf.related_model();
            let child_field = rf.related_field();

            // Compute input object name
            let arity_part = if rf.is_list { "Many" } else { "One" };
            let without_part = format!("Without{}", capitalize(&child_field.name));
            let input_name = format!("{}UnsafeCreate{}{}Input", child_model.name, arity_part, without_part);

            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            // Filter out all inlined relations on `child_model`.
            // -> Only relations that point to other models are allowed in the unsafe input.
            if field_is_opposite_relation_field || !child_field.is_inlined_on_enclosing_model() {
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

                let input_field = input_field(rf.name.clone(), InputType::object(input_object), None);

                if rf.is_required && !all_required_scalar_fields_have_defaults {
                    Some(input_field)
                } else {
                    Some(input_field.optional())
                }
            }
        })
        .collect()
}

/// Builds "<x>CreateOrConnectNestedInput" input object types.
pub(crate) fn nested_connect_or_create_input_object(
    ctx: &mut BuilderContext,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let where_object = filter_input_objects::where_unique_object_type(ctx, &related_model);

    if where_object.into_arc().is_empty() {
        return None;
    }

    let type_name = format!(
        "{}CreateOrConnectWithout{}Input",
        related_model.name,
        parent_field.model().name
    );

    let create_types = create_input_types(ctx, &related_model, Some(parent_field));

    match ctx.get_input_type(&type_name) {
        None => {
            let input_object = Arc::new(init_input_object_type(type_name.clone()));
            ctx.cache_input_type(type_name, input_object.clone());

            let fields = vec![
                input_field("where", InputType::object(where_object), None),
                input_field("create", create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}
