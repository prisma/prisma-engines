mod error;
mod expressionista;
mod interpreter;
mod read;
mod write;

pub use error::*;
pub use expressionista::*;
pub use interpreter::*;
pub use read::ReadQueryExecutor;
pub use write::WriteQueryExecutor;

use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    query_graph::*,
    response_ir::{Response, ResultIrBuilder},
    schema::QuerySchemaRef,
    CoreResult, OutputTypeRef, ResultInfo,
};
use connector::*;
use interpreter::*;
use std::sync::Arc;

/// Central query executor and main entry point into the query core.
pub struct QueryExecutor {
    read_executor: ReadQueryExecutor,
    write_executor: WriteQueryExecutor,
}

type QueryExecutionResult<T> = std::result::Result<T, QueryExecutionError>;

struct QueryPipeline<'a> {
    graph: QueryGraph,
    interpreter: QueryInterpreter<'a>,
    result_info: ResultInfo,
}

impl<'a> QueryPipeline<'a> {
    pub fn new(graph: QueryGraph, interpreter: QueryInterpreter<'a>, result_info: ResultInfo) -> Self {
        Self {
            graph: graph.transform(),
            interpreter,
            result_info,
        }
    }

    pub fn execute(self) -> QueryExecutionResult<Response> {
        let result_info = self.result_info;
        let exp = Expressionista::translate(self.graph);

        self.interpreter
            .interpret(exp, Env::default())
            .map(|result| ResultIrBuilder::build(result, result_info))
    }
}

// Todo:
// - Partial execution semantics?
// - Do we need a clearer separation of queries coming from different query blocks? (e.g. 2 query { ... } in GQL)
// - ReadQueryResult should probably just be QueryResult
// - This is all temporary code until the larger query execution overhaul.
impl QueryExecutor {
    pub fn new(read_executor: ReadQueryExecutor, write_executor: WriteQueryExecutor) -> Self {
        QueryExecutor {
            read_executor,
            write_executor,
        }
    }

    /// Executes a query document, which involves parsing & validating the document,
    /// building queries and a query execution plan, and finally calling the connector APIs to
    /// resolve the queries and build reponses.
    pub fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> QueryExecutionResult<Vec<Response>> {
        // Parse, validate, and extract queries from query document.
        let queries: Vec<(QueryGraph, ResultInfo)> = QueryBuilder::new(query_schema).build(query_doc)?;

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

    // fn execute_queries(&self, queries: Vec<QueryPair>) -> CoreResult<Vec<ResultPair>> {
    //     queries.into_iter().map(|query| self.execute_query(query)).collect()
    // }

    // fn execute_query(&self, query: QueryPair) -> CoreResult<ResultPair> {
    //     let (query, strategy) = query;
    //     let model_opt = query.extract_model();
    //     match query {
    //         Query::Read(read) => {
    //             let query_result = self.read_executor.execute(read, &[])?;

    //             Ok(match strategy {
    //                 ResultResolutionStrategy::Serialize(typ) => ResultPair::Read(query_result, typ),
    //                 ResultResolutionStrategy::Dependent(_) => unimplemented!(), // Dependent query exec. from read is not supported in this execution model.
    //             })
    //         }

    //         Query::Write(write) => {
    //             let query_result = self.write_executor.execute(write)?;

    //             match strategy {
    //                 ResultResolutionStrategy::Serialize(typ) => Ok(ResultPair::Write(query_result, typ)),
    //                 ResultResolutionStrategy::Dependent(dependent_pair) => match model_opt {
    //                     Some(model) => match *dependent_pair {
    //                         (Query::Read(ReadQuery::RecordQuery(mut rq)), strategy) => {
    //                             // Inject required information into the query and execute
    //                             rq.record_finder = Some(query_result.result.to_record_finder(model)?);

    //                             let dependent_pair = (Query::Read(ReadQuery::RecordQuery(rq)), strategy);
    //                             self.execute_query(dependent_pair)
    //                         }
    //                         _ => unreachable!(), // Invariant for now
    //                     },
    //                     None => Err(CoreError::ConversionError(
    //                         "Model required for dependent query execution".into(),
    //                     )),
    //                 },
    //             }
    //         }
    //     }
    // }

    /// Returns db name used in the executor.
    pub fn db_name(&self) -> String {
        self.write_executor.db_name.clone()
    }
}
