use super::*;
use crate::{constants::aggregations::*, FieldPair, ReadQuery};
use connector::RelAggregationSelection;
use prisma_models::{Field, ModelProjection, ModelRef, RecordProjection, RelationFieldRef};
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

/// Creates SelectedFields from a query selection.
/// Automatically adds model IDs to the selected fields as well.
/// Unwraps are safe due to query validation.
pub fn collect_selected_fields(
    from: &[FieldPair],
    distinct: Option<ModelProjection>,
    model: &ModelRef,
) -> ModelProjection {
    let selected_fields = from
        .iter()
        .filter_map(|pair| {
            model
                .fields()
                .find_from_scalar(&pair.parsed_field.name)
                .ok()
                .map(|sf| sf.into())
        })
        .collect::<Vec<Field>>();

    let selected_projection = ModelProjection::new(selected_fields);
    let model_id = model.primary_identifier();
    let selected_fields = model_id.merge(selected_projection);

    // Distinct fields are always selected because we are processing them in-memory
    if let Some(distinct) = distinct {
        selected_fields.merge(distinct)
    } else {
        selected_fields
    }
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
    selected_fields: ModelProjection,
    parent_relation: Option<RelationFieldRef>,
    nested_queries: &[ReadQuery],
) -> ModelProjection {
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

    selected_fields.merge(ModelProjection::union(nested))
}

/// Ensures that if a cursor is provided, its fields are also selected.
/// Necessary for post-processing of unstable orderings with cursor operations.
pub fn merge_cursor_fields(selected_fields: ModelProjection, cursor: &Option<RecordProjection>) -> ModelProjection {
    match cursor {
        Some(cursor) => selected_fields.merge(cursor.into()),
        None => selected_fields,
    }
}

pub fn collect_relation_aggr_selections(from: &[FieldPair], model: &ModelRef) -> Vec<RelAggregationSelection> {
    from.iter()
        .flat_map(|pair| match pair.parsed_field.name.as_str() {
            UNDERSCORE_COUNT => {
                let nested_fields = pair.parsed_field.nested_fields.as_ref().unwrap();

                nested_fields
                    .fields
                    .iter()
                    .map(|nested_pair| {
                        let rf = model
                            .fields()
                            .find_from_relation_fields(&nested_pair.parsed_field.name)
                            .unwrap();

                        RelAggregationSelection::Count(rf)
                    })
                    .collect::<Vec<_>>()
            }
            field_name => panic!("Unknown field name \"{}\" for a relation aggregation", field_name),
        })
        .collect::<Vec<_>>()
}
