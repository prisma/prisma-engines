use super::Filter;
use crate::compare::ScalarListCompare;
use prisma_models::{PrismaValue, ScalarField};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScalarListFilter {
    pub field: Arc<ScalarField>,
    pub condition: ScalarListCondition,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarListCondition {
    /// List contains the given value
    Contains(PrismaValue),

    /// List contains all the given values
    ContainsEvery(Vec<PrismaValue>),

    /// List contains some of the given values
    ContainsSome(Vec<PrismaValue>),

    /// List emptiness check
    IsEmpty(bool),
}

#[allow(warnings)]
impl ScalarListCompare for Arc<ScalarField> {
    fn contains_element<T>(&self, value: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::Contains(value.into()),
        })
    }

    fn contains_every_element<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::ContainsEvery(values.into_iter().map(Into::into).collect()),
        })
    }

    fn contains_some_element<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::ContainsSome(values.into_iter().map(Into::into).collect()),
        })
    }

    fn is_empty_list(&self, b: bool) -> Filter {
        Filter::from(ScalarListFilter {
            field: Arc::clone(self),
            condition: ScalarListCondition::IsEmpty(b),
        })
    }
}
