use super::*;
use crate::ScalarCompare;
use prisma_models::RecordProjection;

pub trait IdFilter {
    fn filter(self) -> Filter;
}

impl IdFilter for RecordProjection {
    fn filter(self) -> Filter {
        let filters: Vec<Filter> = self
            .pairs
            .into_iter()
            .map(|(field, value)| field.equals(value))
            .collect();

        Filter::and(filters)
    }
}

impl IdFilter for Vec<RecordProjection> {
    fn filter(self) -> Filter {
        let filters = self.into_iter().fold(vec![], |mut acc, id| {
            acc.push(id.filter());
            acc
        });

        Filter::or(filters)
    }
}
