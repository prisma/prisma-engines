use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ReadQuery};
use prisma_models::ModelRef;

#[tracing::instrument(skip(field, model))]
pub fn aggregate(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = collect_selection_tree(&nested_fields);
    let args = extractors::extract_query_args(field.arguments, &model)?;

    // Reject any inmemory-requiring operation for aggregations, we don't have an in-memory aggregator yet.
    if args.requires_inmemory_processing() {
        return Err(QueryGraphBuilderError::InputError(
            "Unable to process combination of query arguments for aggregation query. \
             Please note that it is not possible at the moment to have a null-cursor, \
             or a cursor and orderBy combination that not stable (unique)."
                .to_owned(),
        ));
    }

    let selectors: Vec<_> = nested_fields
        .into_iter()
        .map(|field| resolve_query(field, &model))
        .collect::<QueryGraphBuilderResult<_>>()?;

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        selectors,
        group_by: vec![],
        having: None,
    }))
}
