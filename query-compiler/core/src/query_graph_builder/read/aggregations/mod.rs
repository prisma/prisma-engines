mod aggregate;
mod group_by;

pub(crate) use aggregate::*;
pub(crate) use group_by::*;

use super::*;
use crate::FieldPair;
use itertools::Itertools;
use query_structure::{AggregationSelection, Model, ScalarFieldRef};
use schema::constants::aggregations::*;

/// Resolves the given field as a aggregation query.
fn resolve_query(
    field: FieldPair<'_>,
    model: &Model,
    allow_deprecated: bool,
) -> QueryGraphBuilderResult<AggregationSelection> {
    let count_resolver = |mut field: FieldPair<'_>, model: &Model| {
        let nested_fields = field
            .parsed_field
            .nested_fields
            .as_mut()
            .expect("Expected at least one selection for aggregate");

        let all_position = nested_fields
            .fields
            .iter()
            .find_position(|f| f.parsed_field.name == "_all");

        match all_position {
            Some((pos, _)) => {
                nested_fields.fields.remove(pos);

                AggregationSelection::Count {
                    all: Some(model.into()),
                    fields: resolve_fields(model, field),
                }
            }
            None => AggregationSelection::Count {
                all: None,
                fields: resolve_fields(model, field),
            },
        }
    };

    let query = match field.parsed_field.name.as_str() {
        COUNT if allow_deprecated => count_resolver(field, model),
        AVG if allow_deprecated => AggregationSelection::Average(resolve_fields(model, field)),
        SUM if allow_deprecated => AggregationSelection::Sum(resolve_fields(model, field)),
        MIN if allow_deprecated => AggregationSelection::Min(resolve_fields(model, field)),
        MAX if allow_deprecated => AggregationSelection::Max(resolve_fields(model, field)),

        UNDERSCORE_COUNT => count_resolver(field, model),
        UNDERSCORE_AVG => AggregationSelection::Average(resolve_fields(model, field)),
        UNDERSCORE_SUM => AggregationSelection::Sum(resolve_fields(model, field)),
        UNDERSCORE_MIN => AggregationSelection::Min(resolve_fields(model, field)),
        UNDERSCORE_MAX => AggregationSelection::Max(resolve_fields(model, field)),

        name => AggregationSelection::Field(model.fields().find_from_scalar(name).unwrap()),
    };

    Ok(query)
}

fn resolve_fields(model: &Model, field: FieldPair<'_>) -> Vec<ScalarFieldRef> {
    let scalars = model.fields().scalar();
    let fields = field
        .parsed_field
        .nested_fields
        .expect("Expected at least one selection for aggregate")
        .fields;

    fields
        .into_iter()
        .filter_map(|f| {
            if f.parsed_field.name == "_all" {
                None
            } else {
                scalars.clone().find_map(|sf| {
                    if sf.name() == f.parsed_field.name {
                        Some(sf)
                    } else {
                        None
                    }
                })
            }
        })
        .collect()
}

fn collect_selection_tree(fields: &[FieldPair<'_>]) -> Vec<(String, Option<Vec<String>>)> {
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

                    if nested.is_empty() { None } else { Some(nested) }
                }),
            )
        })
        .collect()
}
