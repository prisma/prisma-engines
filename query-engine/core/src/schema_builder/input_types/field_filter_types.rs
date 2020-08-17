use super::*;

/// Builds filter type for the given model field.
pub(crate) fn get_field_filter_type(ctx: &mut BuilderContext, field: &ModelField) -> InputObjectTypeWeakRef {
    match field {
        ModelField::Relation(rf) => relation_filter_type(ctx, rf),
        ModelField::Scalar(sf) if field.is_list() => scalar_list_filter_type(ctx, sf),
        ModelField::Scalar(sf) => scalar_filter_type(ctx, sf),
    }
}

fn relation_filter_type(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = rf.related_model();
    let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);
    let list = if rf.is_list { "List" } else { "" };
    let filter_name = format!("{}{}RelationFilter", capitalize(&related_model.name), list);

    return_cached_input!(ctx, &filter_name);
    let object = Arc::new(init_input_object_type(filter_name.clone()));
    ctx.cache_input_type(filter_name, object.clone());

    let fields = if rf.is_list {
        vec![
            input_field("every", wrap_opt_input_object(related_input_type.clone()), None),
            input_field("some", wrap_opt_input_object(related_input_type.clone()), None),
            input_field("none", wrap_opt_input_object(related_input_type.clone()), None),
        ]
    } else {
        vec![
            input_field(
                "is",
                InputType::opt(InputType::null(InputType::object(related_input_type.clone()))),
                None,
            ),
            input_field(
                "isNot",
                InputType::opt(InputType::null(InputType::object(related_input_type))),
                None,
            ),
        ]
    };

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn scalar_list_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let name = scalar_filter_name(sf);
    return_cached_input!(ctx, &name);

    let object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, object.clone());

    let fields = equality_filters(sf).collect();
    object.set_fields(fields);

    Arc::downgrade(&object)
}

fn scalar_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let name = scalar_filter_name(sf);
    return_cached_input!(ctx, &name);

    let object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, object.clone());

    let mut fields: Vec<_> = match sf.type_identifier {
        TypeIdentifier::String | TypeIdentifier::UUID => equality_filters(sf)
            .chain(inclusion_filters(sf))
            .chain(alphanumeric_filters(sf))
            .chain(string_filters(sf))
            .collect(),

        TypeIdentifier::Int | TypeIdentifier::Float | TypeIdentifier::DateTime => equality_filters(sf)
            .chain(inclusion_filters(sf))
            .chain(alphanumeric_filters(sf))
            .collect(),

        TypeIdentifier::Boolean | TypeIdentifier::Json => equality_filters(sf).collect(),
        TypeIdentifier::Enum(_) => equality_filters(sf).chain(inclusion_filters(sf)).collect(),
    };

    fields.push(input_field(
        "not",
        InputType::opt(InputType::object(Arc::downgrade(&object))),
        None,
    ));

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn equality_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = map_optional_input_type(sf);

    vec![input_field("equals", mapped_type.clone(), None)].into_iter()
}

fn inclusion_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = if sf.is_required {
        InputType::opt(InputType::list(map_required_input_type(sf)))
    } else {
        InputType::opt(InputType::null(InputType::list(map_required_input_type(sf))))
    };

    vec![input_field("in", mapped_type.clone(), None)].into_iter()
}

fn alphanumeric_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = map_optional_input_type(sf);

    vec![
        input_field("lt", mapped_type.clone(), None),
        input_field("lte", mapped_type.clone(), None),
        input_field("gt", mapped_type.clone(), None),
        input_field("gte", mapped_type.clone(), None),
    ]
    .into_iter()
}

fn string_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = map_optional_input_type(sf);

    vec![
        input_field("contains", mapped_type.clone(), None),
        input_field("startsWith", mapped_type.clone(), None),
        input_field("endsWith", mapped_type.clone(), None),
    ]
    .into_iter()
}

fn scalar_filter_name(sf: &ScalarFieldRef) -> String {
    let list = if sf.is_list { "List" } else { "" };
    let nullable = if sf.is_required { "" } else { "Nullable" };

    match sf.type_identifier {
        TypeIdentifier::UUID => format!("Uuid{}{}Filter", nullable, list),
        TypeIdentifier::String => format!("String{}{}Filter", nullable, list),
        TypeIdentifier::Int => format!("Int{}{}Filter", nullable, list),
        TypeIdentifier::Float => format!("Float{}{}Filter", nullable, list),
        TypeIdentifier::Boolean => format!("Bool{}{}Filter", nullable, list),
        TypeIdentifier::DateTime => format!("DateTime{}{}Filter", nullable, list),
        TypeIdentifier::Json => format!("Json{}{}Filter", nullable, list),
        TypeIdentifier::Enum(ref e) => format!("Enum{}{}{}Filter", e, nullable, list),
    }
}
