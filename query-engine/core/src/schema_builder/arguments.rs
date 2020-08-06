use super::*;

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Argument {
    let where_object = input_types::filter_input_objects::where_object_type(ctx, model);

    argument("where", InputType::opt(InputType::object(where_object)), None)
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Argument> {
    let input_object_type = input_types::filter_input_objects::where_unique_object_type(ctx, &model);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(argument("where", InputType::object(input_object_type), None))
    }
}

/// Builds "data" argument intended for the create field.
pub(crate) fn create_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<Argument>> {
    let input_object_type = input_types::create_input_objects::create_input_type(ctx, model, None);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(vec![argument("data", InputType::object(input_object_type), None)])
    }
}

/// Builds "where" (unique) argument intended for the delete field.
pub(crate) fn delete_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<Argument>> {
    where_unique_argument(ctx, model).map(|arg| vec![arg])
}

/// Builds "where" (unique) and "data" arguments intended for the update field.
pub(crate) fn update_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<Argument>> {
    where_unique_argument(ctx, model).map(|unique_arg| {
        let input_object = input_types::update_input_objects::update_input_type(ctx, model);
        let input_object_type = InputType::object(input_object);

        vec![argument("data", input_object_type, None), unique_arg]
    })
}

/// Builds "where" (unique), "create", and "update" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<Argument>> {
    where_unique_argument(ctx, model).and_then(|where_unique_arg| {
        let update_type = input_types::update_input_objects::update_input_type(ctx, model);
        let create_type = input_types::create_input_objects::create_input_type(ctx, model, None);

        if update_type.into_arc().is_empty() || create_type.into_arc().is_empty() {
            None
        } else {
            Some(vec![
                where_unique_arg,
                argument("create", InputType::object(create_type), None),
                argument("update", InputType::object(update_type), None),
            ])
        }
    })
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<Argument> {
    let update_object = input_types::update_input_objects::update_many_input_type(ctx, model);
    let where_arg = where_argument(ctx, model);

    vec![argument("data", InputType::object(update_object), None), where_arg]
}

/// Builds "where" argument intended for the delete many field.
pub(crate) fn delete_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<Argument> {
    let where_arg = where_argument(ctx, model);

    vec![where_arg]
}

/// Builds "many records where" arguments based on the given model and field.
pub(crate) fn many_records_field_arguments(ctx: &mut BuilderContext, field: &ModelField) -> Vec<Argument> {
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
pub(crate) fn many_records_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<Argument> {
    let unique_input_type = InputType::opt(InputType::object(
        input_types::filter_input_objects::where_unique_object_type(ctx, model),
    ));

    let mut args = vec![
        where_argument(ctx, &model),
        order_by_argument(ctx, &model),
        argument("cursor", unique_input_type.clone(), None),
        argument("take", InputType::opt(InputType::int()), None),
        argument("skip", InputType::opt(InputType::int()), None),
    ];

    if feature_flags::get().distinct {
        let enum_type = Arc::new(EnumType::FieldRef(FieldRefEnumType {
            name: format!("{}DistinctFieldEnum", capitalize(&model.name)),
            values: model
                .fields()
                .scalar()
                .into_iter()
                .map(|field| (field.name.clone(), field))
                .collect(),
        }));

        args.push(argument(
            "distinct",
            InputType::opt(InputType::list(InputType::Enum(enum_type))),
            None,
        ));
    }

    args
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Argument {
    let object_type = input_types::order_by_object_type(ctx, model);

    argument(
        "orderBy",
        InputType::opt(InputType::list(InputType::object(object_type))),
        None,
    )
}
