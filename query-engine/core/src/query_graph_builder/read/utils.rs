use super::*;
use crate::{ArgumentListLookup, FieldPair, ParsedField, ReadQuery};
use connector::RelAggregationSelection;
use prisma_models::prelude::*;
use schema_builder::constants::{aggregations::*, args};
use std::sync::Arc;

pub fn collect_selection_order(from: &[FieldPair]) -> Vec<String> {
    from.iter()
        .map(|pair| {
            pair.parsed_field
                .alias
                .clone()
                .unwrap_or_else(|| pair.parsed_field.name.clone())
        })
        .collect()
}

/// Creates a `FieldSelection` from a query selection.
/// Automatically adds model IDs to the selected fields as well.
/// Unwraps are safe due to query validation.
pub fn collect_selected_fields(
    from_pairs: &[FieldPair],
    distinct: Option<FieldSelection>,
    model: &ModelRef,
) -> FieldSelection {
    let model_id = model.primary_identifier();
    let selected_fields = pairs_to_selections(model, from_pairs);

    let selection = FieldSelection::new(selected_fields);
    let selection = model_id.merge(selection);

    // Distinct fields are always selected because we are processing them in-memory
    if let Some(distinct) = distinct {
        selection.merge(distinct)
    } else {
        selection
    }
}

fn pairs_to_selections<T>(parent: T, pairs: &[FieldPair]) -> Vec<SelectedField>
where
    T: Into<ParentContainer>,
{
    let parent = parent.into();
    let fields = pairs
        .iter()
        .filter_map(|pair| {
            parent
                .find_field(&pair.parsed_field.name)
                .map(|field| (pair.parsed_field.clone(), field))
        })
        .collect::<Vec<(ParsedField, Field)>>();

    fields
        .into_iter()
        .flat_map(|field| match field {
            (_, Field::Relation(rf)) => rf.scalar_fields().into_iter().map(Into::into).collect(),
            (_, Field::Scalar(sf)) => vec![sf.into()],
            (pf, Field::Composite(cf)) => vec![extract_composite_selection(pf, cf)],
        })
        .collect()
}

fn extract_composite_selection(pf: ParsedField, cf: CompositeFieldRef) -> SelectedField {
    let object = pf
        .nested_fields
        .expect("Invalid composite query shape: Composite field selected without sub-selection.");

    let typ = cf.typ.clone();

    SelectedField::Composite(CompositeSelection {
        field: cf,
        selections: pairs_to_selections(&typ, &object.fields),
    })
}

pub fn collect_nested_queries(from: Vec<FieldPair>, model: &ModelRef) -> QueryGraphBuilderResult<Vec<ReadQuery>> {
    from.into_iter()
        .filter_map(|pair| {
            if is_aggr_selection(&pair) {
                return None;
            }

            let model_field = model.fields().find_from_all(&pair.parsed_field.name).unwrap();

            match model_field {
                Field::Scalar(_) => None,
                Field::Composite(_) => None,
                Field::Relation(ref rf) => {
                    let model = rf.related_model();
                    let parent = Arc::clone(&rf);

                    Some(related::find_related(pair.parsed_field, parent, model))
                }
            }
        })
        .collect::<QueryGraphBuilderResult<Vec<ReadQuery>>>()
}

/// Performs a lookahead based on the nested queries and merges fields required
/// to resolve the nested queries.
/// A lookback on the parent is also performed to ensure that fields required for
/// resolving the parent relation are present.
pub fn merge_relation_selections(
    selected_fields: FieldSelection,
    parent_relation: Option<RelationFieldRef>,
    nested_queries: &[ReadQuery],
) -> FieldSelection {
    // Context: We are on the child model when calling this function.
    let selected_fields = if let Some(rf) = parent_relation {
        let field = rf.related_field();
        selected_fields.merge(field.linking_fields())
    } else {
        selected_fields
    };

    let nested: Vec<_> = nested_queries
        .iter()
        .map(|nested_query| {
            if let ReadQuery::RelatedRecordsQuery(ref rq) = nested_query {
                rq.parent_field.linking_fields()
            } else {
                unreachable!()
            }
        })
        .collect();

    selected_fields.merge(FieldSelection::union(nested))
}

/// Ensures that if a cursor is provided, its fields are also selected.
/// Necessary for post-processing of unstable orderings with cursor operations.
pub fn merge_cursor_fields(selected_fields: FieldSelection, cursor: &Option<SelectionResult>) -> FieldSelection {
    match cursor {
        Some(cursor) => selected_fields.merge(cursor.into()),
        None => selected_fields,
    }
}

pub fn collect_relation_aggr_selections(
    from: Vec<FieldPair>,
    model: &ModelRef,
) -> QueryGraphBuilderResult<Vec<RelAggregationSelection>> {
    let mut selections = vec![];

    for pair in from {
        match pair.parsed_field.name.as_str() {
            UNDERSCORE_COUNT => {
                let nested_fields = pair.parsed_field.nested_fields.unwrap();

                for mut nested_pair in nested_fields.fields {
                    let rf = model
                        .fields()
                        .find_from_relation_fields(&nested_pair.parsed_field.name)
                        .unwrap();
                    let filter = match nested_pair.parsed_field.arguments.lookup(args::WHERE) {
                        Some(where_arg) => Some(extract_filter(where_arg.value.try_into()?, rf.related_model())?),
                        _ => None,
                    };

                    selections.push(RelAggregationSelection::Count(rf, filter));
                }
            }
            field_name => panic!("Unknown field name \"{}\" for a relation aggregation", field_name),
        }
    }

    Ok(selections)
}
