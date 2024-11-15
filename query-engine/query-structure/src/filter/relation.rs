use crate::{filter::Filter, RelationCompare, RelationField};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RelationFilter {
    /// Starting field of the relation traversal.
    pub field: RelationField,

    /// Filter the related records need to fulfill.
    pub nested_filter: Box<Filter>,

    /// The type of relation condition to use.
    /// E.g. if all related records or only some need
    /// to fulfill `nested_filter`.
    pub condition: RelationCondition,
}

impl std::fmt::Debug for RelationFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelationFilter")
            .field("field", &format!("{}", self.field))
            .field("nested_filter", &self.nested_filter)
            .field("condition", &self.condition)
            .finish()
    }
}

impl RelationFilter {
    pub fn invert(self, invert: bool) -> Self {
        if invert {
            let is_to_one = !self.field.is_list();

            Self {
                field: self.field,
                nested_filter: self.nested_filter,
                condition: self.condition.invert(invert, is_to_one),
            }
        } else {
            self
        }
    }
}

/// Filter that is solely responsible for checking if
/// a to-one related record is null.
/// Todo there's no good, obvious reason why this is a separate filter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OneRelationIsNullFilter {
    pub field: RelationField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationCondition {
    /// Every single related record needs to fulfill a condition.
    /// `every` query condition.
    EveryRelatedRecord,

    /// At least one related record needs to fulfill a condition.
    /// `some` query condition.
    AtLeastOneRelatedRecord,

    /// No related record must to fulfill a condition.
    /// `none` query condition.
    NoRelatedRecord,

    /// To-one relation only - the related record must fulfill a condition.
    ToOneRelatedRecord,
}

impl RelationCondition {
    pub fn invert_of_subselect(self) -> bool {
        matches!(self, RelationCondition::EveryRelatedRecord)
    }

    pub fn invert(self, invert: bool, to_one: bool) -> Self {
        if invert {
            match self {
                RelationCondition::EveryRelatedRecord => RelationCondition::NoRelatedRecord,
                RelationCondition::AtLeastOneRelatedRecord => RelationCondition::NoRelatedRecord,
                RelationCondition::NoRelatedRecord if to_one => RelationCondition::ToOneRelatedRecord,
                RelationCondition::NoRelatedRecord => RelationCondition::AtLeastOneRelatedRecord,
                RelationCondition::ToOneRelatedRecord => RelationCondition::NoRelatedRecord,
            }
        } else {
            self
        }
    }
}

impl RelationCompare for RelationField {
    /// Every related record matches the filter.
    fn every_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>,
    {
        Filter::from(RelationFilter {
            field: self.clone(),
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
            field: self.clone(),
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
            field: self.clone(),
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
            field: self.clone(),
            nested_filter: Box::new(filter.into()),
            condition: RelationCondition::NoRelatedRecord,
        })
    }

    /// One of the relations is `Null`.
    fn one_relation_is_null(&self) -> Filter {
        Filter::from(OneRelationIsNullFilter { field: self.clone() })
    }
}
