use std::{borrow::Cow, slice};

use itertools::Either;
use psl::schema_ast::ast::FieldArity;

use crate::{InternalDataModelRef, ScalarFieldRef, Type, TypeIdentifier, Zipper};

/// Selections for aggregation queries.
#[derive(Debug, Clone)]
pub enum AggregationSelection {
    /// Single field selector. Only valid in the context of group by statements.
    Field(ScalarFieldRef),

    /// Counts records of the model that match the query.
    /// `all` indicates that an all-records selection has been made (e.g. SQL *).
    /// `fields` are specific fields to count on. By convention, if `all` is set,
    /// it will always be the last of the count results.
    Count {
        all: Option<CountAllAggregationSelection>,
        fields: Vec<ScalarFieldRef>,
    },

    /// Compute average for each field contained.
    Average(Vec<ScalarFieldRef>),

    /// Compute sum for each field contained.
    Sum(Vec<ScalarFieldRef>),

    /// Compute mininum for each field contained.
    Min(Vec<ScalarFieldRef>),

    /// Compute maximum for each field contained.
    Max(Vec<ScalarFieldRef>),
}

impl AggregationSelection {
    /// Returns (field_db_name, TypeIdentifier, FieldArity)
    pub fn identifiers(&self) -> impl Iterator<Item = SelectionIdentifier<'_>> {
        match self {
            AggregationSelection::Field(field) => {
                Either::Left(self.map_field_types(slice::from_ref(field), |t| t, |a| a))
            }
            AggregationSelection::Sum(fields) => {
                Either::Left(self.map_field_types(fields, |t| t, |_| FieldArity::Required))
            }
            AggregationSelection::Min(fields) => {
                Either::Left(self.map_field_types(fields, |t| t, |_| FieldArity::Required))
            }
            AggregationSelection::Max(fields) => {
                Either::Left(self.map_field_types(fields, |t| t, |_| FieldArity::Required))
            }

            AggregationSelection::Average(fields) => Either::Left(self.map_field_types(
                fields,
                |t| match t {
                    TypeIdentifier::Decimal => TypeIdentifier::Decimal,
                    _ => TypeIdentifier::Float,
                },
                |_| FieldArity::Required,
            )),

            AggregationSelection::Count { all, fields } => Either::Right(
                self.map_field_types(fields, |_| TypeIdentifier::Int, |_| FieldArity::Required)
                    .chain(all.iter().map(|all| SelectionIdentifier {
                        name: all.name(),
                        db_alias: self.aggregation_db_alias("_all"),
                        field_db_name: all.db_name(),
                        typ: all.r#type(),
                        arity: all.arity(),
                    })),
            ),
        }
    }

    pub fn aggregation_name(&self) -> Option<&'static str> {
        match self {
            AggregationSelection::Field(_) => None,
            AggregationSelection::Count { .. } => Some("_count"),
            AggregationSelection::Average(_) => Some("_avg"),
            AggregationSelection::Sum(_) => Some("_sum"),
            AggregationSelection::Min(_) => Some("_min"),
            AggregationSelection::Max(_) => Some("_max"),
        }
    }

    fn aggregation_db_alias<'name>(&self, db_name: &'name str) -> Cow<'name, str> {
        const FIELD_SEPARATOR: &str = "$";
        self.aggregation_name().map_or(Cow::Borrowed(db_name), |aggregation| {
            Cow::Owned(format!("{aggregation}{FIELD_SEPARATOR}{db_name}"))
        })
    }

    fn map_field_types<'a>(
        &self,
        fields: &'a [ScalarFieldRef],
        type_mapper: fn(TypeIdentifier) -> TypeIdentifier,
        arity_mapper: fn(FieldArity) -> FieldArity,
    ) -> impl Iterator<Item = SelectionIdentifier<'a>> {
        fields.iter().map(move |f| SelectionIdentifier {
            name: f.name(),
            db_alias: self.aggregation_db_alias(f.db_name()),
            field_db_name: f.db_name(),
            typ: f.dm.clone().zip(type_mapper(f.type_identifier())),
            arity: arity_mapper(f.arity()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CountAllAggregationSelection {
    pub dm: InternalDataModelRef,
}

impl CountAllAggregationSelection {
    pub fn new(dm: InternalDataModelRef) -> Self {
        CountAllAggregationSelection { dm }
    }

    #[inline]
    pub fn name(&self) -> &str {
        "all"
    }

    #[inline]
    pub fn db_name(&self) -> &str {
        "all"
    }

    #[inline]
    pub fn type_identifier(&self) -> TypeIdentifier {
        TypeIdentifier::Int
    }

    #[inline]
    pub fn arity(&self) -> FieldArity {
        FieldArity::Required
    }

    pub fn r#type(&self) -> Type {
        self.dm.clone().zip(self.type_identifier())
    }
}

impl<I> From<&'_ Zipper<I>> for CountAllAggregationSelection {
    fn from(zipper: &Zipper<I>) -> Self {
        CountAllAggregationSelection::new(zipper.dm.clone())
    }
}

pub struct SelectionIdentifier<'a> {
    pub name: &'a str,
    pub db_alias: Cow<'a, str>,
    pub field_db_name: &'a str,
    pub typ: Type,
    pub arity: FieldArity,
}
