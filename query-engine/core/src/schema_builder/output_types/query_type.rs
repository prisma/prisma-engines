use super::*;

/// Builds the root `Query` type.
#[tracing::instrument(name = "build_query_type", skip(ctx))]
pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    let non_embedded_models = ctx.internal_data_model.non_embedded_models();
    let fields = non_embedded_models
        .into_iter()
        .map(|model| {
            let mut vec = vec![
                find_first_field(ctx, &model),
                all_items_field(ctx, &model),
                plain_aggregation_field(ctx, &model),
            ];

            if feature_flags::get().groupBy {
                vec.push(group_by_aggregation_field(ctx, &model));
            }

            append_opt(&mut vec, find_unique_field(ctx, &model));
            vec
        })
        .flatten()
        .collect();

    let ident = Identifier::new("Query".to_owned(), PRISMA_NAMESPACE);
    let strong_ref = Arc::new(object_type(ident, fields, None));

    (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
/// Find one unique semantics.
fn find_unique_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::where_unique_argument(ctx, model).map(|arg| {
        let field_name = ctx.pluralize_internal(camel_case(&model.name), format!("findUnique{}", model.name));

        field(
            field_name,
            vec![arg],
            OutputType::object(output_objects::map_model_object_type(ctx, &model)),
            Some(QueryInfo {
                model: Some(Arc::clone(&model)),
                tag: QueryTag::FindUnique,
            }),
        )
        .nullable()
    })
}

/// Builds a find first item field for given model.
fn find_first_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::many_records_arguments(ctx, &model, true);
    let field_name = format!("findFirst{}", model.name);

    field(
        field_name,
        args,
        OutputType::object(output_objects::map_model_object_type(ctx, &model)),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::FindFirst,
        }),
    )
    .nullable()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn all_items_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::many_records_arguments(ctx, &model, true);
    let field_name = ctx.pluralize_internal(camel_case(pluralize(&model.name)), format!("findMany{}", model.name));
    let object_type = output_objects::map_model_object_type(ctx, &model);

    field(
        field_name,
        args,
        OutputType::list(OutputType::object(object_type)),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::FindMany,
        }),
    )
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn plain_aggregation_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    field(
        format!("aggregate{}", model.name),
        arguments::many_records_arguments(ctx, &model, false),
        OutputType::object(aggregation::plain::aggregation_object_type(ctx, &model)),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::Aggregate,
        }),
    )
}

/// Builds a "group by" aggregation query field (e.g. "groupByUser") for given model.
fn group_by_aggregation_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    field(
        format!("groupBy{}", model.name),
        arguments::group_by_arguments(ctx, &model),
        OutputType::list(OutputType::object(aggregation::group_by::group_by_output_object_type(
            ctx, &model,
        ))),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::GroupBy,
        }),
    )
}
