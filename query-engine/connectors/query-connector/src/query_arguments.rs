use crate::filter::Filter;
use prisma_models::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkipAndLimit {
    pub skip: usize,
    pub limit: Option<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct QueryArguments {
    pub after: Option<RecordProjection>,
    pub before: Option<RecordProjection>,
    pub skip: Option<i64>,
    pub first: Option<i64>,
    pub last: Option<i64>,
    pub filter: Option<Filter>,
    pub order_by: Option<OrderBy>,
}

impl QueryArguments {
    fn needs_reversed_order(&self) -> bool {
        self.last.is_some()
    }

    fn needs_implicit_ordering(&self) -> bool {
        self.skip.is_some()
            || self.after.is_some()
            || self.first.is_some()
            || self.before.is_some()
            || self.last.is_some()
            || self.order_by.is_some()
    }

    pub fn ordering_directions(&self) -> OrderDirections {
        OrderDirections {
            needs_to_be_reverse_order: self.needs_reversed_order(),
            needs_implicit_id_ordering: self.needs_implicit_ordering(),
            primary_order_by: self.order_by.clone(),
        }
    }

    pub fn is_with_pagination(&self) -> bool {
        self.last.or(self.first).or(self.skip).is_some()
    }

    pub fn window_limits(&self) -> (i64, i64) {
        let skip = self.skip.unwrap_or(0) + 1;

        match self.last.or(self.first) {
            Some(limited_count) => (skip, limited_count + skip),
            None => (skip, 100_000_000),
        }
    }

    pub fn skip_and_limit(&self) -> SkipAndLimit {
        match self.last.or(self.first) {
            Some(limited_count) => SkipAndLimit {
                skip: self.skip.unwrap_or(0) as usize,
                limit: Some((limited_count + 1) as usize),
            },
            None => SkipAndLimit {
                skip: self.skip.unwrap_or(0) as usize,
                limit: None,
            },
        }
    }

    pub fn can_batch(&self) -> bool {
        self.filter.as_ref().map(|filter| filter.can_batch()).unwrap_or(false)
    }

    pub fn batched(self) -> Vec<Self> {
        match self.filter {
            Some(filter) => {
                let after = self.after;
                let before = self.before;
                let skip = self.skip;
                let first = self.first;
                let last = self.last;
                let order_by = self.order_by;

                filter
                    .batched()
                    .into_iter()
                    .map(|filter| QueryArguments {
                        after: after.clone(),
                        before: before.clone(),
                        skip: skip.clone(),
                        first: first.clone(),
                        last: last.clone(),
                        filter: Some(filter),
                        order_by: order_by.clone(),
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

pub struct OrderDirections {
    pub needs_implicit_id_ordering: bool,
    pub needs_to_be_reverse_order: bool,
    pub primary_order_by: Option<OrderBy>,
}
