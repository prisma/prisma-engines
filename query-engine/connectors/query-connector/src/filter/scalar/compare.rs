use super::*;
use crate::*;
use prisma_models::*;

impl ScalarCompare for ScalarFieldRef {
    /// Field is in a given value
    fn is_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::In(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotIn(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::Equals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not equal the given value.
    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field contains the given value.
    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::Contains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not contain the given value.
    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotContains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field starts with the given value.
    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::StartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not start with the given value.
    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotStartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field ends with the given value.
    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::EndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not end with the given value.
    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotEndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than the given value.
    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::LessThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than or equals the given value.
    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::LessThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than the given value.
    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::GreaterThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than or equals the given value.
    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::NotSearch(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn is_set(&self, val: bool) -> Filter {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(self.clone()),
            condition: ScalarCondition::IsSet(val),
            mode: QueryMode::Default,
        })
    }
}

impl ScalarCompare for ModelProjection {
    /// Field is in a given value
    fn is_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::In(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotIn(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::Equals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not equal the given value.
    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field contains the given value.
    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::Contains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not contain the given value.
    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotContains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field starts with the given value.
    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::StartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not start with the given value.
    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotStartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field ends with the given value.
    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::EndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not end with the given value.
    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotEndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than the given value.
    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::LessThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than or equals the given value.
    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::LessThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than the given value.
    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::GreaterThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than or equals the given value.
    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotSearch(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn is_set(&self, val: bool) -> Filter {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::IsSet(val),
            mode: QueryMode::Default,
        })
    }
}

impl ScalarCompare for FieldSelection {
    /// Field is in a given value
    fn is_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::In(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: T) -> Filter
    where
        T: Into<ConditionListValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotIn(values.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::Equals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not equal the given value.
    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field contains the given value.
    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::Contains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not contain the given value.
    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotContains(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field starts with the given value.
    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::StartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not start with the given value.
    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotStartsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field ends with the given value.
    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::EndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field does not end with the given value.
    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotEndsWith(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than the given value.
    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::LessThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is less than or equals the given value.
    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::LessThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than the given value.
    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::GreaterThan(val.into()),
            mode: QueryMode::Default,
        })
    }

    /// Field is greater than or equals the given value.
    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::NotSearch(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn is_set(&self, val: bool) -> Filter {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.as_scalar_fields().expect("Todo composites in filters.")),
            condition: ScalarCondition::IsSet(val),
            mode: QueryMode::Default,
        })
    }
}
