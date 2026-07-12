use super::extract_filter;
use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use query_structure::{CompositeCompare, CompositeFieldRef, Filter, PrismaValue};
use schema::{ObjectTag, constants::filters};
use std::convert::TryInto;

pub fn parse(
    input_map: ParsedInputMap<'_>,
    field: &CompositeFieldRef,
    _reverse: bool,
) -> QueryGraphBuilderResult<Filter> {
    let is_envelope = matches!(&input_map.tag, Some(ObjectTag::CompositeEnvelope));

    if is_envelope {
        // Unwrap is safe: We require exactly one field to be present on all filters.
        let (filter_key, value) = input_map.into_iter().next().unwrap();

        match (filter_key.as_ref(), value) {
            // Common composite filters.
            (filters::EQUALS, input) => Ok(field.equals(input.try_into()?)),
            (filters::IS_SET, input) => Ok(field.is_set(input.try_into()?)),

            // To-many composite filters.
            (filters::EVERY, input) => Ok(field.every(extract_filter(input.try_into()?, field.typ())?)),
            (filters::SOME, input) => Ok(field.some(extract_filter(input.try_into()?, field.typ())?)),
            (filters::NONE, input) => Ok(field.none(extract_filter(input.try_into()?, field.typ())?)),
            (filters::IS_EMPTY, input) => Ok(field.is_empty(input.try_into()?)),

            // To-one composite filters
            (filters::IS, ParsedInputValue::Single(PrismaValue::Null)) => Ok(field.equals(PrismaValue::Null)),
            (filters::IS, input) => Ok(field.is(extract_filter(input.try_into()?, field.typ())?)),

            (filters::IS_NOT, ParsedInputValue::Single(PrismaValue::Null)) => {
                Ok(Filter::not(vec![field.equals(PrismaValue::Null)]))
            }
            (filters::IS_NOT, input) => Ok(field.is_not(extract_filter(input.try_into()?, field.typ())?)),

            _ => Err(QueryGraphBuilderError::InputError(format!(
                "Invalid filter key `{filter_key}` input combination for composite filter"
            ))),
        }
    } else {
        // Equality shorthand
        Ok(field.equals(ParsedInputValue::Map(input_map).try_into()?))
    }
}
