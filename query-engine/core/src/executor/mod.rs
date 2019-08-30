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
    query_graph::*,
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    response_ir::{Response, ResultIrBuilder},
    CoreResult, OutputTypeRef, schema::QuerySchemaRef, ResultInfo,
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

struct QueryPipeline {
    graph: QueryGraph,
    interpreter: QueryInterpreter,
    result_info: ResultInfo,
}

impl QueryPipeline {
    pub fn new(query: Query, interpreter: QueryInterpreter, result_info: ResultInfo) -> Self {
        let graph = QueryGraph::from(query);
        Self {
            graph,
            interpreter,
            result_info,
        }
    }

    pub fn execute(self) -> QueryExecutionResult<Response> {
        let result_info = self.result_info;
        let exp = Expressionista::translate(self.graph);

        self.interpreter.interpret(exp, Env::default()).map(|result| ResultIrBuilder::build(result, result_info))
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
    pub fn execute(&self, query_doc: QueryDocument, query_schema: QuerySchemaRef) -> CoreResult<Vec<Response>> {
        // Parse and validate query document
        let queries: Vec<(Query, ResultInfo)> = QueryBuilder::new(query_schema).build_test(query_doc)?;

        // Create pipelines for all separate queries
        let results = queries.into_iter().map(|(query, info)| {
            let interpreter = QueryInterpreter {
                writer: self.write_executor.clone(),
                reader: self.read_executor.clone(),
            };

            QueryPipeline::new(query, interpreter, info)
        });

        // todo merge results
        unimplemented!()
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
