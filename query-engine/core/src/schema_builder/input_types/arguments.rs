use super::*;
use constants::args;
use datamodel_connector::ConnectorCapability;

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext, model: &ModelRef) -> InputField {
    let where_object = filter_objects::where_object_type(ctx, model);

    input_field(args::WHERE, InputType::object(where_object), None).optional()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Option<InputField> {
    let input_object_type = filter_objects::where_unique_object_type(ctx, &model);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(input_field(args::WHERE, InputType::object(input_object_type), None))
    }
}

/// Builds "data" argument intended for the create field.
/// The data argument is not present if no data can be created.
pub(crate) fn create_one_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    let create_types = create_one_objects::create_one_input_types(ctx, model, None);
    let any_empty = create_types.iter().any(|typ| typ.is_empty());
    let all_empty = create_types.iter().all(|typ| typ.is_empty());

    if all_empty {
        None
    } else {
        Some(vec![input_field(args::DATA, create_types, None).optional_if(any_empty)])
    }
}

/// Builds "where" (unique) argument intended for the delete field.
pub(crate) fn delete_one_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|arg| vec![arg])
}

/// Builds "where" (unique) and "data" arguments intended for the update field.
pub(crate) fn update_one_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|unique_arg| {
        let update_types = update_one_objects::update_one_input_types(ctx, model, None);

        vec![input_field(args::DATA, update_types, None), unique_arg]
    })
}

/// Builds "where" (unique), "create", and "update" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).and_then(|where_unique_arg| {
        let update_types = update_one_objects::update_one_input_types(ctx, model, None);
        let create_types = create_one_objects::create_one_input_types(ctx, model, None);

        if update_types.iter().all(|typ| typ.is_empty()) || create_types.iter().all(|typ| typ.is_empty()) {
            None
        } else {
            Some(vec![
                where_unique_arg,
                input_field(args::CREATE, create_types, None),
                input_field(args::UPDATE, update_types, None),
            ])
        }
    })
}

/// Builds "skip_duplicates" and "data" arguments intended for the create many field.
pub(crate) fn create_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let create_many_type = InputType::object(create_many_objects::create_many_object_type(ctx, model, None));
    let data_arg = input_field("data", InputType::list(create_many_type), None);

    if ctx.capabilities.contains(ConnectorCapability::CreateSkipDuplicates) {
        let skip_arg = input_field(args::SKIP_DUPLICATES, InputType::boolean(), None).optional();

        vec![data_arg, skip_arg]
    } else {
        vec![data_arg]
    }
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let update_many_types = update_many_objects::update_many_input_types(ctx, model, None);
    let where_arg = where_argument(ctx, model);

    vec![input_field(args::DATA, update_many_types, None), where_arg]
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
            many_records_arguments(ctx, &rf.related_model(), true)
        }
        ModelField::Relation(rf) if rf.is_list && rf.related_model().is_embedded => vec![],
        ModelField::Relation(rf) if !rf.is_list => vec![],
        _ => unreachable!(),
    }
}

/// Builds "many records where" arguments solely based on the given model.
pub(crate) fn many_records_arguments(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_distinct: bool,
) -> Vec<InputField> {
    let unique_input_type = InputType::object(filter_objects::where_unique_object_type(ctx, model));

    let mut args = vec![
        where_argument(ctx, &model),
        order_by_argument(
            ctx,
            &model,
            true,
            false,
            ctx.has_feature(&PreviewFeature::FullTextSearch)
                && ctx.has_capability(ConnectorCapability::FullTextSearchWithoutIndex),
        ),
        input_field(args::CURSOR, unique_input_type, None).optional(),
        input_field(args::TAKE, InputType::int(), None).optional(),
        input_field(args::SKIP, InputType::int(), None).optional(),
    ];

    if include_distinct {
        args.push(
            input_field(
                args::DISTINCT,
                InputType::list(InputType::Enum(model_field_enum(model))),
                None,
            )
            .optional(),
        );
    }

    args
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_relations: bool,
    include_scalar_aggregations: bool,
    include_full_text_search: bool,
) -> InputField {
    let order_object_type = InputType::object(order_by_objects::order_by_object_type(
        ctx,
        model,
        include_relations,
        include_scalar_aggregations,
        include_full_text_search,
    ));

    input_field(
        args::ORDER_BY,
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
}

pub(crate) fn group_by_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let field_enum_type = InputType::Enum(model_field_enum(model));

    vec![
        where_argument(ctx, &model),
        order_by_argument(ctx, &model, false, true, false),
        input_field(
            args::BY,
            vec![InputType::list(field_enum_type.clone()), field_enum_type],
            None,
        ),
        input_field(
            args::HAVING,
            InputType::object(filter_objects::scalar_filter_object_type(ctx, model, true)),
            None,
        )
        .optional(),
        input_field(args::TAKE, InputType::int(), None).optional(),
        input_field(args::SKIP, InputType::int(), None).optional(),
    ]
}
