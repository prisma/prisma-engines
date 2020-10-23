use super::*;

/// Builds the root `Query` type.
pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    let non_embedded_models = ctx.internal_data_model.non_embedded_models();
    let fields = non_embedded_models
        .into_iter()
        .map(|model| {
            let mut vec = vec![
                find_first_field(ctx, &model),
                all_items_field(ctx, &model),
                aggregation_field(ctx, &model),
            ];

            append_opt(&mut vec, find_one_field(ctx, &model));
            vec
        })
        .flatten()
        .collect();

    let strong_ref = Arc::new(object_type("Query", fields, None));

    (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
/// Find one unique semantics.
fn find_one_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::where_unique_argument(ctx, model).map(|arg| {
        let field_name = ctx.pluralize_internal(camel_case(&model.name), format!("findOne{}", model.name));

        field(
            field_name,
            vec![arg],
            OutputType::object(output_objects::map_model_object_type(ctx, &model)),
            Some(QueryInfo {
                model: Some(Arc::clone(&model)),
                tag: QueryTag::FindOne,
            }),
        )
        .optional()
    })
}

/// Builds a find first item field for given model.
fn find_first_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::many_records_arguments(ctx, &model);
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
    .optional()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn all_items_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::many_records_arguments(ctx, &model);
    let field_name = ctx.pluralize_internal(camel_case(pluralize(&model.name)), format!("findMany{}", model.name));

    field(
        field_name,
        args,
        OutputType::list(OutputType::object(output_objects::map_model_object_type(ctx, &model))),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::FindMany,
        }),
    )
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn aggregation_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::many_records_arguments(ctx, &model);
    let field_name = ctx.pluralize_internal(
        format!("aggregate{}", model.name), // Has no legacy counterpart.
        format!("aggregate{}", model.name),
    );

    field(
        field_name,
        args,
        OutputType::object(output_objects::aggregation_object_type(ctx, &model)),
        Some(QueryInfo {
            model: Some(Arc::clone(&model)),
            tag: QueryTag::Aggregate,
        }),
    )
}
