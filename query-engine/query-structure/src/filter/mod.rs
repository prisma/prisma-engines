//! Filtering types to select records from the database
//!
//! The creation of the types should be done with
//! [ScalarCompare](/query-connector/trait.ScalarCompare.html) and
//! [RelationCompare](/query-connector/trait.RelationCompare.html).
//! [CompositeCompare](/query-connector/trait.RelationCompare.html).

mod compare;
mod composite;
mod into_filter;
mod json;
mod list;
mod relation;
mod scalar;

pub use compare::*;
pub use composite::*;
pub use into_filter::*;
pub use json::*;
pub use list::*;
pub use relation::*;
pub use scalar::*;

use crate::ScalarFieldRef;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Vec<Filter>),
    Scalar(ScalarFilter),
    ScalarList(ScalarListFilter),
    OneRelationIsNull(OneRelationIsNullFilter),
    Relation(RelationFilter),
    Composite(CompositeFilter),
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

impl AggregationFilter {
    pub fn filter(&self) -> &Filter {
        match self {
            AggregationFilter::Count(f) => f,
            AggregationFilter::Average(f) => f,
            AggregationFilter::Sum(f) => f,
            AggregationFilter::Min(f) => f,
            AggregationFilter::Max(f) => f,
        }
    }
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

    pub fn can_batch(&self) -> bool {
        match self {
            Self::Scalar(sf) => sf.can_batch(),
            Self::And(filters) => filters.iter().all(|f| f.can_batch()),
            Self::Or(filters) => filters.iter().all(|f| f.can_batch()),
            _ => true,
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

    pub fn as_scalar(&self) -> Option<&ScalarFilter> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn into_scalar(self) -> Option<ScalarFilter> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &Filter::Empty
    }

    pub fn scalars(&self) -> Vec<ScalarFieldRef> {
        let mut scalars: Vec<ScalarFieldRef> = Vec::new();

        let filter_check = |_sf: &ScalarFilter| true;
        Self::filter_and_collect_scalars(self, filter_check, &mut scalars);
        scalars
    }

    pub fn unique_scalars(&self) -> Vec<ScalarFieldRef> {
        let mut uniques: Vec<ScalarFieldRef> = Vec::new();

        let filter_check = |sf: &ScalarFilter| sf.is_unique();
        Self::filter_and_collect_scalars(self, filter_check, &mut uniques);
        uniques
    }

    /// Returns true if filter contains conditions on relation fields.
    pub fn has_relations(&self) -> bool {
        use AggregationFilter::*;
        use Filter::*;
        match self {
            Not(branches) | Or(branches) | And(branches) => branches.iter().any(|filter| filter.has_relations()),
            Scalar(..) | ScalarList(..) | Composite(..) | BoolFilter(..) | Empty => false,
            Aggregation(filter) => match filter {
                Average(filter) | Count(filter) | Sum(filter) | Min(filter) | Max(filter) => filter.has_relations(),
            },
            OneRelationIsNull(..) | Relation(..) => true,
        }
    }

    fn filter_and_collect_scalars(
        filter: &Filter,
        filter_check: fn(&ScalarFilter) -> bool,
        scalars: &mut Vec<ScalarFieldRef>,
    ) {
        match filter {
            Filter::And(inner) => inner
                .iter()
                .for_each(|f| Self::filter_and_collect_scalars(f, filter_check, scalars)),
            Filter::Scalar(sf) => {
                if filter_check(sf) {
                    if let Some(field) = sf.scalar_ref() {
                        scalars.push(field.to_owned())
                    }
                }
            }
            _ => (),
        }
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

impl From<CompositeFilter> for Filter {
    fn from(cf: CompositeFilter) -> Self {
        Filter::Composite(cf)
    }
}
