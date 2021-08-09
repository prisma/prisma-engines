use crate::{filter::Filter, JsonFilterPath, JsonTargetType};
use prisma_models::PrismaValue;

/// Comparing methods for scalar fields.
pub trait ScalarCompare {
    fn is_in<T>(&self, val: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>;

    fn not_in<T>(&self, val: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>;

    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;

    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;
    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;
    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>;
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
        T: Into<PrismaValue>;

    fn contains_every_element<T>(&self, filter: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>;

    fn contains_some_element<T>(&self, filter: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>;

    fn is_empty_list(&self, b: bool) -> Filter;
}

/// Comparison methods for json fields
pub trait JsonCompare {
    fn json_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_not_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_less_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_less_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_greater_than<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_greater_than_or_equals<T>(&self, value: T, path: Option<JsonFilterPath>) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_not_contains<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_not_starts_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;

    fn json_not_ends_with<T>(&self, value: T, path: Option<JsonFilterPath>, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>;
}
