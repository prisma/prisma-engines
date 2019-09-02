use super::*;
use crate::query_document::{ParsedField, ArgumentListLookup};
use connector::read_ast::{ReadQuery, RecordQuery};
use prisma_models::ModelRef;

pub struct ReadOneRecordBuilder {
    field: ParsedField,
    model: ModelRef,
}

impl ReadOneRecordBuilder {
    pub fn new(field: ParsedField, model: ModelRef) -> Self {
        Self { field, model }
    }
}

impl Builder<ReadQuery> for ReadOneRecordBuilder {
    /// Builds a read query tree from a parsed top-level field of a query
    /// Unwraps are safe because of query validation that ensures conformity to the query schema.
    fn build(mut self) -> QueryBuilderResult<ReadQuery> {
        let record_finder = match self.field.arguments.lookup("where") {
            Some(where_arg) => Some(utils::extract_record_finder(where_arg.value, &self.model)?),
            None => None,
        };

        let name = self.field.name;
        let alias = self.field.alias;
        let nested_fields = self.field.nested_fields.unwrap().fields;
        let selection_order: Vec<String> = collect_selection_order(&nested_fields);
        let selected_fields = collect_selected_fields(&nested_fields, &self.model, None);
        let nested = collect_nested_queries(nested_fields, &self.model)?;

        Ok(ReadQuery::RecordQuery(RecordQuery {
            name,
            alias,
            record_finder,
            selected_fields,
            nested,
            selection_order,
        }))
    }
}
