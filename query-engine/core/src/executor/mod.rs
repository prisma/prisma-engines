mod error;
mod interpreter;
mod read;
mod write;

pub use error::*;
pub use read::ReadQueryExecutor;
pub use write::WriteQueryExecutor;

use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    response_ir::{Response, ResultIrBuilder},
    CoreError, CoreResult, OutputTypeRef, QueryPair, QuerySchemaRef, ResultPair, ResultResolutionStrategy,
};
use connector::*;
use interpreter::*;
use petgraph::{graph::*, *};
use prisma_models::RelationFieldRef;
use std::borrow::Borrow;
use std::sync::Arc;

/// Central query executor and main entry point into the query core.
pub struct QueryExecutor {
    read_executor: ReadQueryExecutor,
    write_executor: WriteQueryExecutor,
}

type QueryExecutionResult<T> = std::result::Result<T, QueryExecutionError>;
type QueryGraph = Graph<Query, Dependency>;

#[derive(Debug)]
pub enum Dependency {
    Write(RelationFieldRef),
    Read(OutputTypeRef),
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
        // 1. Parse and validate query document (building)
        let queries = QueryBuilder::new(query_schema).build(query_doc)?;

        // 2. Build query plan
        let mut graph = self.build_graph(queries);
        self.transform(&mut graph);

        let interpreter = QueryInterpreter {
            writer: self.write_executor.clone(),
            reader: self.read_executor.clone(),
        };

        let exp = Expressionista::translate(graph);
        let result = interpreter.interpret(exp, Env::default())?;
        dbg!(result);

        // 3. Execute query plan
        // let results: Vec<ResultPair> = self.execute_queries(queries)?;

        // 4. Build IR response / Parse results into IR response
        // Ok(results
        //     .into_iter()
        //     .fold(ResultIrBuilder::new(), |builder, result| builder.push(result))
        //     .build())
        unimplemented!()
    }

    fn build_graph(&self, pairs: Vec<QueryPair>) -> QueryGraph {
        let mut graph = QueryGraph::new();

        pairs.into_iter().for_each(|pair| match pair {
            (Query::Write(mut wq), ResultResolutionStrategy::Dependent(qp)) => {
                let nested = wq.replace_nested_writes();
                let top = graph.add_node(Query::Write(wq));

                self.build_nested_graph(top, nested, &mut graph);

                match *qp {
                    (Query::Read(rq), ResultResolutionStrategy::Serialize(typ)) => {
                        let read = graph.add_node(Query::Read(rq));
                        graph.add_edge(top, read, Dependency::Read(typ));
                    }
                    _ => unreachable!(),
                };
            }
            _ => unimplemented!(),
        });

        graph
    }

    fn build_nested_graph(&self, top: NodeIndex, nested: NestedWriteQueries, graph: &mut QueryGraph) {
        nested.creates.into_iter().for_each(|nc| {
            let relation_field = Arc::clone(&nc.relation_field);
            let nested = nc.nested_writes.clone();
            let n = graph.add_node(Query::Write(WriteQuery::Root("".into(), Some("".into()), nc.into())));

            graph.add_edge(top, n, Dependency::Write(relation_field));
            self.build_nested_graph(n, nested, graph);
        });
    }

    fn transform(&self, graph: &mut QueryGraph) {
        let candidates: Vec<EdgeIndex> = graph
            .raw_edges()
            .into_iter()
            .filter_map(|edge| {
                let parent = graph.node_weight(edge.source()).unwrap();
                let child = graph.node_weight(edge.target()).unwrap();
                let edge_index = graph.find_edge(edge.source(), edge.target()).unwrap();

                match (parent, child) {
                    (
                        Query::Write(WriteQuery::Root(_, _, RootWriteQuery::CreateRecord(_))),
                        Query::Write(WriteQuery::Root(_, _, RootWriteQuery::CreateRecord(_))),
                    ) => {
                        let relation_field: &RelationFieldRef = match &edge.weight {
                            Dependency::Write(rf) => rf,
                            _ => unreachable!(),
                        };

                        if dbg!(relation_field.relation_is_inlined_in_parent()) {
                            Some(edge_index)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        candidates.into_iter().for_each(|edge_index| {
            let (parent, child) = graph.edge_endpoints(edge_index).unwrap();
            let edge = graph.remove_edge(edge_index).unwrap();

            if let Dependency::Write(rf) = edge {
                graph.add_edge(child, parent, Dependency::Write(rf.related_field()));
            }
        });
    }

    fn execute_queries(&self, queries: Vec<QueryPair>) -> CoreResult<Vec<ResultPair>> {
        queries.into_iter().map(|query| self.execute_query(query)).collect()
    }

    fn execute_query(&self, query: QueryPair) -> CoreResult<ResultPair> {
        let (query, strategy) = query;
        let model_opt = query.extract_model();
        match query {
            Query::Read(read) => {
                let query_result = self.read_executor.execute(read, &[])?;

                Ok(match strategy {
                    ResultResolutionStrategy::Serialize(typ) => ResultPair::Read(query_result, typ),
                    ResultResolutionStrategy::Dependent(_) => unimplemented!(), // Dependent query exec. from read is not supported in this execution model.
                })
            }

            Query::Write(write) => {
                let query_result = self.write_executor.execute(write)?;

                match strategy {
                    ResultResolutionStrategy::Serialize(typ) => Ok(ResultPair::Write(query_result, typ)),
                    ResultResolutionStrategy::Dependent(dependent_pair) => match model_opt {
                        Some(model) => match *dependent_pair {
                            (Query::Read(ReadQuery::RecordQuery(mut rq)), strategy) => {
                                // Inject required information into the query and execute
                                rq.record_finder = Some(query_result.result.to_record_finder(model)?);

                                let dependent_pair = (Query::Read(ReadQuery::RecordQuery(rq)), strategy);
                                self.execute_query(dependent_pair)
                            }
                            _ => unreachable!(), // Invariant for now
                        },
                        None => Err(CoreError::ConversionError(
                            "Model required for dependent query execution".into(),
                        )),
                    },
                }
            }
        }
    }

    /// Returns db name used in the executor.
    pub fn db_name(&self) -> String {
        self.write_executor.db_name.clone()
    }
}
