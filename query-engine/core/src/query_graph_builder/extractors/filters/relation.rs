use super::extract_filter;
use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderResult};
use connector::{Filter, RelationCompare};
use prisma_models::RelationFieldRef;
use std::convert::TryInto;

pub fn parse(filter_key: &str, field: &RelationFieldRef, input: ParsedInputValue) -> QueryGraphBuilderResult<Filter> {
    let value: Option<ParsedInputMap> = input.try_into()?;

    Ok(match (filter_key, value) {
        // Relation list filters
        ("some", Some(value)) => field.at_least_one_related(extract_filter(value, &field.related_model())?),
        ("none", Some(value)) => field.no_related(extract_filter(value, &field.related_model())?),
        ("every", Some(value)) => field.every_related(extract_filter(value, &field.related_model())?),

        // One-relation filters
        ("is", Some(value)) => field.to_one_related(extract_filter(value, &field.related_model())?),
        ("is", None) => field.one_relation_is_null(),
        ("isNot", Some(value)) => field.no_related(extract_filter(value, &field.related_model())?),
        ("isNot", None) => Filter::not(vec![field.one_relation_is_null()]),

        _ => unreachable!(),
    })
}
