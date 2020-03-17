mod aggregate;
mod many;
mod one;
mod related;

pub use aggregate::*;
pub use many::*;
pub use one::*;
pub use related::*;

use super::*;
use crate::{query_document::ParsedField, ReadQuery};
use prisma_models::{
    Field, ModelRef, RelationFieldRef, SelectedField, SelectedFields, SelectedRelationField, SelectedScalarField,
};
use std::sync::Arc;

pub enum ReadQueryBuilder {
    ReadOneRecordBuilder(ReadOneRecordBuilder),
    ReadManyRecordsBuilder(ReadManyRecordsBuilder),
    ReadRelatedRecordsBuilder(ReadRelatedRecordsBuilder),
    AggregateRecordsBuilder(AggregateRecordsBuilder),
}

impl Builder<ReadQuery> for ReadQueryBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        match self {
            ReadQueryBuilder::ReadOneRecordBuilder(b) => b.build(),
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
pub fn collect_selected_fields(from: &[ParsedField], model: &ModelRef) -> SelectedFields {
    let selected_fields = from
        .iter()
        .map(|selected_field| {
            let model_field = model.fields().find_from_all(&selected_field.name).unwrap();
            match model_field {
                Field::Scalar(ref sf) => SelectedField::Scalar(SelectedScalarField { field: Arc::clone(sf) }),
                Field::Relation(ref rf) => SelectedField::Relation(SelectedRelationField { field: Arc::clone(rf) }),
            }
        })
        .collect::<Vec<SelectedField>>();

    let mut selected_fields = SelectedFields::new(selected_fields);

    let model_id = model.primary_identifier();
    for field in model_id {
        selected_fields.add(field);
    }

    selected_fields
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
    mut selected_fields: SelectedFields,
    parent_relation: Option<RelationFieldRef>,
    nested_queries: &[ReadQuery],
) -> SelectedFields {
    // Context: We are on the child model when calling this function.
    if let Some(rf) = parent_relation {
        let field = rf.related_field();
        selected_fields.add_all(field.linking_fields().into_iter());
    }

    for nested in nested_queries {
        if let ReadQuery::RelatedRecordsQuery(ref rq) = nested {
            selected_fields.add_all(rq.parent_field.linking_fields().into_iter());
        }
    }

    selected_fields.deduplicate()
}
