mod aggregate;
mod group_by;

pub(crate) use aggregate::*;
pub(crate) use group_by::*;

use super::*;
use crate::{FieldPair, ParsedObject};
use query_structure::{AggregationSelection, Model, ScalarFieldRef};
use schema::constants::aggregations::*;

fn collect_selection_tree_and_selectors(
    fields: Vec<FieldPair<'_>>,
    model: &Model,
    allow_deprecated: bool,
) -> QueryGraphBuilderResult<(Vec<(String, Option<Vec<String>>)>, Vec<AggregationSelection>)> {
    let mut selection_order = Vec::with_capacity(fields.len());
    let mut selectors = Vec::with_capacity(fields.len());

    for field in fields {
        let (selection, selector) = resolve_query_and_selection(field, model, allow_deprecated)?;
        selection_order.push(selection);
        selectors.push(selector);
    }

    Ok((selection_order, selectors))
}

/// Resolves the given field as an aggregation query.
fn resolve_query_and_selection(
    field: FieldPair<'_>,
    model: &Model,
    allow_deprecated: bool,
) -> QueryGraphBuilderResult<((String, Option<Vec<String>>), AggregationSelection)> {
    let name = field.parsed_field.name;
    let nested_fields = field.parsed_field.nested_fields;

    let (nested_selection, query) = match name.as_str() {
        COUNT if allow_deprecated => {
            let (nested_selection, has_count_all, fields) = resolve_fields(model, nested_fields);
            (
                nested_selection,
                AggregationSelection::Count {
                    all: has_count_all.then(|| model.into()),
                    fields,
                },
            )
        }
        AVG if allow_deprecated => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Average(fields))
        }
        SUM if allow_deprecated => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Sum(fields))
        }
        MIN if allow_deprecated => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Min(fields))
        }
        MAX if allow_deprecated => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Max(fields))
        }

        UNDERSCORE_COUNT => {
            let (nested_selection, has_count_all, fields) = resolve_fields(model, nested_fields);
            (
                nested_selection,
                AggregationSelection::Count {
                    all: has_count_all.then(|| model.into()),
                    fields,
                },
            )
        }
        UNDERSCORE_AVG => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Average(fields))
        }
        UNDERSCORE_SUM => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Sum(fields))
        }
        UNDERSCORE_MIN => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Min(fields))
        }
        UNDERSCORE_MAX => {
            let (nested_selection, _, fields) = resolve_fields(model, nested_fields);
            (nested_selection, AggregationSelection::Max(fields))
        }

        name => (
            None,
            AggregationSelection::Field(model.fields().find_from_scalar(name).unwrap()),
        ),
    };

    Ok(((name, nested_selection), query))
}

fn resolve_fields(
    model: &Model,
    nested_fields: Option<ParsedObject<'_>>,
) -> (Option<Vec<String>>, bool, Vec<ScalarFieldRef>) {
    let scalars = model.fields().scalar();
    let fields = nested_fields
        .expect("Expected at least one selection for aggregate")
        .fields;
    let mut selection_order = Vec::with_capacity(fields.len());
    let mut has_count_all = false;
    let mut selected_fields = Vec::with_capacity(fields.len());

    for field in fields {
        let name = field.parsed_field.name;

        if name == "_all" {
            has_count_all = true;
            selection_order.push(name);
            continue;
        }

        let scalar = scalars
            .clone()
            .find_map(|sf| if sf.name() == name.as_str() { Some(sf) } else { None });

        selection_order.push(name);

        if let Some(scalar) = scalar {
            selected_fields.push(scalar);
        }
    }

    let selection_order = if selection_order.is_empty() {
        None
    } else {
        Some(selection_order)
    };

    (selection_order, has_count_all, selected_fields)
}
