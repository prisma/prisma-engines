use super::*;

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext, model: &ModelRef) -> InputField {
    let where_object = input_types::filter_input_objects::where_object_type(ctx, model);

    input_field("where", InputType::object(where_object), None).optional()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Option<InputField> {
    let input_object_type = input_types::filter_input_objects::where_unique_object_type(ctx, &model);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(input_field("where", InputType::object(input_object_type), None))
    }
}

/// Builds "data" argument intended for the create field.
pub(crate) fn create_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    let input_object_type = input_types::create_input_objects::create_input_type(ctx, model, None);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(vec![input_field("data", InputType::object(input_object_type), None)])
    }
}

/// Builds "where" (unique) argument intended for the delete field.
pub(crate) fn delete_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|arg| vec![arg])
}

/// Builds "where" (unique) and "data" arguments intended for the update field.
pub(crate) fn update_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|unique_arg| {
        let input_object = input_types::update_input_objects::update_input_type(ctx, model);
        let input_object_type = InputType::object(input_object);

        vec![input_field("data", input_object_type, None), unique_arg]
    })
}

/// Builds "where" (unique), "create", and "update" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).and_then(|where_unique_arg| {
        let update_type = input_types::update_input_objects::update_input_type(ctx, model);
        let create_type = input_types::create_input_objects::create_input_type(ctx, model, None);

        if update_type.into_arc().is_empty() || create_type.into_arc().is_empty() {
            None
        } else {
            Some(vec![
                where_unique_arg,
                input_field("create", InputType::object(create_type), None),
                input_field("update", InputType::object(update_type), None),
            ])
        }
    })
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let update_object = input_types::update_input_objects::update_many_input_type(ctx, model);
    let where_arg = where_argument(ctx, model);

    vec![input_field("data", InputType::object(update_object), None), where_arg]
}

/// Builds "where" argument intended for the delete many field.
pub(crate) fn delete_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let where_arg = where_argument(ctx, model);

    vec![where_arg]
}

/// Builds "many records where" arguments based on the given model and field.
pub(crate) fn many_records_field_arguments(ctx: &mut BuilderContext, field: &ModelField) -> Vec<InputField> {
    match field {
        ModelField::Scalar(_) => vec![],
        ModelField::Relation(rf) if rf.is_list && !rf.related_model().is_embedded => {
            many_records_arguments(ctx, &rf.related_model())
        }
        ModelField::Relation(rf) if rf.is_list && rf.related_model().is_embedded => vec![],
        ModelField::Relation(rf) if !rf.is_list => vec![],
        _ => unreachable!(),
    }
}

/// Builds "many records where" arguments solely based on the given model.
pub(crate) fn many_records_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let unique_input_type = InputType::object(input_types::filter_input_objects::where_unique_object_type(ctx, model));

    let mut args = vec![
        where_argument(ctx, &model),
        order_by_argument(ctx, &model),
        input_field("cursor", unique_input_type, None).optional(),
        input_field("take", InputType::int(), None).optional(),
        input_field("skip", InputType::int(), None).optional(),
    ];

    let enum_type = Arc::new(EnumType::FieldRef(FieldRefEnumType {
        name: format!("{}DistinctFieldEnum", capitalize(&model.name)),
        values: model
            .fields()
            .scalar()
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect(),
    }));

    args.push(input_field("distinct", InputType::list(InputType::Enum(enum_type)), None).optional());
    args
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(ctx: &mut BuilderContext, model: &ModelRef) -> InputField {
    let order_object_type = InputType::object(input_types::order_by_object_type(ctx, model));

    input_field(
        "orderBy",
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
}
