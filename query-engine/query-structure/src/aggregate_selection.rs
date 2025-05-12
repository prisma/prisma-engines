use std::slice;

use itertools::Either;
use psl::schema_ast::ast::FieldArity;

use crate::{ScalarFieldRef, TypeIdentifier};

/// Selections for aggregation queries.
#[derive(Debug, Clone)]
pub enum AggregationSelection {
    /// Single field selector. Only valid in the context of group by statements.
    Field(ScalarFieldRef),

    /// Counts records of the model that match the query.
    /// `all` indicates that an all-records selection has been made (e.g. SQL *).
    /// `fields` are specific fields to count on. By convention, if `all` is true,
    /// it will always be the last of the count results.
    Count { all: bool, fields: Vec<ScalarFieldRef> },

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
    pub fn identifiers(&self) -> impl Iterator<Item = SelectionIdenfitier<'_>> {
        match self {
            AggregationSelection::Field(field) => {
                Either::Left(Self::map_field_types(slice::from_ref(field), |t| t, |a| a))
            }
            AggregationSelection::Sum(fields) => {
                Either::Left(Self::map_field_types(fields, |t| t, |_| FieldArity::Required))
            }
            AggregationSelection::Min(fields) => {
                Either::Left(Self::map_field_types(fields, |t| t, |_| FieldArity::Required))
            }
            AggregationSelection::Max(fields) => {
                Either::Left(Self::map_field_types(fields, |t| t, |_| FieldArity::Required))
            }

            AggregationSelection::Average(fields) => Either::Left(Self::map_field_types(
                fields,
                |t| match t {
                    TypeIdentifier::Decimal => TypeIdentifier::Decimal,
                    _ => TypeIdentifier::Float,
                },
                |_| FieldArity::Required,
            )),

            AggregationSelection::Count { all, fields } => {
                let mapped = Self::map_field_types(fields, |_| TypeIdentifier::Int, |_| FieldArity::Required);
                if *all {
                    Either::Right(mapped.chain([SelectionIdenfitier {
                        name: "all",
                        db_name: "all",
                        typ: TypeIdentifier::Int,
                        arity: FieldArity::Required,
                    }]))
                } else {
                    Either::Left(mapped)
                }
            }
        }
    }

    fn map_field_types(
        fields: &[ScalarFieldRef],
        type_mapper: fn(TypeIdentifier) -> TypeIdentifier,
        arity_mapper: fn(FieldArity) -> FieldArity,
    ) -> impl Iterator<Item = SelectionIdenfitier<'_>> {
        fields.iter().map(move |f| SelectionIdenfitier {
            name: f.name(),
            db_name: f.db_name(),
            typ: type_mapper(f.type_identifier()),
            arity: arity_mapper(f.arity()),
        })
    }
}

pub struct SelectionIdenfitier<'a> {
    pub name: &'a str,
    pub db_name: &'a str,
    pub typ: TypeIdentifier,
    pub arity: FieldArity,
}
