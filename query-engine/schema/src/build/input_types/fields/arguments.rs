use super::*;
use constants::args;
use input_types::objects::order_by_objects::OrderByOptions;
use mutations::create_one;
use objects::*;
use prisma_models::{prelude::ParentContainer, CompositeFieldRef};

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> InputField {
    let where_object = filter_objects::where_object_type(ctx, model);

    input_field(ctx, args::WHERE, InputType::object(where_object), None).optional()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<InputField> {
    let input_object_type = filter_objects::where_unique_object_type(ctx, model);

    if ctx.db[input_object_type].is_empty() {
        panic!("THIS IS IT HERE");
        None
    } else {
        Some(input_field(
            ctx,
            args::WHERE,
            InputType::object(input_object_type),
            None,
        ))
    }
}

/// Builds "where" (unique) argument intended for the delete field.
pub(crate) fn delete_one_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|arg| vec![arg])
}

/// Builds "where" (unique) and "data" arguments intended for the update field.
pub(crate) fn update_one_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).map(|unique_arg| {
        let update_types = update_one_objects::update_one_input_types(ctx, model, None);

        vec![input_field(ctx, args::DATA, update_types, None), unique_arg]
    })
}

/// Builds "where" (unique), "create", and "update" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<Vec<InputField>> {
    where_unique_argument(ctx, model).and_then(|where_unique_arg| {
        let update_types = update_one_objects::update_one_input_types(ctx, model, None);
        let create_types = create_one::create_one_input_types(ctx, model, None);

        if update_types.iter().all(|typ| typ.is_empty(&ctx.db)) || create_types.iter().all(|typ| typ.is_empty(&ctx.db))
        {
            None
        } else {
            Some(vec![
                where_unique_arg,
                input_field(ctx, args::CREATE, create_types, None),
                input_field(ctx, args::UPDATE, update_types, None),
            ])
        }
    })
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Vec<InputField> {
    let update_many_types = update_many_objects::update_many_input_types(ctx, model, None);
    let where_arg = where_argument(ctx, model);

    vec![input_field(ctx, args::DATA, update_many_types, None), where_arg]
}

/// Builds "where" argument intended for the delete many field.
pub(crate) fn delete_many_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Vec<InputField> {
    let where_arg = where_argument(ctx, model);

    vec![where_arg]
}

/// Builds "many records where" arguments based on the given model and field.
pub(crate) fn many_records_output_field_arguments(ctx: &mut BuilderContext<'_>, field: &ModelField) -> Vec<InputField> {
    match field {
        ModelField::Scalar(_) => vec![],

        // To-many relation.
        ModelField::Relation(rf) if rf.is_list() => {
            relation_to_many_selection_arguments(ctx, &rf.related_model(), true)
        }

        // To-one optional relation.
        ModelField::Relation(rf) if !rf.is_required() && ctx.has_feature(PreviewFeature::ExtendedWhereUnique) => {
            relation_to_one_selection_arguments(ctx, &rf.related_model())
        }

        // To-one required relation.
        ModelField::Relation(_) => vec![],

        // To-many composite.
        ModelField::Composite(cf) if cf.is_list() => composite_selection_arguments(ctx, cf),

        // To-one composite.
        ModelField::Composite(_) => vec![],
    }
}

/// Builds "many records where" arguments for to-many relation selection sets.
pub(crate) fn relation_to_many_selection_arguments(
    ctx: &mut BuilderContext<'_>,
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
        input_field(ctx, args::CURSOR, unique_input_type, None).optional(),
        input_field(ctx, args::TAKE, InputType::int(), None).optional(),
        input_field(ctx, args::SKIP, InputType::int(), None).optional(),
    ];

    if include_distinct {
        let input_type = InputType::list(InputType::Enum(model_field_enum(ctx, model)));
        args.push(input_field(ctx, args::DISTINCT, input_type, None).optional());
    }

    args
}

/// Builds "many records where" arguments for to-many relation selection sets.
pub(crate) fn relation_to_one_selection_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Vec<InputField> {
    vec![where_argument(ctx, model)]
}

/// Builds "many composite where" arguments for to-many composite selection sets.
pub(crate) fn composite_selection_arguments(_ctx: &mut BuilderContext<'_>, _cf: &CompositeFieldRef) -> Vec<InputField> {
    vec![]
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(
    ctx: &mut BuilderContext<'_>,
    container: &ParentContainer,
    options: &OrderByOptions,
) -> InputField {
    let order_object_type = InputType::object(order_by_objects::order_by_object_type(ctx, container, options));

    input_field(
        ctx,
        args::ORDER_BY,
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
}

pub(crate) fn group_by_arguments(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Vec<InputField> {
    let field_enum_type = InputType::Enum(model_field_enum(ctx, model));
    let filter_object = InputType::object(filter_objects::scalar_filter_object_type(ctx, model, true));

    vec![
        where_argument(ctx, model),
        order_by_argument(ctx, &model.into(), &OrderByOptions::new().with_aggregates()),
        input_field(
            ctx,
            args::BY,
            vec![InputType::list(field_enum_type.clone()), field_enum_type],
            None,
        ),
        input_field(ctx, args::HAVING, filter_object, None).optional(),
        input_field(ctx, args::TAKE, InputType::int(), None).optional(),
        input_field(ctx, args::SKIP, InputType::int(), None).optional(),
    ]
}
