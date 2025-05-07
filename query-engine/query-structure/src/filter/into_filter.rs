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
        if let Ok([(SelectedField::Scalar(sf), value @ PrismaValue::Placeholder { .. })]) =
            self.iter().exactly_one().map(|res| &res.pairs[..])
        {
            return sf.is_in_template(value.clone());
        };

        let filters = self.into_iter().fold(vec![], |mut acc, id| {
            acc.push(id.filter());
            acc
        });

        Filter::or(filters)
    }
}
