use crate::{CompositeCompare, CompositeFieldRef, filter::Filter};
use prisma_value::PrismaValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompositeFilter {
    /// Starting field of the Composite traversal.
    pub field: CompositeFieldRef,

    // /// Filter the composite need to fulfill.
    // pub nested_filter: Box<Filter>,
    /// Condition the composite field filter uses.
    pub condition: Box<CompositeCondition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompositeCondition {
    /// Every composite in the list needs to fulfill a condition.
    Every(Filter),

    /// One or more composite in a list needs to fulfill a condition.
    Some(Filter),

    /// No composite in a list must fulfill a condition.
    None(Filter),

    /// Checks whether or not the composite list is empty.
    Empty(bool),

    /// Entire composite equals the given value. Can be for lists or single composites.
    Equals(PrismaValue),

    /// To-one composite only - the composite must fulfill the filter.
    Is(Filter),

    /// To-one composite only - the composite must not fulfill the filter.
    IsNot(Filter),

    /// Checks whether or not the composite field exists (is `undefined` or not)
    IsSet(bool),
}

impl CompositeCompare for CompositeFieldRef {
    fn every<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::Every(filter.into())),
        }
        .into()
    }

    fn some<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::Some(filter.into())),
        }
        .into()
    }

    fn none<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::None(filter.into())),
        }
        .into()
    }

    fn is<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::Is(filter.into())),
        }
        .into()
    }

    fn is_not<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::IsNot(filter.into())),
        }
        .into()
    }

    fn is_empty(&self, b: bool) -> Filter {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::Empty(b)),
        }
        .into()
    }

    fn is_set(&self, b: bool) -> Filter {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::IsSet(b)),
        }
        .into()
    }

    fn equals(&self, val: PrismaValue) -> Filter {
        CompositeFilter {
            field: self.clone(),
            condition: Box::new(CompositeCondition::Equals(val)),
        }
        .into()
    }
}
