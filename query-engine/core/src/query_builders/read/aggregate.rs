use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ReadQuery};
use prisma_models::ModelRef;

pub struct AggregateRecordsBuilder {
    field: ParsedField,
    model: ModelRef,
}

impl AggregateRecordsBuilder {
    pub fn new(field: ParsedField, model: ModelRef) -> Self {
        Self { field, model }
    }
}

impl Builder<ReadQuery> for AggregateRecordsBuilder {
    fn build(self) -> QueryBuilderResult<ReadQuery> {
        let name = self.field.name;
        let alias = self.field.alias;
        let model = self.model;

        Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
            name,
            alias,
            model,
        }))
    }
}
