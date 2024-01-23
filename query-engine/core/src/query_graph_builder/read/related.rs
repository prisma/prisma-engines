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
    let selection_order: Vec<String> = utils::collect_selection_order(&sub_selections);
    let selected_fields = utils::collect_selected_fields(&sub_selections, args.distinct.clone(), &model, query_schema)?;
    let nested = utils::collect_nested_queries(sub_selections, &model, query_schema)?;
    let parent_field = parent;

    let selected_fields = utils::merge_relation_selections(selected_fields, Some(parent_field.clone()), &nested);
    let selected_fields = utils::merge_cursor_fields(selected_fields, &args.cursor);

    Ok(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name,
        alias,
        parent_field,
        args,
        selected_fields,
        nested,
        selection_order,
        parent_results: None,
    }))
}
