use super::extract_filter;
use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{Filter, RelationCompare};
use prisma_models::RelationFieldRef;
use std::convert::TryInto;

pub fn parse(filter_key: &str, field: &RelationFieldRef, input: ParsedInputValue) -> QueryGraphBuilderResult<Filter> {
    let value: Option<ParsedInputMap> = input.try_into()?;

    match (filter_key, value) {
        // Relation list filters
        ("some", Some(value)) => Ok(field.at_least_one_related(extract_filter(value, &field.related_model())?)),
        ("none", Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),
        ("every", Some(value)) => Ok(field.every_related(extract_filter(value, &field.related_model())?)),

        // One-relation filters
        ("is", Some(value)) => Ok(field.to_one_related(extract_filter(value, &field.related_model())?)),
        ("is", None) => Ok(field.one_relation_is_null()),
        ("isNot", Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),
        ("isNot", None) => Ok(Filter::not(vec![field.one_relation_is_null()])),

        _ => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid filter key `{}` input combination for relation filter",
            filter_key
        ))),
    }
}
