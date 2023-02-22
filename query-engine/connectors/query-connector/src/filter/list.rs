use super::*;
use crate::compare::ScalarListCompare;
use prisma_models::{ScalarField, ScalarFieldRef};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScalarListFilter {
    pub field: ScalarFieldRef,
    pub condition: ScalarListCondition,
}

impl ScalarListFilter {
    /// Returns the referenced field of the filter condition if there's one
    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        self.condition.as_field_ref()
    }
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

impl ScalarListCondition {
    /// Returns the referenced field of the filter condition if there's one
    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        match self {
            ScalarListCondition::Contains(v) => v.as_field_ref(),
            ScalarListCondition::ContainsEvery(v) => v.as_field_ref(),
            ScalarListCondition::ContainsSome(v) => v.as_field_ref(),
            ScalarListCondition::IsEmpty(_) => None,
        }
    }
}

#[allow(warnings)]
impl ScalarListCompare for ScalarField {
    fn contains_element<T>(&self, value: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarListFilter {
            field: self.clone(),
            condition: ScalarListCondition::Contains(value.into()),
        })
    }

    fn contains_every_element<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarListFilter {
            field: self.clone(),
            condition: ScalarListCondition::ContainsEvery(values.into()),
        })
    }

    fn contains_some_element<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarListFilter {
            field: self.clone(),
            condition: ScalarListCondition::ContainsSome(values.into()),
        })
    }

    fn is_empty_list(&self, b: bool) -> Filter {
        Filter::from(ScalarListFilter {
            field: self.clone(),
            condition: ScalarListCondition::IsEmpty(b),
        })
    }
}
