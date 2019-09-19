use super::*;
use crate::{query_document::*, query_graph::*, schema::*, IrSerializer};

// TODO: Think about if this is really necessary here, or if the whole code should move into
// the query_document module, possibly already as part of the parser.
pub struct QueryBuilder {
    pub query_schema: QuerySchemaRef,
}

impl QueryBuilder {
    pub fn new(query_schema: QuerySchemaRef) -> Self {
        QueryBuilder { query_schema }
    }

    pub fn build(self, query_doc: QueryDocument) -> QueryBuilderResult<Vec<(QueryGraph, IrSerializer)>> {
        query_doc
            .operations
            .into_iter()
            .map(|op| self.map_operation(op))
            .collect::<QueryBuilderResult<Vec<(QueryGraph, IrSerializer)>>>()
            .map_err(|err| err.into())
    }

    /// Maps an operation to a query.
    fn map_operation(&self, operation: Operation) -> QueryBuilderResult<(QueryGraph, IrSerializer)> {
        match operation {
            Operation::Read(selection) => self.map_read_operation(selection),
            Operation::Write(selection) => self.map_write_operation(selection),
        }
    }

    /// Maps a read operation to one or more queries.
    fn map_read_operation(&self, read_selection: Selection) -> QueryBuilderResult<(QueryGraph, IrSerializer)> {
        let query_object = self.query_schema.query();
        Self::process(read_selection, &query_object)
    }

    /// Maps a write operation to one or more queries.
    fn map_write_operation(&self, write_selection: Selection) -> QueryBuilderResult<(QueryGraph, IrSerializer)> {
        let mutation_object = self.query_schema.mutation();
        Self::process(write_selection, &mutation_object)
    }

    fn process(selection: Selection, object: &ObjectTypeStrongRef) -> QueryBuilderResult<(QueryGraph, IrSerializer)> {
        let mut selections = vec![selection];
        let mut parsed_object = QueryDocumentParser::parse_object(&selections, object)?;
        let parsed_field = parsed_object.fields.pop().unwrap();
        let result_info = Self::derive_serializer(&selections.pop().unwrap(), &parsed_field);

        let query_graph = match &parsed_field.schema_field.clone().query_builder {
            Some(builder) => builder.build(parsed_field),
            None => Err(QueryValidationError::AssertionError(format!(
                "Expected attached query builder on {} object, root level field '{}'.",
                object.name, parsed_field.name
            ))),
        }?;

        Ok((query_graph, result_info))
    }

    fn derive_serializer(selection: &Selection, field: &ParsedField) -> IrSerializer {
        IrSerializer {
            key: selection.alias.clone().unwrap_or_else(|| selection.name.clone()),
            output_type: field.schema_field.field_type.clone(),
        }
    }
}
