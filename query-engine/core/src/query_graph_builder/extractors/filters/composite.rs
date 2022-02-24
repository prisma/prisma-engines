use super::extract_filter;
use crate::{
    constants::filters, ObjectTag, ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{CompositeCompare, Filter};
use prisma_models::{CompositeFieldRef, RelationFieldRef};
use std::convert::TryInto;

pub fn parse(
    mut input_map: ParsedInputMap,
    field: &CompositeFieldRef,
    reverse: bool,
) -> QueryGraphBuilderResult<Filter> {
    let is_envelope = matches!(&input_map.tag, Some(ObjectTag::CompositeEnvelope));

    if is_envelope {
        // Unwrap is safe: We require exactly one field to be present
        let (filter_key, value) = input_map.into_iter().next().unwrap();

        match (filter_key.as_ref(), value) {
            // To-many composite filters.
            (filters::EVERY, input) => Ok(field.every(extract_filter(input.try_into()?, &field.typ)?)),

            // (filters::EVERY, Some(value)) => Ok(field.every_related(extract_filter(value, &field.related_model())?)),
            // (filters::SOME, Some(value)) => {
            //     Ok(field.at_least_one_related(extract_filter(value, &field.related_model())?))
            // }
            // (filters::NONE, Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),

            // // To-one composite filters
            // (filters::IS, Some(value)) => Ok(field.to_one_related(extract_filter(value, &field.related_model())?)),
            // (filters::IS, None) => Ok(field.one_relation_is_null()),
            // (filters::IS_NOT, Some(value)) => Ok(field.no_related(extract_filter(value, &field.related_model())?)),
            // (filters::IS_NOT, None) => Ok(Filter::not(vec![field.one_relation_is_null()])),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "Invalid filter key `{}` input combination for composite filter",
                filter_key
            ))),
        }
    } else {
        todo!()
    }
}
