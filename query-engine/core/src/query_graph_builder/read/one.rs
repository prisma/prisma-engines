use super::*;
use crate::{query_document::*, ReadQuery, RecordQuery};
use prisma_models::ModelRef;
use std::convert::TryInto;

/// Builds a read query from a parsed incoming read query field.
pub fn find_one(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let filter = match field.arguments.lookup("where") {
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
