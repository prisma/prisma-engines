use super::Filter;
use crate::compare::ScalarCompare;
use once_cell::sync::Lazy;
use prisma_models::{DataSourceFieldRef, ModelProjection, PrismaListValue, PrismaValue};
use std::{collections::BTreeSet, env, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarProjection {
    Single(DataSourceFieldRef),
    Compound(Vec<DataSourceFieldRef>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Filtering with a scalar value. From a GraphQL point of view this is in the
/// head of the query:
///
/// ```graphql
/// findManyUser(where: { id: 5 })
/// ````
///
/// This translates to a projection of one column `id` with a condition where
/// the column value equals `5`.
pub struct ScalarFilter {
    pub projection: ScalarProjection,
    pub condition: ScalarCondition,
}

/// Number of allowed elements in query's `IN` or `NOT IN` statement.
/// Certain databases error out if querying with too many items. For test
/// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
/// value to a smaller number.
static BATCH_SIZE: Lazy<usize> = Lazy::new(|| match env::var("QUERY_BATCH_SIZE") {
    Ok(size) => size.parse().unwrap_or(5000),
    Err(_) => 5000,
});

impl ScalarFilter {
    /// The number of values in the filter. `IN` and `NOT IN` may contain more
    /// than one.
    pub fn len(&self) -> usize {
        match self.condition {
            ScalarCondition::In(ref l) => l.len(),
            ScalarCondition::NotIn(ref l) => l.len(),
            _ => 1,
        }
    }

    /// If `true`, the filter can be split into smaller filters executed in
    /// separate queries.
    pub fn can_batch(&self) -> bool {
        self.len() > *BATCH_SIZE
    }

    /// If possible, converts the filter into multiple smaller filters.
    pub fn batched(self) -> Vec<ScalarFilter> {
        fn inner(mut list: PrismaListValue) -> Vec<PrismaListValue> {
            let dedup_list: BTreeSet<_> = list.drain(..).collect();

            let mut batches = Vec::with_capacity(list.len() % *BATCH_SIZE + 1);
            batches.push(Vec::with_capacity(*BATCH_SIZE));

            for (idx, item) in dedup_list.into_iter().enumerate() {
                if idx != 0 && idx % *BATCH_SIZE == 0 {
                    batches.push(Vec::with_capacity(*BATCH_SIZE));
                }

                batches.last_mut().unwrap().push(item);
            }

            batches
        }

        match self.condition {
            ScalarCondition::In(list) => {
                let projection = self.projection;

                inner(list)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::In(batch),
                    })
                    .collect()
            }
            ScalarCondition::NotIn(list) => {
                let projection = self.projection;

                inner(list)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::NotIn(batch),
                    })
                    .collect()
            }
            _ => vec![self],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarCondition {
    Equals(PrismaValue),
    NotEquals(PrismaValue),
    Contains(PrismaValue),
    NotContains(PrismaValue),
    StartsWith(PrismaValue),
    NotStartsWith(PrismaValue),
    EndsWith(PrismaValue),
    NotEndsWith(PrismaValue),
    LessThan(PrismaValue),
    LessThanOrEquals(PrismaValue),
    GreaterThan(PrismaValue),
    GreaterThanOrEquals(PrismaValue),
    In(PrismaListValue),
    NotIn(PrismaListValue),
}

impl ScalarCompare for DataSourceFieldRef {
    /// Field is in a given value
    fn is_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::In(values.into_iter().map(|i| i.into()).collect()),
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::NotIn(values.into_iter().map(|i| i.into()).collect()),
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::Equals(val.into()),
        })
    }

    /// Field does not equal the given value.
    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::NotEquals(val.into()),
        })
    }

    /// Field contains the given value.
    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::Contains(val.into()),
        })
    }

    /// Field does not contain the given value.
    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::NotContains(val.into()),
        })
    }

    /// Field starts with the given value.
    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::StartsWith(val.into()),
        })
    }

    /// Field does not start with the given value.
    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::NotStartsWith(val.into()),
        })
    }

    /// Field ends with the given value.
    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::EndsWith(val.into()),
        })
    }

    /// Field does not end with the given value.
    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::NotEndsWith(val.into()),
        })
    }

    /// Field is less than the given value.
    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::LessThan(val.into()),
        })
    }

    /// Field is less than or equals the given value.
    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::LessThanOrEquals(val.into()),
        })
    }

    /// Field is greater than the given value.
    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::GreaterThan(val.into()),
        })
    }

    /// Field is greater than or equals the given value.
    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
        })
    }
}

