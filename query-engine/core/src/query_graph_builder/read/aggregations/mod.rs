use super::*;

mod aggregate;
mod group_by;

pub use aggregate::*;
pub use group_by::*;

use crate::{constants::outputs::fields, FieldPair};
use connector::AggregationSelection;
use itertools::Itertools;
use prisma_models::{ModelRef, ScalarFieldRef};

/// Resolves the given field as a aggregation query.
#[allow(clippy::unnecessary_wraps)]
#[tracing::instrument(skip(field, model))]
fn resolve_query(mut field: FieldPair, model: &ModelRef) -> QueryGraphBuilderResult<AggregationSelection> {
    let query = match field.parsed_field.name.as_str() {
        fields::COUNT => {
            let nested_fields = field
                .parsed_field
                .nested_fields
                .as_mut()
                .expect("Expected at least one selection for aggregate");

            let all_position = nested_fields
                .fields
                .iter()
                .find_position(|f| f.parsed_field.name == "_all");

            match all_position {
                Some((pos, _)) => {
                    nested_fields.fields.remove(pos);

                    AggregationSelection::Count {
                        all: true,
                        fields: resolve_fields(model, field),
                    }
                }
                None => AggregationSelection::Count {
                    all: false,
                    fields: resolve_fields(model, field),
                },
            }
        }
        fields::AVG => AggregationSelection::Average(resolve_fields(model, field)),
        fields::SUM => AggregationSelection::Sum(resolve_fields(model, field)),
        fields::MIN => AggregationSelection::Min(resolve_fields(model, field)),
        fields::MAX => AggregationSelection::Max(resolve_fields(model, field)),
        name => AggregationSelection::Field(model.fields().find_from_scalar(name).unwrap()),
    };

    Ok(query)
}

fn resolve_fields(model: &ModelRef, field: FieldPair) -> Vec<ScalarFieldRef> {
    let scalars = model.fields().scalar();
    let fields = field
        .parsed_field
        .nested_fields
        .expect("Expected at least one selection for aggregate")
        .fields;

    fields
        .into_iter()
        .filter_map(|f| {
            if f.parsed_field.name == "_all" {
                None
            } else {
                scalars.iter().find_map(|sf| {
                    if sf.name == f.parsed_field.name {
                        Some(sf.clone())
                    } else {
                        None
                    }
                })
            }
        })
        .collect()
}

fn collect_selection_tree(fields: &[FieldPair]) -> Vec<(String, Option<Vec<String>>)> {
    fields
        .iter()
        .map(|field| {
            let field = &field.parsed_field;
            (
                field.name.clone(),
                field.nested_fields.as_ref().and_then(|nested_object| {
                    let nested: Vec<_> = nested_object
                        .fields
                        .iter()
                        .map(|f| f.parsed_field.name.clone())
                        .collect();

                    if nested.is_empty() {
                        None
                    } else {
                        Some(nested)
                    }
                }),
            )
        })
        .collect()
}
