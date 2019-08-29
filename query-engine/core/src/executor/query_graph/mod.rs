mod builder;

use builder::*;
use prisma_models::RelationFieldRef;
use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    response_ir::{Response, ResultIrBuilder},
    CoreError, CoreResult, OutputTypeRef, QueryPair, QuerySchemaRef, ResultPair, ResultResolutionStrategy,
};
use connector::*;
use petgraph::{graph::*};

type InnerGraph = Graph<Query, GraphEdge>;

#[derive(Debug, Default)]
pub struct QueryGraph {
    graph: InnerGraph,
}

#[derive(Debug)]
pub enum GraphEdge {
    Write(RelationFieldRef),
    Read(OutputTypeRef),
}

impl From<query> for QueryGraph {
    fn from(q: Query) -> Self {
        QueryGraphBuilder::build(q)
    }
}

impl QueryGraph {

}
