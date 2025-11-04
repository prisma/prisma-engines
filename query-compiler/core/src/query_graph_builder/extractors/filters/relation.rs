use super::extract_filter;
use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use query_structure::*;
use schema::constants::filters;
use std::convert::TryInto;

pub fn parse(
    filter_key: &str,
    field: &RelationFieldRef,
    input: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<Filter> {
    let value: Option<ParsedInputMap<'_>> = input.try_into()?;

    match (filter_key, value) {
        // Relation list filters
        (filters::SOME, Some(value)) => Ok(field.at_least_one_related(extract_filter(value, field.related_model())?)),
        (filters::NONE, Some(value)) => Ok(field.no_related(extract_filter(value, field.related_model())?)),
        (filters::EVERY, Some(value)) => Ok(field.every_related(extract_filter(value, field.related_model())?)),

        // One-relation filters
        (filters::IS, Some(value)) => Ok(field.to_one_related(extract_filter(value, field.related_model())?)),
        (filters::IS, None) => Ok(field.one_relation_is_null()),
        (filters::IS_NOT, Some(value)) => Ok(field.no_related(extract_filter(value, field.related_model())?)),
        (filters::IS_NOT, None) => Ok(Filter::not(vec![field.one_relation_is_null()])),

        _ => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid filter key `{filter_key}` input combination for relation filter"
        ))),
    }
}
