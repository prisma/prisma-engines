use super::*;
use crate::{query_document::*, query_graph::*, schema::*, IrSerializer};

pub struct QueryGraphBuilder {
    pub query_schema: QuerySchemaRef,
}

impl QueryGraphBuilder {
    pub fn new(query_schema: QuerySchemaRef) -> Self {
        Self { query_schema }
    }

    /// Maps an operation to a query.
    pub fn build(self, operation: Operation) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
        match operation {
            Operation::Read(selection) => self.build_internal(selection, &self.query_schema.query()),
            Operation::Write(selection) => self.build_internal(selection, &self.query_schema.mutation()),
        }
    }

    // /// Maps a read operation to one or more queries.
    // fn map_read_operation(&self, read_selection: Selection) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
    //     let query_object = self.query_schema.query();
    //     Self::process(read_selection, &query_object)
    // }

    // /// Maps a write operation to one or more queries.
    // fn map_write_operation(&self, write_selection: Selection) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
    //     let mutation_object = self.query_schema.mutation();
    //     let (mut graph, ir_ser) = Self::process(write_selection, &mutation_object)?;

    //     // [DTODO] This needs to  move... and do the right thing
    //     // if let QueryGraph::Graph(ref mut graph) = graph {
    //     //     graph.flag_transactional();
    //     // };

    //     Ok((graph, ir_ser))
    // }

    fn build_internal(
        &self,
        selection: Selection,
        object: &ObjectTypeStrongRef,
    ) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
        // [DTODO] The parsing should not require wrapping.
        let mut selections = vec![selection];
        let mut parsed_object = QueryDocumentParser::parse_object(&selections, object)?;

        let field_pair = parsed_object.fields.pop().unwrap();
        let result_info = Self::derive_serializer(&selections.pop().unwrap(), &field_pair.schema_field);

        if field_pair.schema_field.query_info.is_some() {
            let graph = self.dispatch_build(field_pair)?;
            Ok((graph, result_info))
        } else {
            Err(QueryGraphBuilderError::SchemaError(format!(
                "Expected query information to be attached on schema object '{}', field '{}'.",
                object.name(),
                field_pair.parsed_field.name
            )))
        }
    }

    fn derive_serializer(selection: &Selection, field: &FieldRef) -> IrSerializer {
        IrSerializer {
            key: selection
                .alias()
                .clone()
                .unwrap_or_else(|| selection.name().to_string()),
            output_type: field.field_type.clone(),
        }
    }

    fn dispatch_build(&self, field_pair: FieldPair) -> QueryGraphBuilderResult<QueryGraph> {
        let mut graph = QueryGraph::new();
        let query_info = field_pair.schema_field.query_info.as_ref().unwrap();

        match (&query_info.tag, &query_info.model) {
            (QueryTag::FindOne, Some(model)) => todo!(),
            (QueryTag::FindMany, Some(model)) => todo!(),
            (QueryTag::Aggregate, Some(model)) => todo!(),
            (QueryTag::CreateOne, Some(model)) => todo!(),
            (QueryTag::UpdateOne, Some(model)) => todo!(),
            (QueryTag::UpdateMany, Some(model)) => todo!(),
            (QueryTag::UpsertOne, Some(model)) => todo!(),
            (QueryTag::DeleteOne, Some(model)) => todo!(),
            (QueryTag::DeleteMany, Some(model)) => todo!(),
            (QueryTag::Raw, _) => write::raw_query(&mut graph, field_pair.parsed_field),
            (tag, model_opt) => Err(QueryGraphBuilderError::SchemaError(format!(
                "Query tag '{}' and model '{:?}' combination invalid.",
                tag,
                model_opt.as_ref().map(|model| model.name.as_str())
            ))),
        }?;

        // Run final transformations.
        graph.finalize()?;
        trace!("{}", graph);

        Ok(graph)
    }
}
