//! Prisma read query AST
use super::FilteredQuery;
use crate::ToGraphviz;
use enumflags2::BitFlags;
use query_structure::{AggregationSelection, Filter, QueryArguments, RelationLoadStrategy, Take, prelude::*};
use std::fmt::Display;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum ReadQuery {
    RecordQuery(RecordQuery),
    ManyRecordsQuery(ManyRecordsQuery),
    RelatedRecordsQuery(RelatedRecordsQuery),
    AggregateRecordsQuery(AggregateRecordsQuery),
}

impl ReadQuery {
    /// Returns true if the query is expected to return a single record.
    pub fn is_unique(&self) -> bool {
        match self {
            ReadQuery::RecordQuery(_) => true,
            ReadQuery::ManyRecordsQuery(q) => q.args.take == Take::One || q.args.take == Take::NegativeOne,
            ReadQuery::RelatedRecordsQuery(q) => q.args.take == Take::One || q.args.take == Take::NegativeOne,
            ReadQuery::AggregateRecordsQuery(_) => false,
        }
    }

    pub fn get_alias_or_name(&self) -> &str {
        match self {
            ReadQuery::RecordQuery(q) => q.alias.as_deref().unwrap_or(&q.name),
            ReadQuery::ManyRecordsQuery(q) => q.alias.as_deref().unwrap_or(&q.name),
            ReadQuery::RelatedRecordsQuery(q) => q.alias.as_deref().unwrap_or(&q.name),
            ReadQuery::AggregateRecordsQuery(q) => q.alias.as_deref().unwrap_or(&q.name),
        }
    }

    /// Checks whether or not the field selection of this query satisfies the inputted field selection.
    pub fn satisfies(&self, expected: &FieldSelection) -> bool {
        self.returns().map(|sel| sel.is_superset_of(expected)).unwrap_or(false)
    }

    /// Returns the field selection of a read query.
    fn returns(&self) -> Option<&FieldSelection> {
        match self {
            ReadQuery::RecordQuery(x) => Some(&x.selected_fields),
            ReadQuery::ManyRecordsQuery(x) => Some(&x.selected_fields),
            ReadQuery::RelatedRecordsQuery(x) => Some(&x.selected_fields),
            ReadQuery::AggregateRecordsQuery(_x) => None,
        }
    }

    /// Updates the field selection of the query to satisfy the inputted FieldSelection.
    pub fn satisfy_dependency(&mut self, field_selection: FieldSelection) {
        match self {
            ReadQuery::RecordQuery(x) => {
                x.selected_fields.merge_in_place(field_selection);
            }
            ReadQuery::ManyRecordsQuery(x) => {
                x.selected_fields.merge_in_place(field_selection);
            }
            ReadQuery::RelatedRecordsQuery(x) => {
                x.selected_fields.merge_in_place(field_selection);
            }
            ReadQuery::AggregateRecordsQuery(_) => (),
        }
    }

    pub fn model(&self) -> Model {
        match self {
            ReadQuery::RecordQuery(x) => x.model.clone(),
            ReadQuery::ManyRecordsQuery(x) => x.model.clone(),
            ReadQuery::RelatedRecordsQuery(x) => x.parent_field.related_field().model(),
            ReadQuery::AggregateRecordsQuery(x) => x.model.clone(),
        }
    }

    pub(crate) fn has_cursor(&self) -> bool {
        match self {
            ReadQuery::RecordQuery(_) => false,
            ReadQuery::ManyRecordsQuery(q) => q.args.cursor.is_some() || q.nested.iter().any(|q| q.has_cursor()),
            ReadQuery::RelatedRecordsQuery(q) => q.args.cursor.is_some() || q.nested.iter().any(|q| q.has_cursor()),
            ReadQuery::AggregateRecordsQuery(_) => false,
        }
    }
}

impl FilteredQuery for ReadQuery {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        match self {
            Self::RecordQuery(q) => q.get_filter(),
            Self::ManyRecordsQuery(q) => q.get_filter(),
            _ => unimplemented!(),
        }
    }

    fn set_filter(&mut self, filter: Filter) {
        match self {
            Self::RecordQuery(q) => q.set_filter(filter),
            Self::ManyRecordsQuery(q) => q.set_filter(filter),
            _ => unimplemented!(),
        }
    }
}

