use super::extract_filter;
use crate::{
    constants::inputs::filters, ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{Filter, RelationCompare};
use prisma_models::RelationFieldRef;
use std::convert::TryInto;

#[tracing::instrument(name = "parse_relation_field", skip(filter_key, field, input))]
pub fn parse(filter_key: &str, field: &RelationFieldRef, input: ParsedInputValue) -> QueryGraphBuilderResult<Filter> {
    let value: Option<ParsedInputMap> = input.try_into()?;

    match (filter_key, value) {
        // Relation list filters
        (filters::SOME, Some(value)) => Ok(field.at_least_one_related(extract_filter(value, &field.related_model())?)),
        (filters::NONE, Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),
        (filters::EVERY, Some(value)) => Ok(field.every_related(extract_filter(value, &field.related_model())?)),

        // One-relation filters
        (filters::IS, Some(value)) => Ok(field.to_one_related(extract_filter(value, &field.related_model())?)),
        (filters::IS, None) => Ok(field.one_relation_is_null()),
        (filters::IS_NOT, Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),
        (filters::IS_NOT, None) => Ok(Filter::not(vec![field.one_relation_is_null()])),

        _ => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid filter key `{}` input combination for relation filter",
            filter_key
        ))),
    }
}
