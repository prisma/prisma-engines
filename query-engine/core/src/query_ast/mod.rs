mod read;
mod write;

pub use read::*;
pub use write::*;

use connector::filter::Filter;
use prisma_models::{ModelProjection, ModelRef};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Query {
    Read(ReadQuery),
    Write(WriteQuery),
}

impl Query {
    pub fn returns(&self, projection: &ModelProjection) -> bool {
        match self {
            Self::Read(rq) => rq.returns(projection),
            Self::Write(wq) => wq.returns(projection),
        }
    }

    pub fn model(&self) -> ModelRef {
        match self {
            Self::Read(rq) => rq.model(),
            Self::Write(wq) => wq.model(),
        }
    }
}

impl FilteredQuery for Query {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        match self {
            Self::Read(rq) => rq.get_filter(),
            Self::Write(wq) => wq.get_filter(),
        }
    }

    fn set_filter(&mut self, filter: Filter) {
        match self {
            Self::Read(rq) => rq.set_filter(filter),
            Self::Write(wq) => wq.set_filter(filter),
        }
    }
}

pub trait FilteredQuery {
    fn add_filter<T>(&mut self, filter: T)
    where
        T: Into<Filter>,
    {
        let filter = filter.into();
        let existing_filter = self.get_filter();
        let filter = match existing_filter {
            Some(Filter::And(ref mut v)) => {
                v.push(filter);
                None
            }
            Some(Filter::Or(ref mut v)) => {
                v.push(filter);
                None
            }
            Some(Filter::Not(ref mut v)) => {
                v.push(filter);
                None
            }
            Some(Filter::Empty) => Some(filter),
            Some(other) => Some(Self::default_filter_behaviour(vec![other.clone(), filter])),
            None => Some(filter),
        };

        if let Some(filter) = filter {
            self.set_filter(filter);
        }
    }

    fn get_filter(&mut self) -> Option<&mut Filter>;
    fn set_filter(&mut self, filter: Filter);

    fn default_filter_behaviour(inner_filters: Vec<Filter>) -> Filter {
        Filter::Or(inner_filters)
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Read(q) => write!(f, "{}", q),
            Self::Write(q) => write!(f, "{}", q),
        }
    }
}
