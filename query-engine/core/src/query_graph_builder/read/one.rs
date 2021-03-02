use super::*;
use crate::{constants::inputs::args, query_document::*, ReadQuery, RecordQuery};
use prisma_models::ModelRef;
use std::convert::TryInto;

/// Builds a read query from a parsed incoming read query field.
#[tracing::instrument(skip(field, model))]
pub fn find_unique(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let filter = match field.arguments.lookup(args::WHERE) {
        Some(where_arg) => {
            let arg: ParsedInputMap = where_arg.value.try_into()?;
            Some(extractors::extract_unique_filter(arg, &model)?)
        }
        None => None,
    };

    let name = field.name;
    let alias = field.alias;
    let model = model;
    let nested_fields = field.nested_fields.unwrap().fields;
    let (aggr_fields_pairs, nested_fields) = extractors::extract_nested_rel_aggr_selections(nested_fields);

    if !aggr_fields_pairs.is_empty() {
        return Err(QueryGraphBuilderError::InputError(
            "Aggregation selections are not yet implemented for findUnique queries.".to_owned(),
        ));
    }

    let selection_order: Vec<String> = utils::collect_selection_order(&nested_fields);
    let selected_fields = utils::collect_selected_fields(&nested_fields, &model);
    let nested = utils::collect_nested_queries(nested_fields, &model)?;
    let selected_fields = utils::merge_relation_selections(selected_fields, None, &nested);

    Ok(ReadQuery::RecordQuery(RecordQuery {
        name,
        alias,
        model,
        filter,
        selected_fields,
        nested,
        selection_order,
    }))
}
