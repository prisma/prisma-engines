use super::*;

use crate::ScalarCompare;
use crate::{SelectedField, SelectionResult};

pub trait IntoFilter {
    fn filter(self) -> Filter;
}

impl IntoFilter for SelectionResult {
    fn filter(self) -> Filter {
        let filters: Vec<Filter> = self
            .pairs
            .into_iter()
            .map(|(selection, value)| match selection {
                SelectedField::Scalar(sf) => sf.equals(value),
                SelectedField::Composite(_) => todo!(), // [Composites] todo
            })
            .collect();

        Filter::and(filters)
    }
}

impl IntoFilter for Vec<SelectionResult> {
    fn filter(self) -> Filter {
        let filters = self.into_iter().fold(vec![], |mut acc, id| {
            acc.push(id.filter());
            acc
        });

        Filter::or(filters)
    }
}
