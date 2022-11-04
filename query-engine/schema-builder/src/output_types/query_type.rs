use super::*;
use input_types::fields::arguments;

/// Builds the root `Query` type.
pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    let fields: Vec<_> = ctx
        .models()
        .into_iter()
        .flat_map(|model| {
            let mut vec = vec![
                find_first_field(ctx, &model),
                find_first_or_throw_field(ctx, &model),
                all_items_field(ctx, &model),
                plain_aggregation_field(ctx, &model),
            ];

            vec.push(group_by_aggregation_field(ctx, &model));
            append_opt(&mut vec, find_unique_field(ctx, &model));
            append_opt(&mut vec, find_unique_or_throw_field(ctx, &model));

            if ctx.enable_raw_queries && ctx.has_capability(ConnectorCapability::MongoDbQueryRaw) {
                vec.push(mongo_find_raw_field(&model));
                vec.push(mongo_aggregate_raw_field(&model));
            }

            vec
        })
        .collect();

    let ident = Identifier::new("Query".to_owned(), PRISMA_NAMESPACE);
    let strong_ref = Arc::new(object_type(ident, fields, None));

    (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
/// Find one unique semantics.
fn find_unique_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::where_unique_argument(ctx, model).map(|arg| {
        let field_name = format!("findUnique{}", model.name);

        field(
            field_name,
            vec![arg],
            OutputType::object(objects::model::map_type(ctx, model)),
            Some(QueryInfo {
                model: Some(Arc::clone(model)),
                tag: QueryTag::FindUnique,
            }),
        )
        .nullable()
    })
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model
/// that will throw a NotFoundError if the item is not found
fn find_unique_or_throw_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::where_unique_argument(ctx, model).map(|arg| {
        let field_name = format!("findUnique{}OrThrow", model.name);

        field(
            field_name,
            vec![arg],
            OutputType::object(objects::model::map_type(ctx, model)),
            Some(QueryInfo {
                model: Some(Arc::clone(model)),
                tag: QueryTag::FindUniqueOrThrow,
            }),
        )
        .nullable()
    })
}

/// Builds a find first item field for given model.
fn find_first_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::relation_selection_arguments(ctx, model, true);
    let field_name = format!("findFirst{}", model.name);

    field(
        field_name,
        args,
        OutputType::object(objects::model::map_type(ctx, model)),
        Some(QueryInfo {
            model: Some(Arc::clone(model)),
            tag: QueryTag::FindFirst,
        }),
    )
    .nullable()
}

/// Builds a find first item field for given model that throws a NotFoundError in case the item does
/// not exist
fn find_first_or_throw_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::relation_selection_arguments(ctx, model, true);
    let field_name = format!("findFirst{}OrThrow", model.name);

    field(
        field_name,
        args,
        OutputType::object(objects::model::map_type(ctx, model)),
        Some(QueryInfo {
            model: Some(Arc::clone(model)),
            tag: QueryTag::FindFirstOrThrow,
        }),
    )
    .nullable()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn all_items_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::relation_selection_arguments(ctx, model, true);
    let field_name = format!("findMany{}", model.name);
    let object_type = objects::model::map_type(ctx, model);

    field(
        field_name,
        args,
        OutputType::list(OutputType::object(object_type)),
        Some(QueryInfo {
            model: Some(Arc::clone(model)),
            tag: QueryTag::FindMany,
        }),
    )
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn plain_aggregation_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    field(
        format!("aggregate{}", model.name),
        arguments::relation_selection_arguments(ctx, model, false),
        OutputType::object(aggregation::plain::aggregation_object_type(ctx, model)),
        Some(QueryInfo {
            model: Some(Arc::clone(model)),
            tag: QueryTag::Aggregate,
        }),
    )
}

/// Builds a "group by" aggregation query field (e.g. "groupByUser") for given model.
fn group_by_aggregation_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    field(
        format!("groupBy{}", model.name),
        arguments::group_by_arguments(ctx, model),
        OutputType::list(OutputType::object(aggregation::group_by::group_by_output_object_type(
            ctx, model,
        ))),
        Some(QueryInfo {
            model: Some(Arc::clone(model)),
            tag: QueryTag::GroupBy,
        }),
    )
}

fn mongo_aggregate_raw_field(model: &ModelRef) -> OutputField {
    let field_name = format!("aggregate{}Raw", model.name);

    field(
        field_name,
        vec![
            input_field("pipeline", InputType::list(InputType::json()), None).optional(),
            input_field("options", InputType::json(), None).optional(),
        ],
        OutputType::json(),
        Some(QueryInfo {
            tag: QueryTag::QueryRaw {
                query_type: Some("aggregateRaw".to_owned()),
            },
            model: Some(model.clone()),
        }),
    )
}

fn mongo_find_raw_field(model: &ModelRef) -> OutputField {
    let field_name = format!("find{}Raw", model.name);

    field(
        field_name,
        vec![
            input_field("filter", InputType::json(), None).optional(),
            input_field("options", InputType::json(), None).optional(),
        ],
        OutputType::json(),
        Some(QueryInfo {
            tag: QueryTag::QueryRaw {
                query_type: Some("findRaw".to_owned()),
            },
            model: Some(model.clone()),
        }),
    )
}
