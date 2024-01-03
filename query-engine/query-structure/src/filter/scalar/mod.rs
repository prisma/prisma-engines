mod compare;
mod condition;
mod projection;

pub use condition::*;
pub use projection::*;

use crate::*;

use std::collections::BTreeSet;

/// Filtering with a scalar value. From a GraphQL point of view this is in the
/// head of the query:
///
/// ```graphql
/// findManyUser(where: { id: 5 })
/// ```
///
/// This translates to a projection of one column `id` with a condition where
/// the column value equals `5`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    pub fn can_batch(&self) -> bool {
        !matches!(
            self.condition,
            ScalarCondition::NotContains(_)
                | ScalarCondition::NotEquals(_)
                | ScalarCondition::NotIn(_)
                | ScalarCondition::NotSearch(..)
                | ScalarCondition::NotStartsWith(_)
                | ScalarCondition::NotEndsWith(_)
        )
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
            ScalarCondition::In(ConditionListValue::List(list)) => {
                let projection = self.projection;

                inner(list, chunk_size)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::In(ConditionListValue::list(batch)),
                        mode: mode.clone(),
                    })
                    .collect()
            }

            ScalarCondition::NotIn(ConditionListValue::List(list)) => {
                let projection = self.projection;

                inner(list, chunk_size)
                    .into_iter()
                    .map(|batch| ScalarFilter {
                        projection: projection.clone(),
                        condition: ScalarCondition::NotIn(ConditionListValue::list(batch)),
                        mode: mode.clone(),
                    })
                    .collect()
            }
            _ => vec![self],
        }
    }

    /// Returns the referenced scalar field if there is one.
    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        self.condition.as_field_ref()
    }

    /// Returns all the scalar fields related to a scalar filter.
    /// It also includes the referenced field if there is one.
    pub fn scalar_fields(&self) -> Vec<&ScalarFieldRef> {
        let mut fields = self.projection.scalar_fields();

        if let Some(field_ref) = self.as_field_ref() {
            fields.push(field_ref);
        }

        fields
    }

    pub fn is_unique(&self) -> bool {
        if let Some(sf) = self.scalar_ref() {
            sf.unique()
        } else {
            false
        }
    }

    pub fn scalar_ref(&self) -> Option<&ScalarFieldRef> {
        self.projection.as_single()
    }
}
