//! Prisma read query AST
use super::FilteredQuery;
use connector::{filter::Filter, AggregationSelection, QueryArguments, RelAggregationSelection};
use itertools::Itertools;
use prisma_models::{dml::FieldArity, prelude::*};
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum ReadQuery {
    RecordQuery(RecordQuery),
    ManyRecordsQuery(ManyRecordsQuery),
    RelatedRecordsQuery(RelatedRecordsQuery),
    AggregateRecordsQuery(AggregateRecordsQuery),
}

impl ReadQuery {
    pub fn name(&self) -> &str {
        match self {
            ReadQuery::RecordQuery(x) => &x.name,
            ReadQuery::ManyRecordsQuery(x) => &x.name,
            ReadQuery::RelatedRecordsQuery(x) => &x.name,
            ReadQuery::AggregateRecordsQuery(x) => &x.name,
        }
    }

    /// Checks whether or not this query returns a specific set of fields from the underlying data source model.
    pub fn returns(&self, field_selection: &FieldSelection) -> bool {
        match self {
            ReadQuery::RecordQuery(x) => x.selected_fields.is_superset_of(field_selection),
            ReadQuery::ManyRecordsQuery(x) => x.selected_fields.is_superset_of(field_selection),
            ReadQuery::RelatedRecordsQuery(x) => x.selected_fields.is_superset_of(field_selection),
            ReadQuery::AggregateRecordsQuery(_x) => false,
        }
    }

    pub fn model(&self) -> ModelRef {
        match self {
            ReadQuery::RecordQuery(x) => x.model.clone(),
            ReadQuery::ManyRecordsQuery(x) => x.model.clone(),
            ReadQuery::RelatedRecordsQuery(x) => x.parent_field.related_field().model(),
            ReadQuery::AggregateRecordsQuery(x) => x.model.clone(),
        }
    }

    pub fn as_related_records_query(&self) -> Option<&RelatedRecordsQuery> {
        if let Self::RelatedRecordsQuery(v) = self {
            Some(v)
        } else {
            None
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::RecordQuery(q) => write!(
                f,
                "RecordQuery(name: '{}', selection: {}, filter: {:?})",
                q.name, q.selected_fields, q.filter
            ),
            Self::ManyRecordsQuery(q) => write!(
                f,
                r#"ManyRecordsQuery(name: '{}', model: '{}', selection: {}, args: {:?})"#,
                q.name, q.model.name, q.selected_fields, q.args
            ),
            Self::RelatedRecordsQuery(q) => write!(
                f,
                "RelatedRecordsQuery(name: '{}', parent model: '{}', parent relation field: {}, selection: {})",
                q.name,
                q.parent_field.model().name,
                q.parent_field.name,
                q.selected_fields
            ),
            Self::AggregateRecordsQuery(q) => write!(f, "AggregateRecordsQuery: {}", q.name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: ModelRef,
    pub filter: Option<Filter>,
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    pub aggregation_selections: Vec<RelAggregationSelection>,
}

#[derive(Debug, Clone)]
pub struct ManyRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: ModelRef,
    pub args: QueryArguments,
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    pub aggregation_selections: Vec<RelAggregationSelection>,
}

#[derive(Debug, Clone)]
pub struct RelatedRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub parent_field: RelationFieldRef,
    pub args: QueryArguments,
    pub selected_fields: FieldSelection,
    pub nested: Vec<ReadQuery>,
    pub selection_order: Vec<String>,
    pub aggregation_selections: Vec<RelAggregationSelection>,

    /// Fields and values of the parent to satisfy the relation query without
    /// relying on the parent result passed by the interpreter.
    pub parent_results: Option<Vec<SelectionResult>>,
}

impl RelatedRecordsQuery {
    // HACK: this should disappear
    pub fn db_alias(&self, i: usize, depth: usize) -> String {
        let selected_field = self.selected_fields.inner().get(i).unwrap();

        format!("{}.{}.{}", &self.parent_field.name, selected_field.prisma_name(), depth)
    }

    pub fn type_identifier_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        let mut result = self
            .selected_fields
            .selections()
            .map(|field| match field {
                SelectedField::Scalar(sf) => sf.type_identifier_with_arity(),
                SelectedField::Composite(_) => unreachable!(),
            })
            .collect_vec();

        for read in &self.nested {
            match read {
                ReadQuery::RelatedRecordsQuery(rrq) => {
                    result.extend(rrq.type_identifier_with_arities());
                }
                _ => unreachable!(),
            };
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct AggregateRecordsQuery {
    pub name: String,
    pub alias: Option<String>,
    pub model: ModelRef,
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
