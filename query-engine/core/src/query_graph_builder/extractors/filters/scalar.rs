use crate::{QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{Filter, ScalarCompare};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::str::FromStr;

pub enum ScalarFieldFilter {
    Equals,
    NotEquals,
    In,
    NotIn,
    Not,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
    NotContains,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
}

impl ScalarFieldFilter {
    pub fn into_filter(self, field: &ScalarFieldRef, value: PrismaValue) -> QueryGraphBuilderResult<Filter> {
        Ok(match (self, value) {
            (Self::In, PrismaValue::Null(hint)) => field.equals(PrismaValue::Null(hint)),
            (Self::In, PrismaValue::List(values)) => field.is_in(values),
            (Self::NotIn, PrismaValue::Null(hint)) => field.not_equals(PrismaValue::Null(hint)),
            (Self::NotIn, PrismaValue::List(values)) => field.not_in(values),
            (Self::Not, val) => field.not_equals(val),
            (Self::Lt, val) => field.less_than(val),
            (Self::Lte, val) => field.less_than_or_equals(val),
            (Self::Gt, val) => field.greater_than(val),
            (Self::Gte, val) => field.greater_than_or_equals(val),
            (Self::Contains, val) => field.contains(val),
            (Self::NotContains, val) => field.not_contains(val),
            (Self::StartsWith, val) => field.starts_with(val),
            (Self::NotStartsWith, val) => field.not_starts_with(val),
            (Self::EndsWith, val) => field.ends_with(val),
            (Self::NotEndsWith, val) => field.not_ends_with(val),
            (Self::Equals, val) => field.equals(val),
            (Self::NotEquals, val) => field.not_equals(val),
            (_, _) => unreachable!(),
        })
    }
}

impl FromStr for ScalarFieldFilter {
    type Err = QueryGraphBuilderError;

    fn from_str(s: &str) -> QueryGraphBuilderResult<Self> {
        match s.to_lowercase().as_str() {
            "in" => Ok(Self::In),
            "not_in" => Ok(Self::NotIn),
            "not" => Ok(Self::Not),
            "lt" => Ok(Self::Lt),
            "lte" => Ok(Self::Lte),
            "gt" => Ok(Self::Gt),
            "gte" => Ok(Self::Gte),
            "contains" => Ok(Self::Contains),
            "not_contains" => Ok(Self::NotContains),
            "starts_with" => Ok(Self::StartsWith),
            "not_starts_with" => Ok(Self::NotStartsWith),
            "ends_with" => Ok(Self::EndsWith),
            "not_ends_with" => Ok(Self::NotEndsWith),
            "equals" => Ok(Self::Equals),
            "not_equals" => Ok(Self::NotEquals),
            _ => Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                s
            ))),
        }
    }
}
