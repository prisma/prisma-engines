use super::*;
use crate::{query_document::*, query_graph::*, schema::*, IrSerializer};
use prisma_value::PrismaValue;

// TODO: Think about if this is really necessary here, or if the whole code should move into
// the query_document module, possibly already as part of the parser.
pub struct QueryGraphBuilder {
    pub query_schema: QuerySchemaRef,
}

pub enum QueryType {
    Graph(QueryGraph),
    Raw {
        query: String,
        parameters: Vec<PrismaValue>,
        raw_type: RawQueryType,
    },
}

impl QueryType {
    pub fn needs_transaction(&self) -> bool {
        match self {
            Self::Graph(qg) => qg.needs_transaction(),
            Self::Raw { .. } => false,
        }
    }
}

#[derive(Default)]
struct RawArgs {
    query: String,
    parameters: Vec<PrismaValue>,
}

impl RawArgs {
    fn add_arg(&mut self, arg: Option<ParsedArgument>) {
        if let Some(arg) = arg {
            if arg.name == "query" {
                self.query = arg.into_value().unwrap().into_string().unwrap();
            } else {
                self.parameters = arg.into_value().unwrap().into_list().unwrap();
            }
        }
    }
}

impl From<Vec<ParsedArgument>> for RawArgs {
    fn from(mut args: Vec<ParsedArgument>) -> Self {
        let mut ra = Self::default();

        ra.add_arg(args.pop());
        ra.add_arg(args.pop());

        ra
    }
}

impl QueryGraphBuilder {
    pub fn new(query_schema: QuerySchemaRef) -> Self {
        Self { query_schema }
    }

    /// Maps an operation to a query.
    pub fn build(self, operation: Operation) -> QueryGraphBuilderResult<(QueryType, IrSerializer)> {
        match operation {
            Operation::Read(selection) => self.map_read_operation(selection),
            Operation::Write(selection) => self.map_write_operation(selection),
        }
    }

    /// Maps a read operation to one or more queries.
    fn map_read_operation(&self, read_selection: Selection) -> QueryGraphBuilderResult<(QueryType, IrSerializer)> {
        let query_object = self.query_schema.query();
        Self::process(read_selection, &query_object)
    }

    /// Maps a write operation to one or more queries.
    fn map_write_operation(&self, write_selection: Selection) -> QueryGraphBuilderResult<(QueryType, IrSerializer)> {
        let mutation_object = self.query_schema.mutation();

        let (mut graph, ir_ser) = Self::process(write_selection, &mutation_object)?;

        if let QueryType::Graph(ref mut graph) = graph {
            graph.flag_transactional();
        };

        Ok((graph, ir_ser))
    }

    fn process(
        selection: Selection,
        object: &ObjectTypeStrongRef,
    ) -> QueryGraphBuilderResult<(QueryType, IrSerializer)> {
        let mut selections = vec![selection];
        let mut parsed_object = QueryDocumentParser::parse_object(&selections, object)?;

        let parsed_field = parsed_object.fields.pop().unwrap();
        let result_info = Self::derive_serializer(&selections.pop().unwrap(), &parsed_field);

        let schema_field = parsed_field.schema_field.clone();
        let builder = schema_field.query_builder();

        let query_type = match (builder, parsed_field.raw_query_type()) {
            (Some(builder), None) => Ok(QueryType::Graph(builder.build(parsed_field)?)),
            (_, Some(raw_type)) => {
                let raw_args = RawArgs::from(parsed_field.arguments);

                Ok(QueryType::Raw {
                    query: raw_args.query,
                    parameters: raw_args.parameters,
                    raw_type,
                })
            }
            (None, None) => Err(QueryGraphBuilderError::SchemaError(format!(
                "Expected attached query builder on {} object, root level field '{}'.",
                object.name(),
                parsed_field.name
            ))),
        }?;

        Ok((query_type, result_info))
    }

    fn derive_serializer(selection: &Selection, field: &ParsedField) -> IrSerializer {
        IrSerializer {
            key: selection
                .alias()
                .clone()
                .unwrap_or_else(|| selection.name().to_string()),
            output_type: field.schema_field.field_type.clone(),
        }
    }
}
