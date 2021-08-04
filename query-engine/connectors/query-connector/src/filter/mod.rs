//! Filtering types to select records from the database
//!
//! The creation of the types should be done with
//! [ScalarCompare](/query-connector/trait.ScalarCompare.html) and
//! [RelationCompare](/query-connector/trait.RelationCompare.html).

mod id_filter;
mod json;
mod list;
mod relation;
mod scalar;

pub use id_filter::*;
pub use json::*;
pub use list::*;
pub use relation::*;
pub use scalar::*;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Vec<Filter>),
    Scalar(ScalarFilter),
    ScalarList(ScalarListFilter),
    OneRelationIsNull(OneRelationIsNullFilter),
    Relation(RelationFilter),
    BoolFilter(bool),
    Aggregation(AggregationFilter),
    Empty,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum AggregationFilter {
    Count(Box<Filter>),
    Average(Box<Filter>),
    Sum(Box<Filter>),
    Min(Box<Filter>),
    Max(Box<Filter>),
}

impl Filter {
    pub fn and(filters: Vec<Filter>) -> Self {
        Filter::And(filters)
    }

    pub fn or(filters: Vec<Filter>) -> Self {
        Filter::Or(filters)
    }

    pub fn not(filters: Vec<Filter>) -> Self {
        Filter::Not(filters)
    }

    pub fn empty() -> Self {
        Filter::Empty
    }

    /// Returns the size of the topmost filter elements (does not recursively compute the size).
    pub fn size(&self) -> usize {
        match self {
            Self::And(v) => v.len(),
            Self::Or(v) => v.len(),
            Self::Not(v) => v.len(),
            Self::Empty => 0,
            _ => 1,
        }
    }

    pub fn should_batch(&self, chunk_size: usize) -> bool {
        match self {
            Self::Scalar(sf) => sf.should_batch(chunk_size),
            Self::And(filters) => filters.iter().any(|f| f.should_batch(chunk_size)),
            Self::Or(filters) => filters.iter().any(|f| f.should_batch(chunk_size)),
            _ => false,
        }
    }

    pub fn batched(self, chunk_size: usize) -> Vec<Filter> {
        fn split_longest(mut filters: Vec<Filter>, chunk_size: usize) -> (Option<ScalarFilter>, Vec<Filter>) {
            let mut longest: Option<ScalarFilter> = None;
            let mut other = Vec::with_capacity(filters.len());

            while let Some(filter) = filters.pop() {
                match (filter, longest.as_mut()) {
                    (Filter::Scalar(sf), Some(ref mut prev)) if sf.len() > prev.len() => {
                        let previous = longest.replace(sf);
                        other.push(Filter::Scalar(previous.unwrap()));
                    }
                    (Filter::Scalar(sf), None) if sf.should_batch(chunk_size) => {
                        longest = Some(sf);
                    }
                    (filter, _) => other.push(filter),
                }
            }

            (longest, other)
        }

        fn batch<F>(filters: Vec<Filter>, chunk_size: usize, f: F) -> Vec<Filter>
        where
            F: Fn(Vec<Filter>) -> Filter,
        {
            let (longest, other) = split_longest(filters, chunk_size);
            let mut batched = Vec::new();

            if let Some(filter) = longest {
                for filter in filter.batched(chunk_size) {
                    batched.push(Filter::Scalar(filter))
                }

                batched
                    .into_iter()
                    .map(|batch| {
                        let mut filters = other.clone();
                        filters.push(batch);

                        f(filters)
                    })
                    .collect()
            } else {
                vec![f(other)]
            }
        }

        match self {
            Self::Scalar(sf) => sf.batched(chunk_size).into_iter().map(Self::Scalar).collect(),
            Self::And(filters) => batch(filters, chunk_size, Filter::And),
            Self::Or(filters) => batch(filters, chunk_size, Filter::Or),
            _ => vec![self],
        }
    }

    pub fn set_mode(&mut self, mode: QueryMode) {
        match self {
            Filter::And(inner) => inner.iter_mut().for_each(|f| f.set_mode(mode.clone())),
            Filter::Or(inner) => inner.iter_mut().for_each(|f| f.set_mode(mode.clone())),
            Filter::Not(inner) => inner.iter_mut().for_each(|f| f.set_mode(mode.clone())),
            Filter::Scalar(sf) => sf.mode = mode,
            _ => {}
        }
    }

    pub fn count(condition: Filter) -> Self {
        Self::Aggregation(AggregationFilter::Count(Box::new(condition)))
    }

    pub fn average(condition: Filter) -> Self {
        Self::Aggregation(AggregationFilter::Average(Box::new(condition)))
    }

    pub fn sum(condition: Filter) -> Self {
        Self::Aggregation(AggregationFilter::Sum(Box::new(condition)))
    }

    pub fn min(condition: Filter) -> Self {
        Self::Aggregation(AggregationFilter::Min(Box::new(condition)))
    }

    pub fn max(condition: Filter) -> Self {
        Self::Aggregation(AggregationFilter::Max(Box::new(condition)))
    }
}

impl From<ScalarFilter> for Filter {
    fn from(sf: ScalarFilter) -> Self {
        Filter::Scalar(sf)
    }
}

impl From<ScalarListFilter> for Filter {
    fn from(sf: ScalarListFilter) -> Self {
        Filter::ScalarList(sf)
    }
}

impl From<OneRelationIsNullFilter> for Filter {
    fn from(sf: OneRelationIsNullFilter) -> Self {
        Filter::OneRelationIsNull(sf)
    }
}

impl From<RelationFilter> for Filter {
    fn from(sf: RelationFilter) -> Self {
        Filter::Relation(sf)
    }
}

impl From<bool> for Filter {
    fn from(b: bool) -> Self {
        Filter::BoolFilter(b)
    }
}
