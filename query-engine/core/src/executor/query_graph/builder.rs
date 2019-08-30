use super::*;
use prisma_models::RelationFieldRef;
use crate::{
    query_builders::QueryBuilder,
    query_document::QueryDocument,
    response_ir::{Response, ResultIrBuilder},
    CoreError, CoreResult, OutputTypeRef, QueryPair, QuerySchemaRef, ResultPair, ResultResolutionStrategy,
};
use connector::*;
use petgraph::{graph::*};
use std::sync::Arc;

pub struct QueryGraphBuilder {}

impl QueryGraphBuilder {
    // WIP change to query from pair semantic
    pub fn build(query: Query) -> QueryGraph {
        unimplemented!()
        // let mut graph = QueryGraph::new();

        // match query {
        //     (Query::Write(mut wq), ResultResolutionStrategy::Dependent(qp)) => {
        //         let nested = wq.replace_nested_writes();
        //         let top = graph.add_node(Query::Write(wq));

        //         Self::build_nested_graph(top, nested, &mut graph);

        //         match *qp {
        //             (Query::Read(rq), ResultResolutionStrategy::Serialize(typ)) => {
        //                 let read = graph.add_node(Query::Read(rq));
        //                 graph.add_edge(top, read, GraphEdge::Read(typ));
        //             }
        //             _ => unreachable!(),
        //         };
        //     }
        //     _ => unimplemented!(),
        // };

        // Self::transform(&mut graph);

        // QueryGraph {
        //     graph
        // }
    }

    // fn build_nested_graph(top: NodeIndex, nested: NestedWriteQueries, graph: &mut InnerGraph) {
    //     nested.creates.into_iter().for_each(|nc| {
    //         let relation_field = Arc::clone(&nc.relation_field);
    //         let nested = nc.nested_writes.clone();
    //         let n = graph.add_node(Query::Write(WriteQuery::Root("".into(), Some("".into()), nc.into())));

    //         graph.add_edge(top, n, GraphEdge::Write(relation_field));
    //         Self::build_nested_graph(n, nested, graph);
    //     });
    // }

    // pub fn transform(graph: &mut QueryGraph) {
    //     let candidates: Vec<EdgeIndex> = self.graph
    //         .raw_edges()
    //         .into_iter()
    //         .filter_map(|edge| {
    //             let parent = graph.node_weight(edge.source()).unwrap();
    //             let child = graph.node_weight(edge.target()).unwrap();
    //             let edge_index = graph.find_edge(edge.source(), edge.target()).unwrap();

    //             match (parent, child) {
    //                 (
    //                     Query::Write(WriteQuery::Root(_, _, RootWriteQuery::CreateRecord(_))),
    //                     Query::Write(WriteQuery::Root(_, _, RootWriteQuery::CreateRecord(_))),
    //                 ) => {
    //                     let relation_field: &RelationFieldRef = match &edge.weight {
    //                         GraphEdge::Write(rf) => rf,
    //                         _ => unreachable!(),
    //                     };

    //                     if dbg!(relation_field.relation_is_inlined_in_parent()) {
    //                         Some(edge_index)
    //                     } else {
    //                         None
    //                     }
    //                 }
    //                 _ => None,
    //             }
    //         })
    //         .collect();

    //     candidates.into_iter().for_each(|edge_index| {
    //         let (parent, child) = self.graph.edge_endpoints(edge_index).unwrap();
    //         let edge = self.graph.remove_edge(edge_index).unwrap();

    //         if let GraphEdge::Write(rf) = edge {
    //             self.graph.add_edge(child, parent, GraphEdge::Write(rf.related_field()));
    //         }
    //     });
    // }
}