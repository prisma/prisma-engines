use super::*;
use crate::{query_document::*, query_graph::*, schema::*, ResultInfo};

pub struct QueryBuilder {
    pub query_schema: QuerySchemaRef,
}

impl QueryBuilder {
    pub fn new(query_schema: QuerySchemaRef) -> Self {
        QueryBuilder { query_schema }
    }

    pub fn build(self, query_doc: QueryDocument) -> QueryBuilderResult<Vec<(QueryGraph, ResultInfo)>> {
        query_doc
            .operations
            .into_iter()
            .map(|op| self.map_operation(op))
            .collect::<QueryBuilderResult<Vec<(QueryGraph, ResultInfo)>>>()
            .map_err(|err| err.into())
    }

    /// Maps an operation to a query.
    fn map_operation(&self, operation: Operation) -> QueryBuilderResult<(QueryGraph, ResultInfo)> {
        match operation {
            Operation::Read(selection) => self.map_read_operation(selection),
            Operation::Write(selection) => self.map_write_operation(selection),
        }
    }

    /// Maps a read operation to one or more queries.
    fn map_read_operation(&self, read_selection: Selection) -> QueryBuilderResult<(QueryGraph, ResultInfo)> {
        let query_object = self.query_schema.query();
        Self::process(&read_selection, &query_object)
    }

    /// Maps a write operation to one or more queries.
    fn map_write_operation(&self, write_selection: Selection) -> QueryBuilderResult<(QueryGraph, ResultInfo)> {
        let mutation_object = self.query_schema.query();
        Self::process(&write_selection, &mutation_object)
    }

    fn process(selection: &Selection, object: &ObjectTypeStrongRef) -> QueryBuilderResult<(QueryGraph, ResultInfo)> {
        let parsed_field = QueryDocumentParser::parse_field(selection, object)?;
        let result_info = Self::derive_result_info(selection, &parsed_field);

        let query_graph = match &parsed_field.schema_field.clone().query_builder {
            Some(builder) => builder.build(parsed_field),
            None => Err(QueryValidationError::AssertionError(format!(
                "Expected attached query builder on {} object, root level field '{}'.",
                object.name, parsed_field.name
            ))),
        }?;

        Ok((query_graph, result_info))
    }

    // TODO: Issue - the selected fields only reflect the first layer, which is ok, because read results, the only results
    // that have nesting at the moment, have all info encoded in their result. This needs to change when unifying the
    // result types: Push the result state into a structure that can reflect all nesting levels.
    // Tl;dr: Selected_fields are only interesting for write query results at the moment and that's okay.
    fn derive_result_info(selection: &Selection, field: &ParsedField) -> ResultInfo {
        ResultInfo {
            key: selection.alias.clone().unwrap_or_else(|| selection.name.clone()),
            output_type: field.schema_field.field_type.clone(),
            selected_fields: selection.nested_selections.iter().map(|s| s.name.clone()).collect(),
        }
    }
}
