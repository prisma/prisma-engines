use super::*;

use crate::filter::Filter;
use prisma_value::PrismaValue;

/// Comparing methods for scalar fields.
pub trait ScalarCompare {
    fn is_in<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionListValue>;

    fn not_in<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionListValue>;

    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn geometry_within<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn geometry_not_within<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn geometry_intersects<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn geometry_not_intersects<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn is_set(&self, val: bool) -> Filter;
}

/// Comparison methods for relational fields.
pub trait RelationCompare {
    fn every_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn at_least_one_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn to_one_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn no_related<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn one_relation_is_null(&self) -> Filter;
}

/// Comparison methods for scalar list fields.
pub trait ScalarListCompare {
    fn contains_element<T>(&self, value: T) -> Filter
    where
        T: Into<ConditionValue>;

    fn contains_every_element<T>(&self, filter: T) -> Filter
    where
        T: Into<ConditionListValue>;

    fn contains_some_element<T>(&self, filter: T) -> Filter
    where
        T: Into<ConditionListValue>;

    fn is_empty_list(&self, b: bool) -> Filter;
}

/// Comparison methods for json fields
pub trait JsonCompare {
    fn json_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_not_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_less_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_less_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_greater_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_greater_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_not_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_not_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;

    fn json_not_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<ConditionValue>;
}

/// Comparison methods for composite fields.
pub trait CompositeCompare {
    fn every<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn some<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn none<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn is<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn is_not<T>(&self, filter: T) -> Filter
    where
        T: Into<Filter>;

    fn is_empty(&self, b: bool) -> Filter;

    fn is_set(&self, b: bool) -> Filter;

    fn equals(&self, val: PrismaValue) -> Filter;
}
