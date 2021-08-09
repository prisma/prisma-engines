use super::Filter;
use crate::{compare::ScalarCompare, JsonFilterPath, JsonTargetType};
use prisma_models::{ModelProjection, PrismaListValue, PrismaValue, ScalarFieldRef};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScalarProjection {
    /// A single field projection.
    Single(ScalarFieldRef),

    /// A tuple projection, e.g. if (a, b) <in> ((1, 2), (1, 3), ...) is supposed to be queried.
    Compound(Vec<ScalarFieldRef>),
}

impl ScalarProjection {
    pub fn scalar_fields(&self) -> Vec<&ScalarFieldRef> {
        match self {
            ScalarProjection::Single(sf) => vec![sf],
            ScalarProjection::Compound(sfs) => sfs.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Filtering with a scalar value. From a GraphQL point of view this is in the
/// head of the query:
///
/// ```graphql
/// findManyUser(where: { id: 5 })
/// ```
///
/// This translates to a projection of one column `id` with a condition where
/// the column value equals `5`.
pub struct ScalarFilter {
    pub projection: ScalarProjection,
    pub condition: ScalarCondition,
    pub mode: QueryMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QueryMode {
    Default,
    Insensitive,
}

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

    /// Returns `true` if the number of values in the filter is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// If `true`, the filter should be split into smaller filters executed in
    /// separate queries.
    pub fn should_batch(&self, chunk_size: usize) -> bool {
        self.len() > chunk_size
    }

    /// If possible, converts the filter into multiple smaller filters.
    pub fn batched(self, chunk_size: usize) -> Vec<ScalarFilter> {
        fn inner(mut list: PrismaListValue, chunk_size: usize) -> Vec<PrismaListValue> {
            let dedup_list: BTreeSet<_> = list.drain(..).collect();

            let mut batches = Vec::with_capacity(list.len() % chunk_size + 1);
            batches.push(Vec::with_capacity(chunk_size));

            for (idx, item) in dedup_list.into_iter().enumerate() {
                if idx != 0 && idx % chunk_size == 0 {
                    batches.push(Vec::with_capacity(chunk_size));
                }

                batches.last_mut().unwrap().push(item);
            }

            batches
        }

        let mode = self.mode.clone();

        match self.condition {
            ScalarCondition::In(list) => {
                let projection = self.projection;

                inner(list, chunk_size)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::In(batch),
                        mode: mode.clone(),
                    })
                    .collect()
            }

            ScalarCondition::NotIn(list) => {
                let projection = self.projection;

                inner(list, chunk_size)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::NotIn(batch),
                        mode: mode.clone(),
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
    JsonCompare(JsonCondition),
    Search(PrismaValue, Vec<ScalarProjection>),
    NotSearch(PrismaValue, Vec<ScalarProjection>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonCondition {
    pub condition: Box<ScalarCondition>,
    pub path: Option<JsonFilterPath>,
    pub target_type: Option<JsonTargetType>,
}

impl ScalarCondition {
    pub fn invert(self, condition: bool) -> Self {
        if condition {
            match self {
                Self::Equals(v) => Self::NotEquals(v),
                Self::NotEquals(v) => Self::Equals(v),
                Self::Contains(v) => Self::NotContains(v),
                Self::NotContains(v) => Self::Contains(v),
                Self::StartsWith(v) => Self::NotStartsWith(v),
                Self::NotStartsWith(v) => Self::StartsWith(v),
                Self::EndsWith(v) => Self::NotEndsWith(v),
                Self::NotEndsWith(v) => Self::EndsWith(v),
                Self::LessThan(v) => Self::GreaterThanOrEquals(v),
                Self::LessThanOrEquals(v) => Self::GreaterThan(v),
                Self::GreaterThan(v) => Self::LessThanOrEquals(v),
                Self::GreaterThanOrEquals(v) => Self::LessThan(v),
                Self::In(v) => Self::NotIn(v),
                Self::NotIn(v) => Self::In(v),
                Self::JsonCompare(json_compare) => {
                    let inverted_cond = json_compare.condition.invert(true);

                    Self::JsonCompare(JsonCondition {
                        condition: Box::new(inverted_cond),
                        path: json_compare.path,
                        target_type: json_compare.target_type,
                    })
                }
                Self::Search(v, fields) => Self::NotSearch(v, fields),
                Self::NotSearch(v, fields) => Self::Search(v, fields),
            }
        } else {
            self
        }
    }
}

impl ScalarCompare for ScalarFieldRef {
    /// Field is in a given value
    fn is_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::In(values.into_iter().map(|i| i.into()).collect()),
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
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
            mode: QueryMode::Default,
        })
    }

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Single(Arc::clone(self)),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
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
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::In(values.into_iter().map(|i| i.into()).collect()),
            mode: QueryMode::Default,
        })
    }

    /// Field is not in a given value
    fn not_in<T>(&self, values: Vec<T>) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::NotIn(values.into_iter().map(|i| i.into()).collect()),
            mode: QueryMode::Default,
        })
    }

    /// Field equals the given value.
    fn equals<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
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
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::GreaterThanOrEquals(val.into()),
            mode: QueryMode::Default,
        })
    }

    fn search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }

    fn not_search<T>(&self, val: T) -> Filter
    where
        T: Into<PrismaValue>,
    {
        Filter::from(ScalarFilter {
            projection: ScalarProjection::Compound(self.scalar_fields().collect()),
            condition: ScalarCondition::Search(val.into(), vec![]),
            mode: QueryMode::Default,
        })
    }
}
