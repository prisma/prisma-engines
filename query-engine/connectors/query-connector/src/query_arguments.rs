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
/// - `order_by` defines the ordering of records, from most high to low precedence.
/// - `distinct` designates the fields on which the records should be distinct.
/// - The `ignore_*` flags are a temporary bandaid to tell the connector to do not
///   include certain constraints when building queries, because the core is already
///   performing these action in a different manner (e.g. in-memory on all records).
///
/// A query argument struct is always valid over a single model only, meaning that all
/// data referenced in a single query argument instance is always refering to data of
/// a single model (e.g. the cursor projection, distinct projection, orderby, ...).
#[derive(Debug, Clone)]
pub struct QueryArguments {
    pub model: ModelRef,
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
    pub fn new(model: ModelRef) -> Self {
        Self {
            model,
            cursor: None,
            take: None,
            skip: None,
            filter: None,
            order_by: vec![],
            distinct: None,
            ignore_take: false,
            ignore_skip: false,
        }
    }

    pub fn do_nothing(&self) -> bool {
        self.cursor.is_none()
            && self.take.is_none()
            && self.skip.is_none()
            && self.filter.is_none()
            && self.order_by.is_empty()
            && self.distinct.is_none()
    }

    /// An unstable cursor is a cursor that is used in conjunction with an unstable (non-unique) combination of orderBys.
    pub fn contains_unstable_cursor(&self) -> bool {
        self.cursor.is_some() && !self.is_stable_ordering()
    }

    /// A null cursor is a cursor that is used in conjunction with a nullable order by (i.e. a field is optional).
    pub fn contains_null_cursor(&self) -> bool {
        self.cursor.is_some() && self.order_by.iter().any(|o| !o.field.is_required)
    }

    /// Checks if the orderBy provided is guaranteeing a stable ordering of records for the model. Assumes that `model`
    /// is the same as the model used
    /// `true` if at least one unique field is present, or contains a combination of fields that is marked as unique.
    /// `false` otherwise.
    pub fn is_stable_ordering(&self) -> bool {
        let order_fields: Vec<_> = self.order_by.iter().map(|o| &o.field).collect();

        !self.order_by.is_empty()
            && (self.order_by.iter().any(|o| o.field.unique())
                || self
                    .model
                    .unique_indexes()
                    .into_iter()
                    .any(|index| index.fields().into_iter().all(|f| order_fields.contains(&&f))))
    }

    pub fn take_abs(&self) -> Option<i64> {
        self.take.clone().map(|t| if t < 0 { t * -1 } else { t })
    }

    pub fn can_batch(&self) -> bool {
        self.filter.as_ref().map(|filter| filter.can_batch()).unwrap_or(false) && self.cursor.is_none()
    }

    pub fn batched(self) -> Vec<Self> {
        match self.filter {
            Some(filter) => {
                let model = self.model;
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
                        model: model.clone(),
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

    pub fn model(&self) -> &ModelRef {
        &self.model
    }
}

impl<T> From<(ModelRef, T)> for QueryArguments
where
    T: Into<Filter>,
{
    fn from(model_filter: (ModelRef, T)) -> Self {
        let mut query_arguments = Self::new(model_filter.0);
        query_arguments.filter = Some(model_filter.1.into());
        query_arguments
    }
}
