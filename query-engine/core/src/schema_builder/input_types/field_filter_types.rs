use super::*;
use datamodel_connector::ConnectorCapability;
use prisma_models::{dml::DefaultValue, PrismaValue};

/// Builds filter types for the given model field.
pub(crate) fn get_field_filter_types(ctx: &mut BuilderContext, field: &ModelField) -> Vec<InputType> {
    match field {
        ModelField::Relation(rf) => {
            let mut types = vec![InputType::object(full_relation_filter(ctx, rf))];
            types.extend(mto1_relation_filter_shorthand_types(ctx, rf));
            types
        }
        ModelField::Scalar(sf) if field.is_list() => vec![InputType::object(scalar_list_filter_type(ctx, sf))],
        ModelField::Scalar(sf) => {
            let mut types = vec![InputType::object(full_scalar_filter_type(ctx, sf, false))];

            if sf.type_identifier != TypeIdentifier::Json {
                types.push(map_scalar_input_type(sf)); // Scalar equality shorthand

                if !sf.is_required {
                    types.push(InputType::null()); // Scalar null-equality shorthand
                }
            }

            types
        }
    }
}

/// Builds shorthand relation equality (`is`) filter for to-one: `where: { relation_field: { ... } }` (no `is` in between).
/// If the field is also not required, null is also added as possible type.
fn mto1_relation_filter_shorthand_types(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> Vec<InputType> {
    let mut types = vec![];

    if !rf.is_list {
        let related_model = rf.related_model();
        let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);
        types.push(InputType::object(related_input_type));

        if !rf.is_required {
            types.push(InputType::null());
        }
    }

    types
}

fn full_relation_filter(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = rf.related_model();
    let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);
    let list = if rf.is_list { "List" } else { "" };
    let filter_name = format!("{}{}RelationFilter", capitalize(&related_model.name), list);

    return_cached_input!(ctx, &filter_name);
    let object = Arc::new(init_input_object_type(filter_name.clone()));
    ctx.cache_input_type(filter_name, object.clone());

    let fields = if rf.is_list {
        vec![
            input_field("every", InputType::object(related_input_type.clone()), None).optional(),
            input_field("some", InputType::object(related_input_type.clone()), None).optional(),
            input_field("none", InputType::object(related_input_type.clone()), None).optional(),
        ]
    } else {
        vec![
            input_field("is", InputType::object(related_input_type.clone()), None)
                .optional()
                .nullable_if(!rf.is_required),
            input_field("isNot", InputType::object(related_input_type), None)
                .optional()
                .nullable_if(!rf.is_required),
        ]
    };

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn scalar_list_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let name = scalar_filter_name(sf, false);
    return_cached_input!(ctx, &name);

    let object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, object.clone());

    let fields = equality_filters(sf).collect();
    object.set_fields(fields);

    Arc::downgrade(&object)
}

fn full_scalar_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef, nested: bool) -> InputObjectTypeWeakRef {
    let name = scalar_filter_name(sf, nested);
    return_cached_input!(ctx, &name);

    let object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, object.clone());

    let mut fields: Vec<_> = match sf.type_identifier {
        TypeIdentifier::String | TypeIdentifier::UUID => equality_filters(sf)
            .chain(inclusion_filters(sf))
            .chain(alphanumeric_filters(sf))
            .chain(string_filters(sf))
            .chain(query_mode_field(ctx, nested))
            .collect(),

        TypeIdentifier::Int | TypeIdentifier::Float | TypeIdentifier::DateTime => equality_filters(sf)
            .chain(inclusion_filters(sf))
            .chain(alphanumeric_filters(sf))
            .collect(),

        TypeIdentifier::Boolean | TypeIdentifier::Json => equality_filters(sf).collect(),
        TypeIdentifier::Enum(_) => equality_filters(sf).chain(inclusion_filters(sf)).collect(),
    };

    // Shorthand `not equals` filter, skips the nested object filter.
    let mut not_types = vec![map_scalar_input_type(sf)];

    if sf.type_identifier != TypeIdentifier::Json {
        // Full nested filter. Only available on non-JSON fields.
        not_types.push(InputType::object(full_scalar_filter_type(ctx, sf, true)));
    }

    let not_field = input_field("not", not_types, None)
        .optional()
        .nullable_if(!sf.is_required);

    fields.push(not_field);
    object.set_fields(fields);

    Arc::downgrade(&object)
}

fn equality_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    vec![input_field("equals", map_scalar_input_type(sf), None)
        .optional()
        .nullable_if(!sf.is_required)]
    .into_iter()
}

fn inclusion_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let typ = InputType::list(map_scalar_input_type(sf));

    vec![
        input_field("in", typ.clone(), None)
            .optional()
            .nullable_if(!sf.is_required),
        input_field("notIn", typ, None) // Kept for legacy reasons!
            .optional()
            .nullable_if(!sf.is_required),
    ]
    .into_iter()
}

fn alphanumeric_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = map_scalar_input_type(sf);

    vec![
        input_field("lt", mapped_type.clone(), None).optional(),
        input_field("lte", mapped_type.clone(), None).optional(),
        input_field("gt", mapped_type.clone(), None).optional(),
        input_field("gte", mapped_type.clone(), None).optional(),
    ]
    .into_iter()
}

fn string_filters(sf: &ScalarFieldRef) -> impl Iterator<Item = InputField> {
    let mapped_type = map_scalar_input_type(sf);

    vec![
        input_field("contains", mapped_type.clone(), None).optional(),
        input_field("startsWith", mapped_type.clone(), None).optional(),
        input_field("endsWith", mapped_type.clone(), None).optional(),
    ]
    .into_iter()
}

fn query_mode_field(ctx: &BuilderContext, nested: bool) -> impl Iterator<Item = InputField> {
    // Limit query mode field to the topmost filter level.
    // Only build mode field for connectors with insensitive filter support.
    let fields = if !nested && ctx.capabilities.contains(ConnectorCapability::InsensitiveFilters) {
        let enum_type = Arc::new(string_enum_type(
            "QueryMode",
            vec!["default".to_owned(), "insensitive".to_owned()],
        ));

        let field = input_field(
            "mode",
            InputType::enum_type(enum_type),
            Some(DefaultValue::Single(PrismaValue::Enum("default".to_owned()))),
        )
        .optional();

        vec![field]
    } else {
        vec![]
    };

    fields.into_iter()
}

fn scalar_filter_name(sf: &ScalarFieldRef, nested: bool) -> String {
    let list = if sf.is_list { "List" } else { "" };
    let nullable = if sf.is_required { "" } else { "Nullable" };
    let nested = if nested { "Nested" } else { "" };

    match sf.type_identifier {
        TypeIdentifier::UUID => format!("{}Uuid{}{}Filter", nested, nullable, list),
        TypeIdentifier::String => format!("{}String{}{}Filter", nested, nullable, list),
        TypeIdentifier::Int => format!("{}Int{}{}Filter", nested, nullable, list),
        TypeIdentifier::Float => format!("{}Float{}{}Filter", nested, nullable, list),
        TypeIdentifier::Boolean => format!("{}Bool{}{}Filter", nested, nullable, list),
        TypeIdentifier::DateTime => format!("{}DateTime{}{}Filter", nested, nullable, list),
        TypeIdentifier::Json => format!("{}Json{}{}Filter", nested, nullable, list),
        TypeIdentifier::Enum(ref e) => format!("{}Enum{}{}{}Filter", nested, e, nullable, list),
    }
}
