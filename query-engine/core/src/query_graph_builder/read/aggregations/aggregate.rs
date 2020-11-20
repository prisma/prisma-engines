use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ReadQuery};
use prisma_models::ModelRef;

pub fn aggregate(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = collect_selection_tree(&nested_fields);
    let args = extractors::extract_query_args(field.arguments, &model)?;

    // Reject unstable cursors for aggregations, because we can't do post-processing on those (we haven't implemented a in-memory aggregator yet).
    if args.contains_unstable_cursor() {
        return Err(QueryGraphBuilderError::InputError(
            "The chosen cursor and orderBy combination is not stable (unique) and can't be used for aggregations."
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
    }))
}
