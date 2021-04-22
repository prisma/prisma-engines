use crate::{Filter, JsonCompare, ScalarCompare, ScalarFilter};
use prisma_models::{PrismaValue, ScalarFieldRef};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct JsonFilter {
    pub filter: ScalarFilter,
    pub target_type: Option<JsonTargetType>,
    pub path: Option<JsonFilterPath>,
}

impl JsonFilter {
    pub fn set_path(&mut self, path: Option<JsonFilterPath>) {
        self.path = path
    }
}

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
    fn json_contains<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.contains(value)),
            target_type: Some(target_type),
            path: None,
        })
    }

    fn json_not_contains<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.not_contains(value)),
            target_type: Some(target_type),
            path: None,
        })
    }

    fn json_starts_with<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.starts_with(value)),
            target_type: Some(target_type),
            path: None,
        })
    }

    fn json_not_starts_with<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.not_starts_with(value)),
            target_type: Some(target_type),
            path: None,
        })
    }

    fn json_ends_with<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.ends_with(value)),
            target_type: Some(target_type),
            path: None,
        })
    }

    fn json_not_ends_with<T>(&self, value: T, target_type: JsonTargetType) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(JsonFilter {
            filter: convert_filter_to_scalar_filter(self.not_ends_with(value)),
            target_type: Some(target_type),
            path: None,
        })
    }
}

fn convert_filter_to_scalar_filter(filter: Filter) -> ScalarFilter {
    match filter {
        Filter::Scalar(scalar_filter) => scalar_filter,
        x => panic!("A scalar filter was expected, found: {:?}", x),
    }
}
