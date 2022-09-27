use super::*;
use crate::input_types::objects::order_by_objects::OrderByOptions;
use crate::mutations::create_one;
use constants::args;
use objects::*;
use prisma_models::{prelude::ParentContainer, CompositeFieldRef};

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext, model: &ModelRef) -> InputField {
    let where_object = filter_objects::where_object_type(ctx, model);

    input_field(args::WHERE, InputType::object(where_object), None).optional()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Option<InputField> {
    let input_object_type = filter_objects::where_unique_object_type(ctx, model);

    if input_object_type.into_arc().is_empty() {
        None
    } else {
        Some(input_field(args::WHERE, InputType::object(input_object_type), None))
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
        let create_types = create_one::create_one_input_types(ctx, model, None);

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
pub(crate) fn many_records_output_field_arguments(ctx: &mut BuilderContext, field: &ModelField) -> Vec<InputField> {
    match field {
        ModelField::Scalar(_) => vec![],

        // To-many relation.
        ModelField::Relation(rf) if rf.is_list() => relation_selection_arguments(ctx, &rf.related_model(), true),

        // To-one relation.
        ModelField::Relation(_) => vec![],

        // To-many composite.
        ModelField::Composite(cf) if cf.is_list() => composite_selection_arguments(ctx, cf),

        // To-one composite.
        ModelField::Composite(_) => vec![],
    }
}

/// Builds "many records where" arguments for to-many relation selection sets.
pub(crate) fn relation_selection_arguments(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_distinct: bool,
) -> Vec<InputField> {
    let unique_input_type = InputType::object(filter_objects::where_unique_object_type(ctx, model));
    let order_by_options = OrderByOptions {
        include_relations: true,
        include_scalar_aggregations: false,
        include_full_text_search: ctx.can_full_text_search(),
    };

    let mut args = vec![
        where_argument(ctx, model),
        order_by_argument(ctx, &model.into(), &order_by_options),
        input_field(args::CURSOR, unique_input_type, None).optional(),
        input_field(args::TAKE, InputType::int(), None).optional(),
        input_field(args::SKIP, InputType::int(), None).optional(),
    ];

    if include_distinct {
        args.push(
            input_field(
                args::DISTINCT,
                InputType::list(InputType::Enum(model_field_enum(ctx, model))),
                None,
            )
            .optional(),
        );
    }

    args
}

/// Builds "many composite where" arguments for to-many composite selection sets.
pub(crate) fn composite_selection_arguments(_ctx: &mut BuilderContext, _cf: &CompositeFieldRef) -> Vec<InputField> {
    vec![]
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(
    ctx: &mut BuilderContext,
    container: &ParentContainer,
    options: &OrderByOptions,
) -> InputField {
    let order_object_type = InputType::object(order_by_objects::order_by_object_type(ctx, container, options));

    input_field(
        args::ORDER_BY,
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
}

pub(crate) fn group_by_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let field_enum_type = InputType::Enum(model_field_enum(ctx, model));

    vec![
        where_argument(ctx, model),
        order_by_argument(ctx, &model.into(), &OrderByOptions::new().with_aggregates()),
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
