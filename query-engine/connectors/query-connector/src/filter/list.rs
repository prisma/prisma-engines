use super::Filter;
use crate::{compare::ScalarListCompare, ConditionListValue, ConditionValue};
use prisma_models::ScalarField;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScalarListFilter {
    pub field: Arc<ScalarField>,
    pub condition: ScalarListCondition,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarListCondition {
    /// List contains the given value
    Contains(ConditionValue),

    /// List contains all the given values
    ContainsEvery(ConditionListValue),

    /// List contains some of the given values
    ContainsSome(ConditionListValue),

    /// List emptiness check
    IsEmpty(bool),
}

#[allow(warnings)]
impl ScalarListCompare for Arc<ScalarField> {
    fn contains_element<T>(&self, value: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::Contains(value.into()),
        })
    }

    fn contains_every_element<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::ContainsEvery(values.into()),
        })
    }

    fn contains_some_element<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::ContainsSome(values.into()),
        })
    }

    fn is_empty_list(&self, b: bool) -> Filter {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::IsEmpty(b),
        })
    }
}
