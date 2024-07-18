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
        field!(find_many_field, model);
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
fn find_unique_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let model_id = model.id;
    let cloned_model = model.clone();

    field(
        format!("findUnique{}", model.name()),
        move || arguments::find_unique_arguments(ctx, cloned_model),
        OutputType::object(objects::model::model_object_type(ctx, model)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::FindUnique,
        }),
    )
    .nullable()
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model
/// that will throw a NotFoundError if the item is not found
fn find_unique_or_throw_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let model_id = model.id;
    let cloned_model = model.clone();
    field(
        format!("findUnique{}OrThrow", model.name()),
        move || arguments::find_unique_arguments(ctx, cloned_model),
        OutputType::object(objects::model::model_object_type(ctx, model)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::FindUniqueOrThrow,
        }),
    )
    .nullable()
}

/// Builds a find first item field for given model.
fn find_first_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let field_name = format!("findFirst{}", model.name());
    let cloned_model = model.clone();

    field(
        field_name,
        move || {
            arguments::ManyRecordsSelectionArgumentsBuilder::new(ctx, cloned_model)
                .include_distinct()
                .include_relation_load_strategy()
                .build()
        },
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
fn find_first_or_throw_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let field_name = format!("findFirst{}OrThrow", model.name());
    let model_id = model.id;
    let cloned_model = model.clone();

    field(
        field_name,
        move || {
            arguments::ManyRecordsSelectionArgumentsBuilder::new(ctx, model)
                .include_distinct()
                .include_relation_load_strategy()
                .build()
        },
        OutputType::object(objects::model::model_object_type(ctx, cloned_model)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::FindFirstOrThrow,
        }),
    )
    .nullable()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn find_many_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let field_name = format!("findMany{}", model.name());
    let object_type = objects::model::model_object_type(ctx, model.clone());
    let model_id = model.id;

    field(
        field_name,
        move || {
            arguments::ManyRecordsSelectionArgumentsBuilder::new(ctx, model)
                .include_distinct()
                .include_relation_load_strategy()
                .build()
        },
        OutputType::list(InnerOutputType::Object(object_type)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::FindMany,
        }),
    )
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn plain_aggregation_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let cloned_model = model.clone();
    let model_id = model.id;
    field(
        format!("aggregate{}", model.name()),
        move || arguments::ManyRecordsSelectionArgumentsBuilder::new(ctx, cloned_model).build(),
        OutputType::object(aggregation::plain::aggregation_object_type(ctx, model)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::Aggregate,
        }),
    )
}

/// Builds a "group by" aggregation query field (e.g. "groupByUser") for given model.
fn group_by_aggregation_field(ctx: &QuerySchema, model: Model) -> OutputField<'_> {
    let cloned_model = model.clone();
    let model_id = model.id;
    field(
        format!("groupBy{}", model.name()),
        move || arguments::group_by_arguments(ctx, cloned_model),
        OutputType::list(InnerOutputType::Object(
            aggregation::group_by::group_by_output_object_type(ctx, model),
        )),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::GroupBy,
        }),
    )
}

fn mongo_aggregate_raw_field<'a>(model: &Model) -> OutputField<'a> {
    let field_name = format!("aggregate{}Raw", model.name());

    field(
        field_name,
        || {
            vec![
                input_field("pipeline", vec![InputType::list(InputType::json())], None).optional(),
                input_field("options", vec![InputType::json()], None).optional(),
            ]
        },
        OutputType::non_list(OutputType::json()),
        Some(QueryInfo {
            tag: QueryTag::AggregateRaw,
            model: Some(model.id),
        }),
    )
}

fn mongo_find_raw_field<'a>(model: &Model) -> OutputField<'a> {
    let field_name = format!("find{}Raw", model.name());

    field(
        field_name,
        || {
            vec![
                input_field("filter", vec![InputType::json()], None).optional(),
                input_field("options", vec![InputType::json()], None).optional(),
            ]
        },
        OutputType::non_list(OutputType::json()),
        Some(QueryInfo {
            tag: QueryTag::FindRaw,
            model: Some(model.id),
        }),
    )
}
