use crate::*;

use itertools::Itertools;
use prisma_models::*;

#[derive(Debug, Clone, Default)]
pub struct UniqueFilters {
    filters: Vec<UniqueFilter>,
}

impl UniqueFilters {
    pub fn add_filter(&mut self, filter: UniqueFilter) {
        self.filters.push(filter);
    }

    pub fn add_filters(&mut self, filters: Vec<UniqueFilter>) {
        self.filters.extend(filters);
    }

    pub fn filters(self) -> Vec<UniqueFilter> {
        self.filters
    }

    pub fn values(&self) -> impl Iterator<Item = &PrismaValue> + '_ {
        self.filters.iter().flat_map(|f| f.values())
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    pub fn into_filter(&self) -> Filter {
        let mut inner = vec![];

        for unique_filter in &self.filters {
            inner.extend(unique_filter.into_filters());
        }

        Filter::And(inner)
    }

    pub fn into_selection(&self) -> FieldSelection {
        let selections = self.filters.iter().map(|f| f.into_selection()).collect_vec();

        FieldSelection::new(selections)
    }
}

#[derive(Debug, Clone)]
pub enum UniqueFilter {
    // Represents a filter on a unique scalar field.
    Scalar(ScalarUniqueFilter),
    // Represents a filter on a unique composite field.
    Composite(CompositeUniqueFilter),
    // Represents a filter on partial unique composite fields.
    CompositePartial(CompositePartialUniqueFilter),
}

impl UniqueFilter {
    pub fn scalar(field: ScalarFieldRef, value: PrismaValue) -> Self {
        Self::Scalar(ScalarUniqueFilter { field, value })
    }

    pub fn composite(field: CompositeFieldRef, value: PrismaValue, nested: Vec<UniqueFilter>) -> Self {
        Self::Composite(CompositeUniqueFilter { field, value, nested })
    }

    pub fn composite_partial(field: CompositeFieldRef, nested: Vec<UniqueFilter>) -> Self {
        Self::CompositePartial(CompositePartialUniqueFilter { field, nested })
    }

    pub fn into_filters(&self) -> Vec<Filter> {
        match self {
            UniqueFilter::Scalar(suf) => vec![suf.into_filter()],
            UniqueFilter::Composite(cuf) => vec![cuf.into_filter()],
            UniqueFilter::CompositePartial(cpuf) => cpuf.into_filters(),
        }
    }

    pub fn into_selection(&self) -> SelectedField {
        match self {
            UniqueFilter::Scalar(suf) => suf.into_selection(),
            UniqueFilter::Composite(cuf) => cuf.into_selection(),
            UniqueFilter::CompositePartial(cpuf) => cpuf.into_selection(),
        }
    }

    pub fn values(&self) -> Vec<&PrismaValue> {
        match self {
            UniqueFilter::Scalar(suf) => vec![suf.value()],
            UniqueFilter::Composite(cuf) => vec![cuf.value()],
            UniqueFilter::CompositePartial(cpuf) => cpuf.values(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScalarUniqueFilter {
    field: ScalarFieldRef,
    value: PrismaValue,
}

impl ScalarUniqueFilter {
    pub fn value(&self) -> &PrismaValue {
        &self.value
    }

    pub fn into_selection(&self) -> SelectedField {
        SelectedField::Scalar(self.field.clone())
    }

    pub fn into_filter(&self) -> Filter {
        self.field.equals(self.value.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CompositeUniqueFilter {
    field: CompositeFieldRef,
    value: PrismaValue,
    nested: Vec<UniqueFilter>,
}

impl CompositeUniqueFilter {
    pub fn value(&self) -> &PrismaValue {
        &self.value
    }

    pub fn into_selection(&self) -> SelectedField {
        SelectedField::Composite(CompositeSelection {
            field: self.field.clone(),
            selections: self.nested.iter().map(|f| f.into_selection()).collect_vec(),
        })
    }

    pub fn into_filter(&self) -> Filter {
        if self.field.is_list() {
            self.field
                .some(Filter::And(self.nested.iter().flat_map(|n| n.into_filters()).collect()))
        } else {
            self.field.equals(self.value.clone())
        }
    }

    pub fn into_partial(self) -> CompositePartialUniqueFilter {
        CompositePartialUniqueFilter {
            field: self.field,
            nested: self.nested,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositePartialUniqueFilter {
    field: CompositeFieldRef,
    nested: Vec<UniqueFilter>,
}

impl CompositePartialUniqueFilter {
    pub fn values(&self) -> Vec<&PrismaValue> {
        self.nested.iter().flat_map(|p| p.values()).collect_vec()
    }

    pub fn into_filters(&self) -> Vec<Filter> {
        self.nested
            .iter()
            .map(|f| {
                if self.field.is_list() {
                    self.field.some(Filter::And(f.into_filters()))
                } else {
                    self.field.is(Filter::And(f.into_filters()))
                }
            })
            .collect_vec()
    }

    pub fn into_selection(&self) -> SelectedField {
        SelectedField::Composite(CompositeSelection {
            field: self.field.clone(),
            selections: self.nested.iter().map(|f| f.into_selection()).collect_vec(),
        })
    }
}
