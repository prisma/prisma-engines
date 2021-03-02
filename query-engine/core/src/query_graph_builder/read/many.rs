use super::*;
use crate::{query_document::ParsedField, ManyRecordsQuery, ReadQuery};
use prisma_models::ModelRef;

#[tracing::instrument(skip(field, model))]
pub fn find_many(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let args = extractors::extract_query_args(field.arguments, &model)?;
    let name = field.name;
    let alias = field.alias;
    let nested_fields = field.nested_fields.unwrap().fields;
    let (aggr_fields_pairs, nested_fields) = extractors::extract_nested_rel_aggr_selections(nested_fields);
    let aggr_selections: Vec<_> = utils::collect_relation_aggr_selections(&aggr_fields_pairs, &model);
    let selection_order: Vec<String> = utils::collect_selection_order(&nested_fields);
    let selected_fields = utils::collect_selected_fields(&nested_fields, &model);
    let nested = utils::collect_nested_queries(nested_fields, &model)?;
    let model = model;

    let selected_fields = utils::merge_relation_selections(selected_fields, None, &nested);
    let selected_fields = utils::merge_cursor_fields(selected_fields, &args.cursor);

    Ok(ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name,
        alias,
        model,
        args,
        selected_fields,
        nested,
        selection_order,
        aggregation_selections: aggr_selections,
    }))
}
