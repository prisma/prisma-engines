use itertools::Itertools;
use prisma_value::PrismaValue;

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
        match self
            .iter()
            .exactly_one()
            .ok()
            .and_then(SelectionResult::as_placeholders)
        {
            Some(pairs) => Filter::and(
                pairs
                    .into_iter()
                    .map(|(sf, val)| {
                        let PrismaValue::Placeholder(p) = val else {
                            unreachable!("as_placeholders guarantees all values are placeholders")
                        };
                        sf.is_in(p.clone())
                    })
                    .collect(),
            ),
            None => Filter::or(self.into_iter().map(|id| id.filter()).collect()),
        }
    }
}
