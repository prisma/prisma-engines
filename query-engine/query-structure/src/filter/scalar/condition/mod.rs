mod value;

pub use value::{ConditionListValue, ConditionValue};

use super::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarCondition {
    Equals(ConditionValue),
    NotEquals(ConditionValue),
    Contains(ConditionValue),
    NotContains(ConditionValue),
    StartsWith(ConditionValue),
    NotStartsWith(ConditionValue),
    EndsWith(ConditionValue),
    NotEndsWith(ConditionValue),
    LessThan(ConditionValue),
    LessThanOrEquals(ConditionValue),
    GreaterThan(ConditionValue),
    GreaterThanOrEquals(ConditionValue),
    In(ConditionListValue),
    NotIn(ConditionListValue),
    InTemplate(ConditionValue),
    NotInTemplate(ConditionValue),
    JsonCompare(JsonCondition),
    Search(ConditionValue, Vec<ScalarProjection>),
    NotSearch(ConditionValue, Vec<ScalarProjection>),
    IsSet(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonCondition {
    pub condition: Box<ScalarCondition>,
    pub path: Option<JsonFilterPath>,
    pub target_type: Option<JsonTargetType>,
    pub case: Case,
}

impl ScalarCondition {
    pub fn invert(self, condition: bool) -> Self {
        if condition {
            match self {
                Self::Equals(v) => Self::NotEquals(v),
                Self::NotEquals(v) => Self::Equals(v),
                Self::Contains(v) => Self::NotContains(v),
                Self::NotContains(v) => Self::Contains(v),
                Self::StartsWith(v) => Self::NotStartsWith(v),
                Self::NotStartsWith(v) => Self::StartsWith(v),
                Self::EndsWith(v) => Self::NotEndsWith(v),
                Self::NotEndsWith(v) => Self::EndsWith(v),
                Self::LessThan(v) => Self::GreaterThanOrEquals(v),
                Self::LessThanOrEquals(v) => Self::GreaterThan(v),
                Self::GreaterThan(v) => Self::LessThanOrEquals(v),
                Self::GreaterThanOrEquals(v) => Self::LessThan(v),
                Self::In(v) => Self::NotIn(v),
                Self::NotIn(v) => Self::In(v),
                Self::InTemplate(v) => Self::NotInTemplate(v),
                Self::NotInTemplate(v) => Self::InTemplate(v),
                Self::JsonCompare(json_compare) => {
                    let inverted_cond = json_compare.condition.invert(true);

                    Self::JsonCompare(JsonCondition {
                        condition: Box::new(inverted_cond),
                        path: json_compare.path,
                        target_type: json_compare.target_type,
                        case: json_compare.case,
                    })
                }
                Self::Search(v, fields) => Self::NotSearch(v, fields),
                Self::NotSearch(v, fields) => Self::Search(v, fields),
                Self::IsSet(v) => Self::IsSet(!v),
            }
        } else {
            self
        }
    }

    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        match self {
            ScalarCondition::Equals(v) => v.as_field_ref(),
            ScalarCondition::NotEquals(v) => v.as_field_ref(),
            ScalarCondition::Contains(v) => v.as_field_ref(),
            ScalarCondition::NotContains(v) => v.as_field_ref(),
            ScalarCondition::StartsWith(v) => v.as_field_ref(),
            ScalarCondition::NotStartsWith(v) => v.as_field_ref(),
            ScalarCondition::EndsWith(v) => v.as_field_ref(),
            ScalarCondition::NotEndsWith(v) => v.as_field_ref(),
            ScalarCondition::LessThan(v) => v.as_field_ref(),
            ScalarCondition::LessThanOrEquals(v) => v.as_field_ref(),
            ScalarCondition::GreaterThan(v) => v.as_field_ref(),
            ScalarCondition::GreaterThanOrEquals(v) => v.as_field_ref(),
            ScalarCondition::In(v) => v.as_field_ref(),
            ScalarCondition::NotIn(v) => v.as_field_ref(),
            ScalarCondition::InTemplate(v) => v.as_field_ref(),
            ScalarCondition::NotInTemplate(v) => v.as_field_ref(),
            ScalarCondition::JsonCompare(json_cond) => json_cond.condition.as_field_ref(),
            ScalarCondition::Search(v, _) => v.as_field_ref(),
            ScalarCondition::NotSearch(v, _) => v.as_field_ref(),
            ScalarCondition::IsSet(_) => None,
        }
    }
}