impl ScalarCompare for ModelProjection {
    /// Field is in a given value
    fn is_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::In(values.into_iter().map(|i| i.into()).collect()),
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::NotIn(values.into_iter().map(|i| i.into()).collect()),
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::Equals(val.into()),
        })
    }

    /// Field does not equal the given value.
    fn not_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::NotEquals(val.into()),
        })
    }

    /// Field contains the given value.
    fn contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::Contains(val.into()),
        })
    }

    /// Field does not contain the given value.
    fn not_contains<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::NotContains(val.into()),
        })
    }

    /// Field starts with the given value.
    fn starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::StartsWith(val.into()),
        })
    }

    /// Field does not start with the given value.
    fn not_starts_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::NotStartsWith(val.into()),
        })
    }

    /// Field ends with the given value.
    fn ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::EndsWith(val.into()),
        })
    }

    /// Field does not end with the given value.
    fn not_ends_with<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::NotEndsWith(val.into()),
        })
    }

    /// Field is less than the given value.
    fn less_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::LessThan(val.into()),
        })
    }

    /// Field is less than or equals the given value.
    fn less_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::LessThanOrEquals(val.into()),
        })
    }

    /// Field is greater than the given value.
    fn greater_than<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::GreaterThan(val.into()),
        })
    }

    /// Field is greater than or equals the given value.
    fn greater_than_or_equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.data_source_fields().collect()),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{filter::*, *};

    #[test]
    fn equals() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();

        let filter = field.equals("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::Equals(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn not_equals() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.not_equals("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::NotEquals(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn contains() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.contains("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::Contains(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn not_contains() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.not_contains("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::NotContains(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn starts_with() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.starts_with("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::StartsWith(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn not_starts_with() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.not_starts_with("qwert");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::NotStartsWith(val),
            }) => {
                assert_eq!(PrismaValue::from("qwert"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn ends_with() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.ends_with("musti");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::EndsWith(val),
            }) => {
                assert_eq!(PrismaValue::from("musti"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn not_ends_with() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("name")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.not_ends_with("naukio");

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::NotEndsWith(val),
            }) => {
                assert_eq!(PrismaValue::from("naukio"), val);
                assert_eq!(String::from("name"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn less_than() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("id")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.less_than(10);

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::LessThan(val),
            }) => {
                assert_eq!(PrismaValue::from(10), val);
                assert_eq!(String::from("id"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn less_than_or_equals() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("id")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.less_than_or_equals(10);

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::LessThanOrEquals(val),
            }) => {
                assert_eq!(PrismaValue::from(10), val);
                assert_eq!(String::from("id"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn greater_than() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("id")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.greater_than(10);

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::GreaterThan(val),
            }) => {
                assert_eq!(PrismaValue::from(10), val);
                assert_eq!(String::from("id"), field.name);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn greater_then_or_equals() {
        let schema = test_data_model();
        let model = schema.find_model("User").unwrap();

        let field = model
            .fields()
            .find_from_scalar("id")
            .unwrap()
            .data_source_field()
            .clone();
        let filter = field.greater_than_or_equals(10);

        match filter {
            Filter::Scalar(ScalarFilter {
                projection: ScalarProjection::Single(field),
                condition: ScalarCondition::GreaterThanOrEquals(val),
            }) => {
                assert_eq!(PrismaValue::from(10), val);
                assert_eq!(String::from("id"), field.name);
            }
            _ => unreachable!(),
        }
    }
}
