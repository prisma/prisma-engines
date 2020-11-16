use super::*;
use crate::{query_document::*, query_graph::*, schema::*, IrSerializer};
use prisma_value::PrismaValue;

// TODO: Think about if this is really necessary here, or if the whole code should move into
// the query_document module, possibly already as part of the parser.
pub struct QueryGraphBuilder {
    pub query_schema: QuerySchemaRef,
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
    pub fn build(self, operation: Operation) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
        match operation {
            Operation::Read(selection) => self.build_internal(selection, &self.query_schema.query()),
            Operation::Write(selection) => self.build_internal(selection, &self.query_schema.mutation()),
        }
    }

    fn build_internal(
        &self,
        selection: Selection,
        root_object: &ObjectTypeStrongRef, // Either the query or mutation object.
    ) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer)> {
        let mut selections = vec![selection];
        let mut parsed_object = QueryDocumentParser::parse_object(QueryPath::default(), &selections, root_object)?;

        // Because we're processing root objects, there can only be one query / mutation.
        let field_pair = parsed_object.fields.pop().unwrap();
        let serializer = Self::derive_serializer(&selections.pop().unwrap(), &field_pair.schema_field);

        if field_pair.schema_field.query_info.is_some() {
            let graph = self.dispatch_build(field_pair)?;
            Ok((graph, serializer))
        } else {
            Err(QueryGraphBuilderError::SchemaError(format!(
                "Expected query information to be attached on schema object '{}', field '{}'.",
                root_object.identifier.name(),
                field_pair.parsed_field.name
            )))
        }
    }

    fn dispatch_build(&self, field_pair: FieldPair) -> QueryGraphBuilderResult<QueryGraph> {
        let query_info = field_pair.schema_field.query_info.as_ref().unwrap();
        let parsed_field = field_pair.parsed_field;

        let mut graph = match (&query_info.tag, query_info.model.clone()) {
            (QueryTag::FindOne, Some(m)) => read::find_one(parsed_field, m).map(Into::into),
            (QueryTag::FindFirst, Some(m)) => read::find_first(parsed_field, m).map(Into::into),
            (QueryTag::FindMany, Some(m)) => read::find_many(parsed_field, m).map(Into::into),
            (QueryTag::Aggregate, Some(m)) => read::aggregate(parsed_field, m).map(Into::into),
            (QueryTag::CreateOne, Some(m)) => QueryGraph::root(|g| write::create_record(g, m, parsed_field)),
            (QueryTag::UpdateOne, Some(m)) => QueryGraph::root(|g| write::update_record(g, m, parsed_field)),
            (QueryTag::UpdateMany, Some(m)) => QueryGraph::root(|g| write::update_many_records(g, m, parsed_field)),
            (QueryTag::UpsertOne, Some(m)) => QueryGraph::root(|g| write::upsert_record(g, m, parsed_field)),
            (QueryTag::DeleteOne, Some(m)) => QueryGraph::root(|g| write::delete_record(g, m, parsed_field)),
            (QueryTag::DeleteMany, Some(m)) => QueryGraph::root(|g| write::delete_many_records(g, m, parsed_field)),
            (QueryTag::ExecuteRaw, _) => QueryGraph::root(|g| write::execute_raw(g, parsed_field)),
            (QueryTag::QueryRaw, _) => QueryGraph::root(|g| write::query_raw(g, parsed_field)),
            _ => unreachable!("Query builder dispatching failed."),
        }?;

        // Run final transformations.
        graph.finalize()?;
        trace!("{}", graph);

        Ok(graph)
    }

    fn derive_serializer(selection: &Selection, field: &OutputFieldRef) -> IrSerializer {
        IrSerializer {
            key: selection
                .alias()
                .clone()
                .unwrap_or_else(|| selection.name().to_string()),
            output_field: field.clone(),
        }
    }
}
