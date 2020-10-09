use crate::compare::RelationCompare;
use crate::filter::Filter;
use prisma_models::RelationField;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationFilter {
    pub field: Arc<RelationField>,
    pub nested_filter: Box<Filter>,
    pub condition: RelationCondition,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OneRelationIsNullFilter {
    pub field: Arc<RelationField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationCondition {
    EveryRelatedRecord,
    AtLeastOneRelatedRecord,
    NoRelatedRecord,
    ToOneRelatedRecord,
}

impl RelationCondition {
    pub fn invert_of_subselect(self) -> bool {
        matches!(self, RelationCondition::EveryRelatedRecord)
    }
}

impl RelationCompare for Arc<RelationField> {
    /// Every related record matches the filter.
    fn every_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        Filter::from(RelationFilter {
            field: Arc::clone(self),
            nested_filter: Box::new(filter.into()),
            condition: RelationCondition::EveryRelatedRecord,
        })
    }

    /// At least one related record matches the filter.
    fn at_least_one_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        Filter::from(RelationFilter {
            field: Arc::clone(self),
            nested_filter: Box::new(filter.into()),
            condition: RelationCondition::AtLeastOneRelatedRecord,
        })
    }

    /// To one related record. FIXME
    fn to_one_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        Filter::from(RelationFilter {
            field: Arc::clone(self),
            nested_filter: Box::new(filter.into()),
            condition: RelationCondition::ToOneRelatedRecord,
        })
    }

    /// None of the related records matches the filter.
    fn no_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        Filter::from(RelationFilter {
            field: Arc::clone(self),
            nested_filter: Box::new(filter.into()),
            condition: RelationCondition::NoRelatedRecord,
        })
    }

    /// One of the relations is `Null`.
    fn one_relation_is_null(&self) -> Filter {
        Filter::from(OneRelationIsNullFilter {
            field: Arc::clone(self),
        })
    }
}
