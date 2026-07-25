use prisma_value::PrismaValue;

use super::*;

use crate::ScalarCompare;
use crate::{SelectedField, SelectionResult};

pub trait IntoFilter {
    fn filter(self) -> Filter;
}

impl IntoFilter for SelectionResult {
    fn filter(self) -> Filter {
        let mut pairs = self.pairs.into_iter();

        let Some((selection, value)) = pairs.next() else {
            return Filter::and(Vec::new());
        };

        let first = into_scalar_filter(selection, value);

        let Some((selection, value)) = pairs.next() else {
            return first;
        };

        let mut filters = Vec::with_capacity(2 + pairs.size_hint().0);
        filters.push(first);
        filters.push(into_scalar_filter(selection, value));
        filters.extend(pairs.map(|(selection, value)| into_scalar_filter(selection, value)));

        Filter::and(filters)
    }
}

impl IntoFilter for Vec<SelectionResult> {
    fn filter(self) -> Filter {
        if let [result] = &self[..] {
            if let Some(filter) = placeholder_filter(result) {
                return filter;
            }
        }

        Filter::or(self.into_iter().map(|id| id.filter()).collect())
    }
}

fn into_scalar_filter(selection: SelectedField, value: PrismaValue) -> Filter {
    match selection {
        SelectedField::Scalar(sf) => sf.equals(value),
        SelectedField::Composite(_) => unreachable!(), // [Composites] todo
        SelectedField::Relation(_) => unreachable!(),
        SelectedField::Virtual(_) => unreachable!(),
    }
}

fn placeholder_filter(result: &SelectionResult) -> Option<Filter> {
    let mut pairs = result.pairs.iter();

    let Some((selection, value)) = pairs.next() else {
        return Some(Filter::and(Vec::new()));
    };

    let first = into_placeholder_filter(selection, value)?;

    let Some((selection, value)) = pairs.next() else {
        return Some(first);
    };

    let mut filters = Vec::with_capacity(2 + pairs.size_hint().0);
    filters.push(first);
    filters.push(into_placeholder_filter(selection, value)?);

    for (selection, value) in pairs {
        filters.push(into_placeholder_filter(selection, value)?);
    }

    Some(Filter::and(filters))
}

fn into_placeholder_filter(selection: &SelectedField, value: &PrismaValue) -> Option<Filter> {
    let (SelectedField::Scalar(sf), PrismaValue::Placeholder(p)) = (selection, value) else {
        return None;
    };

    Some(sf.is_in(p.clone()))
}
