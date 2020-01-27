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

/// Merges inlined fields into the selected fields of the query as required.
/// The reason is that if a query is part of a nested query tree, it needs
/// to ensure that it fetches all necessary fields of an inlined relation
/// for dependent queries to succeed.
///
/// # Parameters:
/// `selected_fields`: The selected fields as a base for injection.
/// `parent_relation`: If the query is part of a nested tree (only really the
/// case for `RelatedRecordsQuery` right now), we need to check the parent relation field
/// requirements.
/// `nested_queries`: Used for injecting all inlined fields that are required to satisfy
/// dependent, nested queries.
pub fn merge_inlined_relation_fields(
    mut selected_fields: SelectedFields,
    parent_relation: Option<RelationFieldRef>,
    nested_queries: &[ReadQuery],
) -> SelectedFields {
    parent_relation.map(|rf| {
        let field = rf.related_field();

        if field.is_inlined_in_enclosing_model() {
            selected_fields.add_relation(field);
        }
    });

    for nested in nested_queries {
        if let ReadQuery::RelatedRecordsQuery(ref rq) = nested {
            if rq.parent_field.is_inlined_in_enclosing_model() {
                selected_fields.add_relation(rq.parent_field.clone());
            }
        }
    }

    selected_fields.deduplicate()
}
