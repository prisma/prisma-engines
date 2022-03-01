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
    pub cursor: Option<SelectionResult>,
    pub take: Option<i64>,
    pub skip: Option<i64>,
    pub filter: Option<Filter>,
    pub order_by: Vec<OrderBy>,
    pub distinct: Option<FieldSelection>,
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

    /// We can't execute all operations on the DB level reliably.
    /// This is a marker that generally expresses whether or not a set of records can be
    /// retrieved by the connector or if it requires the query engine to fetch a raw set
    /// of records and perform certain operations itself, in-memory.
    pub fn requires_inmemory_processing(&self) -> bool {
        self.distinct.is_some() || self.contains_unstable_cursor() || self.contains_null_cursor()
    }

    /// An unstable cursor is a cursor that is used in conjunction with an unstable (non-unique) combination of orderBys.
    pub fn contains_unstable_cursor(&self) -> bool {
        self.cursor.is_some() && !self.is_stable_ordering()
    }

    /// A null cursor is a cursor that is used in conjunction with a nullable order by (i.e. a field is optional).
    pub fn contains_null_cursor(&self) -> bool {
        self.cursor.is_some()
            && self.order_by.iter().any(|o| match o {
                OrderBy::Scalar(o) => !o.field.is_required(),
                _ => false,
            })
    }

    /// Checks if the orderBy provided is guaranteeing a stable ordering of records for the model.
    /// For that purpose we need to distinguish orderings on the source model, i.e. the model that
    /// we're sorting on the top level (where orderBys are located that are done without relations)
    /// and orderings that require a relation or composite hop. Scalar orderings that require a hop are
    /// only guaranteed stable ordering if they are strictly over 1:1. As soon as there's
    /// a m:1 (or m:n for later implementations) hop involved a unique on the to-one side can't
    /// be considered unique anymore for the purpose of ordering records, as many left hand records
    /// (the many side) can have the one side. A simple example would be a User <> Post relation
    /// where a post can have only one author but an author (User) can have many posts. If posts
    /// are ordered by related author id, then we can't reliably order posts, as the following can happen:
    /// ```text
    /// post_id, post_title, author_id
    /// 1        post1       1
    /// 2        post2       1
    /// 3        post3       2
    /// ```
    /// So even though the id is unique, it's not guaranteeing a stable ordering in the context of orderBy here.
    ///
    /// Returns:
    /// - `true`, if:
    ///      * no orderings are done, or ...
    ///      * at least one unique field is present on the source model `orderBy`, or ...
    ///      * source model contains a combination of fields that is marked as unique, or ...
    ///      * an orderBy hop contains a unique and is done solely over 1:1 relations.
    /// - `false` otherwise.
    pub fn is_stable_ordering(&self) -> bool {
        if self.order_by.is_empty() {
            return true;
        }

        // We're filtering order by aggregation & relevance since they will never lead to stable ordering anyway.
        let stable_candidates: Vec<_> = self
            .order_by
            .iter()
            .filter_map(|o| match o {
                OrderBy::Scalar(o) => Some(o),
                _ => None,
            })
            .collect();

        // Partition into orderings on the same model and ones that require hops.
        // Note: One ordering is always on one scalar in the end.
        let (on_model, on_relation): (Vec<&OrderByScalar>, Vec<&OrderByScalar>) =
            stable_candidates.iter().partition(|o| o.path.is_empty());

        // Indicates whether or not a combination of contained fields is on the source model (we don't check for relations for now).
        let order_by_contains_unique_index = self.model.unique_indexes().into_iter().any(|index| {
            index
                .fields()
                .into_iter()
                .all(|f| on_model.iter().any(|o| o.field == f))
        });

        let source_contains_unique = on_model.iter().any(|o| o.field.unique());
        let relations_contain_1to1_unique = on_relation.iter().any(|o| {
            o.field.unique()
                && o.path.iter().all(|hop| match hop {
                    OrderByHop::Relation(rf) => rf.relation().is_one_to_one(),
                    OrderByHop::Composite(_) => false, // Composites do not have uniques, as such they can't fulfill uniqueness requirement even if they're 1:1.
                })
        });

        let has_optional_hop = on_relation.iter().any(|o| {
            o.path.iter().any(|hop| match hop {
                OrderByHop::Relation(rf) => rf.arity == dml::FieldArity::Optional,
                OrderByHop::Composite(cf) => !cf.is_required(),
            })
        });

        // [Dom] I'm not entirely sure why we're doing this, but I assume that optionals introduce NULLs that make the ordering inherently unstable?
        if has_optional_hop {
            return false;
        }

        source_contains_unique || order_by_contains_unique_index || relations_contain_1to1_unique
    }

    pub fn take_abs(&self) -> Option<i64> {
        self.take.map(|t| if t < 0 { -t } else { t })
    }

    pub fn has_unbatchable_ordering(&self) -> bool {
        self.order_by.iter().any(|o| !matches!(o, OrderBy::Scalar(_)))
    }

    pub fn has_unbatchable_filters(&self) -> bool {
        match &self.filter {
            None => false,
            Some(filter) => !filter.can_batch(),
        }
    }

    pub fn should_batch(&self, chunk_size: usize) -> bool {
        self.filter
            .as_ref()
            .map(|filter| filter.should_batch(chunk_size))
            .unwrap_or(false)
            && self.cursor.is_none()
    }

    pub fn batched(self, chunk_size: usize) -> Vec<Self> {
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
                    .batched(chunk_size)
                    .into_iter()
                    .map(|filter| QueryArguments {
                        model: model.clone(),
                        cursor: cursor.clone(),
                        take,
                        skip,
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
