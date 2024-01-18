use super::*;
use crate::{query_document::ParsedField, ReadQuery, RelatedRecordsQuery};
use query_structure::{Model, RelationFieldRef};
use schema::QuerySchema;

pub(crate) fn find_related(
    field: ParsedField<'_>,
    parent: RelationFieldRef,
    model: Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    let args = extractors::extract_query_args(field.arguments, &model)?;
    let name = field.name;
    let alias = field.alias;
    let sub_selections = field.nested_fields.unwrap().fields;
    // let (aggr_fields_pairs, sub_selections) = extractors::extract_nested_rel_aggr_selections(sub_selections);
    // let virtual_fields = utils::collect_virtual_fields(aggr_fields_pairs, &model)?;
    // let selection_order: Vec<String> = utils::collect_selection_order(&sub_selections);
    let (user_selection, full_selection) =
        utils::collect_selected_fields(&sub_selections, args.distinct.clone(), &model, query_schema)?;
    let nested = utils::collect_nested_queries(sub_selections, &model, query_schema)?;
    let parent_field = parent;

    let full_selection = utils::merge_relation_selections(full_selection, Some(parent_field.clone()), &nested);
    let full_selection = utils::merge_cursor_fields(full_selection, &args.cursor);
    // let selected_fields = selected_fields.merge(virtual_fields);

    Ok(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name,
        alias,
        parent_field,
        args,
        user_selection,
        full_selection,
        nested,
        // selection_order,
        // aggregation_selections,
        parent_results: None,
    }))
}
