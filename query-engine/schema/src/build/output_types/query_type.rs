use super::*;
use input_types::fields::arguments;

/// Builds the root `Query` type.
pub(crate) fn query_fields(ctx: &QuerySchema) -> Vec<FieldFn> {
    let mut fields: Vec<FieldFn> = Vec::with_capacity(ctx.internal_data_model.schema.db.models_count() * 6);

    macro_rules! field {
        ($f:ident, $model_var:expr) => {{
            let model = $model_var.clone();
            fields.push(Box::new(move |ctx| $f(ctx, model.clone())));
        }};
    }

    for model in ctx.internal_data_model.models() {
        field!(find_first_field, model);
        field!(find_first_or_throw_field, model);
        field!(all_items_field, model);
        field!(plain_aggregation_field, model);
        field!(group_by_aggregation_field, model);
        field!(find_unique_field, model);
        field!(find_unique_or_throw_field, model);

        if ctx.enable_raw_queries && ctx.has_capability(ConnectorCapability::MongoDbQueryRaw) {
            let model_cloned = model.clone();
            fields.push(Box::new(move |_ctx| mongo_find_raw_field(&model_cloned)));
            fields.push(Box::new(move |_ctx| mongo_aggregate_raw_field(&model)));
        }
    }

    fields
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
/// Find one unique semantics.
fn find_unique_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    arguments::where_unique_argument(ctx, model.clone())
        .map(|arg| {
            let field_name = format!("findUnique{}", model.name());

            field(
                field_name,
                Some(Arc::new(move || vec![arg.clone()])),
                OutputType::object(objects::model::model_object_type(ctx, model.clone())),
                Some(QueryInfo {
                    model: Some(model.id),
                    tag: QueryTag::FindUnique,
                }),
            )
            .nullable()
        })
        .unwrap()
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model
/// that will throw a NotFoundError if the item is not found
fn find_unique_or_throw_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    arguments::where_unique_argument(ctx, model.clone())
        .map(move |arg| {
            let field_name = format!("findUnique{}OrThrow", model.name());

            field(
                field_name,
                Some(Arc::new(move || vec![arg.clone()])),
                OutputType::object(objects::model::model_object_type(ctx, model.clone())),
                Some(QueryInfo {
                    model: Some(model.id),
                    tag: QueryTag::FindUniqueOrThrow,
                }),
            )
            .nullable()
        })
        .unwrap()
}

/// Builds a find first item field for given model.
fn find_first_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    let field_name = format!("findFirst{}", model.name());
    let cloned_model = model.clone();

    field(
        field_name,
        Some(Arc::new(move || {
            arguments::relation_to_many_selection_arguments(ctx, cloned_model.clone(), true)
        })),
        OutputType::object(objects::model::model_object_type(ctx, model.clone())),
        Some(QueryInfo {
            model: Some(model.id),
            tag: QueryTag::FindFirst,
        }),
    )
    .nullable()
}

/// Builds a find first item field for given model that throws a NotFoundError in case the item does
/// not exist
fn find_first_or_throw_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    let field_name = format!("findFirst{}OrThrow", model.name());
    let cloned_model = model.clone();

    field(
        field_name,
        Some(Arc::new(move || {
            arguments::relation_to_many_selection_arguments(ctx, cloned_model.clone(), true)
        })),
        OutputType::object(objects::model::model_object_type(ctx, model.clone())),
        Some(QueryInfo {
            model: Some(model.id),
            tag: QueryTag::FindFirstOrThrow,
        }),
    )
    .nullable()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn all_items_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    let field_name = format!("findMany{}", model.name());
    let object_type = objects::model::model_object_type(ctx, model.clone());
    let cloned_model = model.clone();

    field(
        field_name,
        Some(Arc::new(move || {
            arguments::relation_to_many_selection_arguments(ctx, cloned_model.clone(), true)
        })),
        OutputType::list(InnerOutputType::Object(object_type)),
        Some(QueryInfo {
            model: Some(model.id),
            tag: QueryTag::FindMany,
        }),
    )
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn plain_aggregation_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    let cloned_model = model.clone();
    field(
        format!("aggregate{}", model.name()),
        Some(Arc::new(move || {
            arguments::relation_to_many_selection_arguments(ctx, cloned_model.clone(), false)
        })),
        OutputType::object(aggregation::plain::aggregation_object_type(ctx, model.clone())),
        Some(QueryInfo {
            model: Some(model.id),
            tag: QueryTag::Aggregate,
        }),
    )
}

/// Builds a "group by" aggregation query field (e.g. "groupByUser") for given model.
fn group_by_aggregation_field(ctx: &QuerySchema, model: ModelRef) -> OutputField<'_> {
    let cloned_model = model.clone();
    field(
        format!("groupBy{}", model.name()),
        Some(Arc::new(move || {
            arguments::group_by_arguments(ctx, cloned_model.clone())
        })),
        OutputType::list(InnerOutputType::Object(
            aggregation::group_by::group_by_output_object_type(ctx, model.clone()),
        )),
        Some(QueryInfo {
            model: Some(model.id),
            tag: QueryTag::GroupBy,
        }),
    )
}

fn mongo_aggregate_raw_field<'a>(model: &ModelRef) -> OutputField<'a> {
    let field_name = format!("aggregate{}Raw", model.name());

    field(
        field_name,
        Some(Arc::new(|| {
            vec![
                input_field("pipeline", vec![InputType::list(InputType::json())], None).optional(),
                input_field("options", vec![InputType::json()], None).optional(),
            ]
        })),
        OutputType::non_list(OutputType::json()),
        Some(QueryInfo {
            tag: QueryTag::AggregateRaw,
            model: Some(model.id),
        }),
    )
}

fn mongo_find_raw_field<'a>(model: &ModelRef) -> OutputField<'a> {
    let field_name = format!("find{}Raw", model.name());

    field(
        field_name,
        Some(Arc::new(|| {
            vec![
                input_field("filter", vec![InputType::json()], None).optional(),
                input_field("options", vec![InputType::json()], None).optional(),
            ]
        })),
        OutputType::non_list(OutputType::json()),
        Some(QueryInfo {
            tag: QueryTag::FindRaw,
            model: Some(model.id),
        }),
    )
}
