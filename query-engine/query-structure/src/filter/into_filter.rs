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
        if let Some(pairs) = self.iter().exactly_one().ok().and_then(extract_placeholder_fields) {
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

fn extract_placeholder_fields(res: &SelectionResult) -> Option<Vec<(&ScalarFieldRef, &PrismaValue)>> {
    let pairs = res
        .pairs
        .iter()
        .map(|pair| {
            if let (SelectedField::Scalar(sf), value @ PrismaValue::Placeholder { .. }) = pair {
                Some((sf, value))
            } else {
                None
            }
        })
        .while_some()
        .collect_vec();
    if pairs.len() == res.pairs.len() {
        Some(pairs)
    } else {
        None
    }
}
