use super::*;
use crate::{query_document::*, QueryOptions, ReadQuery, RecordQuery, THROW_ON_EMPTY};
use prisma_models::ModelRef;
use schema_builder::constants::args;
use std::convert::TryInto;

pub fn find_unique(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    find_unique_with_options(field, model, QueryOptions::none())
}

pub fn find_unique_or_throw(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    find_unique_with_options(field, model, THROW_ON_EMPTY.to_vec().into())
}

/// Builds a read query from a parsed incoming read query field.
#[inline]
pub fn find_unique_with_options(
    mut field: ParsedField,
    model: ModelRef,
    options: QueryOptions,
) -> QueryGraphBuilderResult<ReadQuery> {
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
    let aggregation_selections = utils::collect_relation_aggr_selections(aggr_fields_pairs, &model)?;
    let selection_order: Vec<String> = utils::collect_selection_order(&nested_fields);
    let selected_fields = utils::collect_selected_fields(&nested_fields, None, &model);
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
        aggregation_selections,
        options,
    }))
}
