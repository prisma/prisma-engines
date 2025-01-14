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
    pub fn identifiers(&self) -> Vec<(String, TypeIdentifier, FieldArity)> {
        match self {
            AggregationSelection::Field(field) => {
                vec![(field.db_name().to_owned(), field.type_identifier(), field.arity())]
            }

            AggregationSelection::Count { all, fields } => {
                let mut mapped = Self::map_field_types(fields, Some(TypeIdentifier::Int));

                if *all {
                    mapped.push(("all".to_owned(), TypeIdentifier::Int, FieldArity::Required));
                }

                mapped
            }

            AggregationSelection::Average(fields) => Self::map_field_types(fields, Some(TypeIdentifier::Float)),
            AggregationSelection::Sum(fields) => Self::map_field_types(fields, None),
            AggregationSelection::Min(fields) => Self::map_field_types(fields, None),
            AggregationSelection::Max(fields) => Self::map_field_types(fields, None),
        }
    }

    fn map_field_types(
        fields: &[ScalarFieldRef],
        fixed_type: Option<TypeIdentifier>,
    ) -> Vec<(String, TypeIdentifier, FieldArity)> {
        fields
            .iter()
            .map(|f| {
                (
                    f.db_name().to_owned(),
                    fixed_type.unwrap_or_else(|| f.type_identifier()),
                    FieldArity::Required,
                )
            })
            .collect()
    }
}
