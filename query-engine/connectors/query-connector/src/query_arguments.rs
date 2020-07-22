use crate::filter::Filter;
use prisma_models::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkipAndLimit {
    pub skip: usize,
    pub limit: Option<usize>,
}

/// `QueryArguments` define various constraints queried data should fulfill:
/// - `cursor`, `take`, `skip` page through the data.
/// - `filter` scopes the data by defining conditions (akin to `WHERE` in SQL).
/// - `order_by` defines the ordering of records.
/// - `distinct` designates the fields on which the records should be distinct.
/// - The `ignore_*` flags are a temporary bandaid to tell the connector to do not
///   include certain constraints when building queries, because the core is already
///   performing these action in a different manner (e.g. in-memory on all records).
#[derive(Debug, Default, Clone)]
pub struct QueryArguments {
    pub cursor: Option<RecordProjection>,
    pub take: Option<i64>,
    pub skip: Option<i64>,
    pub filter: Option<Filter>,
    pub order_by: Vec<OrderBy>,
    pub distinct: Option<ModelProjection>,
    pub ignore_skip: bool,
    pub ignore_take: bool,
}

impl QueryArguments {
    // pub fn ordering_directions(&self) -> OrderDirections {
    //     OrderDirections {
    //         needs_implicit_id_ordering: self.needs_implicit_ordering(),
    //         primary_order_by: self.order_by.clone(),
    //     }
    // }

    pub fn take_abs(&self) -> Option<i64> {
        self.take.clone().map(|t| if t < 0 { t * -1 } else { t })
    }

    pub fn can_batch(&self) -> bool {
        self.filter.as_ref().map(|filter| filter.can_batch()).unwrap_or(false)
    }

    pub fn batched(self) -> Vec<Self> {
        match self.filter {
            Some(filter) => {
                let cursor = self.cursor;
                let take = self.take;
                let skip = self.skip;
                let order_by = self.order_by;
                let distinct = self.distinct;
                let ignore_skip = self.ignore_skip;
                let ignore_take = self.ignore_take;

                filter
                    .batched()
                    .into_iter()
                    .map(|filter| QueryArguments {
                        cursor: cursor.clone(),
                        take: take.clone(),
                        skip: skip.clone(),
                        filter: Some(filter),
                        order_by: order_by.clone(),
                        distinct: distinct.clone(),
                        ignore_skip,
                        ignore_take,
                    })
                    .collect()
            }
            _ => vec![self],
        }
    }
}

impl<T> From<T> for QueryArguments
where
    T: Into<Filter>,
{
    fn from(filter: T) -> Self {
        let mut query_arguments = Self::default();
        query_arguments.filter = Some(filter.into());
        query_arguments
    }
}

// pub struct OrderDirections {
//     pub needs_implicit_id_ordering: bool,
//     pub primary_order_by: Option<OrderBy>,
// }
