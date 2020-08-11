use super::extract_filter;
use crate::{ParsedInputMap, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{Filter, RelationCompare};
use prisma_models::RelationFieldRef;
use std::str::FromStr;

pub enum RelationFieldFilter {
    Some,
    None,
    Every,
    Is,
    IsNot,
}

impl RelationFieldFilter {
    pub fn into_filter(
        self,
        field: RelationFieldRef,
        value: Option<ParsedInputMap>,
    ) -> QueryGraphBuilderResult<Filter> {
        Ok(match (self, value) {
            // Relation list filters
            (Self::Some, Some(value)) => field.at_least_one_related(extract_filter(value, &field.related_model())?),
            (Self::None, Some(value)) => field.no_related(extract_filter(value, &field.related_model())?),
            (Self::Every, Some(value)) => field.every_related(extract_filter(value, &field.related_model())?),

            // One-relation filters
            (Self::Is, Some(value)) => field.to_one_related(extract_filter(value, &field.related_model())?),
            (Self::Is, None) => field.one_relation_is_null(),
            (Self::IsNot, Some(value)) => field.no_related(extract_filter(value, &field.related_model())?),
            (Self::IsNot, None) => Filter::not(vec![field.one_relation_is_null()]),

            _ => unreachable!(),
        })
    }
}

impl FromStr for RelationFieldFilter {
    type Err = QueryGraphBuilderError;

    fn from_str(s: &str) -> QueryGraphBuilderResult<Self> {
        match s.to_lowercase().as_str() {
            "some" => Ok(Self::Some),
            "none" => Ok(Self::None),
            "every" => Ok(Self::Every),
            "is" => Ok(Self::Is),
            "is_not" => Ok(Self::IsNot),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                s
            ))),
        }
    }
}
