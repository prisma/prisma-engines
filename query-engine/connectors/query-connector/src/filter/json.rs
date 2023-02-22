use super::scalar::*;
use crate::{Filter, JsonCompare, ScalarFilter};
use prisma_models::ScalarFieldRef;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum JsonTargetType {
    String,
    Array,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum JsonFilterPath {
    String(String),
    Array(Vec<String>),
}

impl JsonCompare for ScalarFieldRef {
    fn json_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::Contains(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_not_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::NotContains(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::StartsWith(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_not_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::NotStartsWith(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::EndsWith(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_not_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::NotEndsWith(value.into())),
                path,
                target_type: Some(target_type),
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::Equals(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_not_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::NotEquals(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_less_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::LessThan(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_less_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::LessThanOrEquals(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_greater_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::GreaterThan(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }

    fn json_greater_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            condition: ScalarCondition::JsonCompare(JsonCondition {
                condition: Box::new(ScalarCondition::GreaterThanOrEquals(value.into())),
                path,
                target_type: None,
            }),
            projection: ScalarProjection::Single(self.clone()),
            mode: QueryMode::Default,
        })
    }
}
