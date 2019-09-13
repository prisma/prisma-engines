mod error;
mod expressionista;
mod interpreter;
mod read;
mod write;
mod formatters;

pub use error::*;
pub use formatters::*;
pub use expressionista::*;
pub use interpreter::*;
pub use read::ReadQueryExecutor;
pub use write::WriteQueryExecutor;

use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    query_graph::*,
    response_ir::{IrSerializer, Response},
    schema::QuerySchemaRef,
};
use connector::*;

type QueryExecutionResult<T> = std::result::Result<T, QueryExecutionError>;

/// Central query executor and main entry point into the query core.
pub struct QueryExecutor {
    read_executor: ReadQueryExecutor,
    write_executor: WriteQueryExecutor,
}

struct QueryPipeline<'a> {
    graph: QueryGraph,
    interpreter: QueryInterpreter<'a>,
    serializer: IrSerializer,
}

impl<'a> QueryPipeline<'a> {
    pub fn new(graph: QueryGraph, interpreter: QueryInterpreter<'a>, serializer: IrSerializer) -> Self {
        Self {
            graph,
            interpreter,
            serializer,
        }
    }

    pub fn execute(self) -> QueryExecutionResult<Response> {
        let serializer = self.serializer;

        println!("{}", self.graph);
        let expr = Expressionista::translate(self.graph)?;

        println!("{}", format_expression(&expr, 0));
        self.interpreter
            .interpret(expr, Env::default())
            .map(|result| serializer.serialize(result))
    }
}

// Todo:
// - Partial execution semantics?
// - ReadQueryResult + write query results should probably just be QueryResult
impl QueryExecutor {
    pub fn new(read_executor: ReadQueryExecutor, write_executor: WriteQueryExecutor) -> Self {
        QueryExecutor {
            read_executor,
            write_executor,
        }
    }

    /// Executes a query document, which involves parsing & validating the document,
    /// building queries in a query graph, translating that graph to an expression tree, and finally interpreting the expression tree
    /// to resolve all queries and build responses.
    pub fn execute(
        &self,
        query_doc: QueryDocument,
        query_schema: QuerySchemaRef,
    ) -> QueryExecutionResult<Vec<Response>> {
        // Parse, validate, and extract query graphs from query document.
        let queries: Vec<(QueryGraph, IrSerializer)> = QueryBuilder::new(query_schema).build(query_doc)?;

        // Create pipelines for all separate queries
        queries
            .into_iter()
            .map(|(query_graph, info)| {
                let interpreter = QueryInterpreter {
                    writer: &self.write_executor,
                    reader: &self.read_executor,
                };

                QueryPipeline::new(query_graph, interpreter, info).execute()
            })
            .collect::<QueryExecutionResult<Vec<Response>>>()
    }

    /// Returns db name used in the executor.
    // TODO the upper layers should never be forced to care about DB names.
    // The way the connectors are structured at the moment forces all layers to know about db names.
    pub fn db_name(&self) -> String {
        self.write_executor.db_name.clone()
    }
}