impl Display for ReadQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordQuery(q) => write!(
                f,
                "RecordQuery(name: '{}', selection: {}, filter: {:#?})",
                q.name, q.selected_fields, q.filter
            ),
            Self::ManyRecordsQuery(q) => write!(
                f,
                r#"ManyRecordsQuery(name: '{}', model: '{}', selection: {}, args: {:#?})"#,
                q.name,
                q.model.name(),
                q.selected_fields,
                q.args
            ),
            Self::RelatedRecordsQuery(q) => write!(
                f,
                "RelatedRecordsQuery(name: '{}', parent model: '{}', parent relation field: '{}', selection: {})",
                q.name,
                q.parent_field.model().name(),
                q.parent_field.name(),
                q.selected_fields
            ),
            Self::AggregateRecordsQuery(q) => write!(f, "AggregateRecordsQuery: {}", q.name),
        }
    }
}

impl ToGraphviz for ReadQuery {
    fn to_graphviz(&self) -> String {
        match self {
            Self::RecordQuery(q) => format!("RecordQuery(name: '{}', selection: {})", q.name, q.selected_fields),
            Self::ManyRecordsQuery(q) => format!(
                r#"ManyRecordsQuery(name: '{}', model: '{}', selection: {})"#,
                q.name,
                q.model.name(),
                q.selected_fields
            ),
            Self::RelatedRecordsQuery(q) => format!(
                "RelatedRecordsQuery(name: '{}', parent model: '{}', parent relation field: {}, selection: {})",
                q.name,
                q.parent_field.model().name(),
                q.parent_field.name(),
                q.selected_fields
            ),
            Self::AggregateRecordsQuery(q) => format!("AggregateRecordsQuery: {}", q.name),
        }
    }
}

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum QueryOption {
    ThrowOnEmpty,
    Other,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct QueryOptions(BitFlags<QueryOption>);

// Allows for: QueryOption::ThrowOnEmpty.into()  to be a QueryOptions
impl From<QueryOption> for QueryOptions {
    fn from(options: QueryOption) -> Self {
        QueryOptions(options.into())
    }
}

// Allows for: (QueryOption::ThrowOnEmpty | QueryOption::Other).into()  to be a QueryOptions
impl From<BitFlags<QueryOption>> for QueryOptions {
    fn from(options: BitFlags<QueryOption>) -> Self {
        QueryOptions(options)
    }
}

impl QueryOptions {
    pub fn none() -> Self {
        Self(BitFlags::empty())
    }

    pub fn contains(&self, option: QueryOption) -> bool {
        self.0.contains(option)
    }
}

#[derive(Debug, Clone)]
pub struct RecordQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: Model,
    pub filter: Option<Filter>,
    // TODO: split into `user_selection` and `full_selection` and get rid of `selection_order`
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    pub options: QueryOptions,
    pub relation_load_strategy: RelationLoadStrategy,
}

#[derive(Debug, Clone)]
pub struct ManyRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: Model,
    pub args: QueryArguments,
    // TODO: split into `user_selection` and `full_selection` and get rid of `selection_order`
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    pub options: QueryOptions,
    pub relation_load_strategy: RelationLoadStrategy,
}

#[derive(Debug, Clone)]
pub struct RelatedRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub parent_field: RelationFieldRef,
    pub args: QueryArguments,
    // TODO: split into `user_selection` and `full_selection` and get rid of `selection_order`
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    /// Fields and values of the parent to satisfy the relation query without
    /// relying on the parent result passed by the interpreter.
    pub parent_results: Option<Vec<SelectionResult>>,
}

impl RelatedRecordsQuery {
    pub fn has_cursor(&self) -> bool {
        self.args.cursor.is_some() || self.nested.iter().any(|q| q.has_cursor())
    }
}

#[derive(Debug, Clone)]
pub struct AggregateRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: Model,
    pub selection_order: Vec<(String, Option<Vec<String>>)>,
    pub args: QueryArguments,
    pub selectors: Vec<AggregationSelection>,
    pub group_by: Vec<ScalarFieldRef>,
    pub having: Option<Filter>,
}

impl FilteredQuery for RecordQuery {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.filter.as_mut()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.filter = Some(filter)
    }
}

impl FilteredQuery for ManyRecordsQuery {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.args.filter.as_mut()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.args.filter = Some(filter)
    }
}

impl FilteredQuery for RelatedRecordsQuery {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.args.filter.as_mut()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.args.filter = Some(filter)
    }
}

impl FilteredQuery for QueryArguments {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.filter.as_mut()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.filter = Some(filter)
    }
}
