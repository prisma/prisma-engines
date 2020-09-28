mod aggregate;
mod first;
mod many;
mod one;
mod related;

pub use aggregate::*;
pub use first::*;
pub use many::*;
pub use one::*;
pub use related::*;

use super::*;
use crate::{query_document::ParsedField, ReadQuery};
use prisma_models::{Field, ModelProjection, ModelRef, RecordProjection, RelationFieldRef};
use std::sync::Arc;

pub enum ReadQueryBuilder {
    ReadOneRecordBuilder(ReadOneRecordBuilder),
    ReadFirstRecordBuilder(ReadFirstRecordBuilder),
    ReadManyRecordsBuilder(ReadManyRecordsBuilder),
    ReadRelatedRecordsBuilder(ReadRelatedRecordsBuilder),
    AggregateRecordsBuilder(AggregateRecordsBuilder),
}

impl Builder<ReadQuery> for ReadQueryBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        match self {
            ReadQueryBuilder::ReadOneRecordBuilder(b) => b.build(),
            ReadQueryBuilder::ReadFirstRecordBuilder(b) => b.build(),
            ReadQueryBuilder::ReadManyRecordsBuilder(b) => b.build(),
            ReadQueryBuilder::ReadRelatedRecordsBuilder(b) => b.build(),
            ReadQueryBuilder::AggregateRecordsBuilder(b) => b.build(),
        }
    }
}

pub fn collect_selection_order(from: &[ParsedField]) -> Vec<String> {
    from.iter()
        .map(|selected_field| {
            selected_field
                .alias
                .clone()
                .unwrap_or_else(|| selected_field.name.clone())
        })
        .collect()
}

/// Creates SelectedFields from a query selection.
/// Automatically adds model IDs to the selected fields as well.
/// Unwraps are safe due to query validation.
pub fn collect_selected_fields(from: &[ParsedField], model: &ModelRef) -> ModelProjection {
    let selected_fields = from
        .iter()
        .filter_map(|selected_field| {
            model
                .fields()
                .find_from_scalar(&selected_field.name)
                .ok()
                .map(|sf| sf.into())
        })
        .collect::<Vec<Field>>();

    let selected_projection = ModelProjection::new(selected_fields);
    let model_id = model.primary_identifier();

    model_id.merge(selected_projection)
}

pub fn collect_nested_queries(from: Vec<ParsedField>, model: &ModelRef) -> QueryGraphBuilderResult<Vec<ReadQuery>> {
    from.into_iter()
        .filter_map(|selected_field| {
            let model_field = model.fields().find_from_all(&selected_field.name).unwrap();
            match model_field {
                Field::Scalar(_) => None,
                Field::Relation(ref rf) => {
                    let model = rf.related_model();
                    let parent = Arc::clone(&rf);

                    Some(ReadQueryBuilder::ReadRelatedRecordsBuilder(
                        ReadRelatedRecordsBuilder::new(model, parent, selected_field),
                    ))
                }
            }
        })
        .collect::<Vec<ReadQueryBuilder>>()
        .into_iter()
        .map(|builder| builder.build())
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
        .into_iter()
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
        Some(cursor) => selected_fields.clone().merge(cursor.into()),
        None => selected_fields,
    }
}
