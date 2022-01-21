use super::*;
use prisma_models::dml::DefaultValue;

#[tracing::instrument(skip(ctx, model, parent_field))]
pub(crate) fn create_one_input_types(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<InputType> {
    let checked_input = InputType::object(checked_create_input_type(ctx, model, parent_field));
    let unchecked_input = InputType::object(unchecked_create_input_type(ctx, model, parent_field));

    // If the inputs are equal, only use one.
    if checked_input == unchecked_input {
        vec![checked_input]
    } else {
        vec![checked_input, unchecked_input]
    }
}

/// Builds the create input type (<x>CreateInput / <x>CreateWithout<y>Input)
/// Also valid for nested inputs. A nested input is constructed if the `parent_field` is provided.
/// "Checked" input refers to disallowing writing relation scalars directly, as it can lead to unintended
/// data integrity violations if used incorrectly.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn checked_create_input_type(
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

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    // Compute input fields for scalar fields.
    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar_writable()
        .into_iter()
        .filter(|f| field_should_be_kept_for_checked_create_input_type(&f))
        .collect();

    // Todo(dom): This is duplicated code with unchecked.
    let mut fields = input_fields::scalar_input_fields(
        ctx,
        scalar_fields,
        field_create_input,
        |ctx, f, _| input_fields::scalar_list_input_field_mapper(ctx, model.name.clone(), "Create", f, true),
        true,
    );

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_checked_create(ctx, model, parent_field);

    // Compute input fields for composite fields.
    let mut composite_fields = fields::composite_create_input_fields(ctx, model);

    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// For checked create input types only. Compute input fields for relational fields.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn relation_input_fields_for_checked_create(
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
            let arity_part = if rf.is_list() { "NestedMany" } else { "NestedOne" };
            let without_part = format!("Without{}", capitalize(&related_field.name));
            let ident = Identifier::new(
                format!("{}Create{}{}Input", related_model.name, arity_part, without_part),
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
                    .as_scalar_fields()
                    .expect("Expected linking fields to be scalar.")
                    .into_iter()
                    .all(|scalar_field| scalar_field.default_value.is_some());

                let input_field = input_field(rf.name.clone(), InputType::object(input_object), None);

                if rf.is_required() && !all_required_scalar_fields_have_defaults {
                    Some(input_field)
                } else {
                    Some(input_field.optional())
                }
            }
        })
        .collect()
}

fn field_should_be_kept_for_checked_create_input_type(field: &ScalarFieldRef) -> bool {
    !field.is_auto_generated_int_id
}

/// Builds the create input type (<x>UncheckedCreateInput / <x>UncheckedCreateWithout<y>Input)
/// Also valid for nested inputs. A nested input is constructed if the `parent_field` is provided.
/// "Unchecked" input refers to allowing to write _all_ scalars on a model directly, which can
/// lead to unintended data integrity violations if used incorrectly.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn unchecked_create_input_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    // We allow creation from both sides of the relation - which would lead to an endless loop of input types
    // if we would allow to create the parent from a child create that is already a nested create.
    // To solve it, we remove the parent relation from the input ("Without<Parent>").
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!(
            "{}UncheckedCreateWithout{}Input",
            model.name,
            capitalize(f.name.as_str())
        ),
        _ => format!("{}UncheckedCreateInput", model.name),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

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

    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|sf| !linking_fields.contains(sf))
        .collect();

    let mut fields = input_fields::scalar_input_fields(
        ctx,
        scalar_fields,
        field_create_input,
        |ctx, f, _| input_fields::scalar_list_input_field_mapper(ctx, model.name.clone(), "Create", f, true),
        true,
    );

    // Compute input fields for relational fields.
    let mut relational_fields = relation_input_fields_for_unchecked_create(ctx, model, parent_field);

    fields.append(&mut relational_fields);

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

/// For unchecked create input types only. Compute input fields for relational fields.
#[tracing::instrument(skip(ctx, model, parent_field))]
fn relation_input_fields_for_unchecked_create(
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
            let arity_part = if rf.is_list() { "NestedMany" } else { "NestedOne" };
            let without_part = format!("Without{}", capitalize(&child_field.name));
            let ident = Identifier::new(
                format!("{}UncheckedCreate{}{}Input", child_model.name, arity_part, without_part),
                PRISMA_NAMESPACE,
            );

            let field_is_opposite_relation_field =
                parent_field.filter(|pf| pf.related_field().name == rf.name).is_some();

            // Filter out all inlined relations on `child_model`.
            // -> Only relations that point to other models are allowed in the unchecked input.
            if field_is_opposite_relation_field || !child_field.is_inlined_on_enclosing_model() {
                None
            } else {
                let input_object = match ctx.get_input_type(&ident) {
                    Some(t) => t,
                    None => {
                        let input_object = Arc::new(init_input_object_type(ident.clone()));
                        ctx.cache_input_type(ident, input_object.clone());

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
                    .as_scalar_fields()
                    .expect("Expected linking fields to be scalar.")
                    .into_iter()
                    .all(|scalar_field| scalar_field.default_value.is_some());

                let input_field = input_field(rf.name.clone(), InputType::object(input_object), None);

                if rf.is_required() && !all_required_scalar_fields_have_defaults {
                    Some(input_field)
                } else {
                    Some(input_field.optional())
                }
            }
        })
        .collect()
}

pub(crate) fn field_create_input(
    ctx: &mut BuilderContext,
    f: ScalarFieldRef,
    default: Option<DefaultValue>,
) -> InputField {
    let typ = map_scalar_input_type_for_field(ctx, &f);
    let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

    match &f.type_identifier {
        TypeIdentifier::Json if has_adv_json => {
            let enum_type = json_null_input_enum(!f.is_required());

            input_field(f.name.clone(), vec![InputType::Enum(enum_type), typ], default)
                .optional_if(!f.is_required() || f.default_value.is_some() || f.is_updated_at)
        }

        _ => input_field(f.name.clone(), typ, default)
            .optional_if(!f.is_required() || f.default_value.is_some() || f.is_updated_at)
            .nullable_if(!f.is_required()),
    }
}
