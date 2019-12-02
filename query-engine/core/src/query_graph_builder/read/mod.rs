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
use prisma_models::{Field, ModelRef, RelationFieldRef, SelectedField, SelectedFields};
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

pub fn collect_selected_fields(
    parsed_fields: &[ParsedField],
    model: &ModelRef,
    parent: Option<RelationFieldRef>,
) -> SelectedFields {
    let mut selected_fields = Vec::with_capacity(parsed_fields.len());

    for selected_field in parsed_fields {
        let model_field = model.fields().find_from_all(&selected_field.name).unwrap();

        match model_field {
            Field::Scalar(ref sf) => selected_fields.push(SelectedField::from(Arc::clone(sf))),
            Field::Relation(ref rf) => selected_fields.push(SelectedField::from(Arc::clone(rf))),
        }
    }

    if let Some(ref rf) = parent {
        let relation = rf.relation();

        if rf.relation
    };

    SelectedFields::new(selected_fields)
}

pub fn collect_nested_queries(fields: Vec<ParsedField>, model: &ModelRef) -> QueryGraphBuilderResult<Vec<ReadQuery>> {
    let mut queries = Vec::with_capacity(fields.len());

    for mut field in fields {
        let model_field = model.fields().find_from_all(&field.name).unwrap();

        if let Field::Relation(ref rf) = model_field {
            let model = rf.related_model();
            let parent = Arc::clone(&rf);
            let args = utils::extract_query_args(field.arguments.drain(0..).collect(), &model)?;

            let builder = if rf.relation().is_many_to_many() || args.is_with_pagination() {
                ReadQueryBuilder::ReadRelatedRecordsBuilder(ReadRelatedRecordsBuilder::new(model, parent, field, args))
            } else if rf.relation_is_inlined_in_parent() {
                ReadQueryBuilder::ReadManyRecordsBuilder(ReadManyRecordsBuilder::new(field, model, args, Some(parent)))
            } else {
                // in child
                unimplemented!()
            };

            queries.push(builder.build()?);
        }
    }

    Ok(queries)
}
