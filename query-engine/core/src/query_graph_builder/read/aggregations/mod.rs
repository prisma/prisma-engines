use super::*;

mod aggregate;
mod group_by;

pub use aggregate::*;
pub use group_by::*;

use crate::FieldPair;
use connector::AggregationSelection;
use prisma_models::{ModelRef, ScalarFieldRef};

/// Resolves the given field as a aggregation query.
fn resolve_query(field: FieldPair, model: &ModelRef) -> QueryGraphBuilderResult<AggregationSelection> {
    let query = match field.parsed_field.name.as_str() {
        "count" => {
            let field = resolve_fields(model, field).pop();
            AggregationSelection::Count(field)
        }
        "avg" => AggregationSelection::Average(resolve_fields(model, field)),
        "sum" => AggregationSelection::Sum(resolve_fields(model, field)),
        "min" => AggregationSelection::Min(resolve_fields(model, field)),
        "max" => AggregationSelection::Max(resolve_fields(model, field)),
        name => AggregationSelection::Field(model.fields().find_from_scalar(name).unwrap()),
    };

    Ok(query)
}

fn resolve_fields(model: &ModelRef, field: FieldPair) -> Vec<ScalarFieldRef> {
    let scalars = model.fields().scalar();
    let fields = match field.parsed_field.nested_fields {
        Some(nested_obj) => nested_obj.fields,
        None => vec![],
    };

    fields
        .into_iter()
        .map(|f| {
            scalars
                .iter()
                .find_map(|sf| {
                    if sf.name == f.parsed_field.name {
                        Some(sf.clone())
                    } else {
                        None
                    }
                })
                .expect("Expected validation to guarantee valid aggregation fields.")
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
                field.nested_fields.as_ref().map(|nested_object| {
                    nested_object
                        .fields
                        .iter()
                        .map(|f| f.parsed_field.name.clone())
                        .collect()
                }),
            )
        })
        .collect()
}
