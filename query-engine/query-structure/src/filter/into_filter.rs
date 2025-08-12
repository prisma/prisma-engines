use alloc::vec::Vec;
use itertools::Itertools;

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
                SelectedField::Composite(_) => unreachable!(), // [Composites] todo
                SelectedField::Relation(_) => unreachable!(),
                SelectedField::Virtual(_) => unreachable!(),
            })
            .collect();

        Filter::and(filters)
    }
}

impl IntoFilter for Vec<SelectionResult> {
    fn filter(self) -> Filter {
        if let Some(pairs) = self
            .iter()
            .exactly_one()
            .ok()
            .and_then(SelectionResult::as_placeholders)
        {
            return Filter::and(pairs.into_iter().fold(vec![], |mut acc, (sf, val)| {
                acc.push(sf.is_in_template(val.clone()));
                acc
            }));
        }

        let filters = self.into_iter().fold(vec![], |mut acc, id| {
            acc.push(id.filter());
            acc
        });

        Filter::or(filters)
    }
}
