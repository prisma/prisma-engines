use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

pub fn connect_nested_set(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    // let mut finders = Vec::new();
    // for value in utils::coerce_vec(value) {
    //     let record_finder = extract_record_finder(value, &child_model)?;
    //     finders.push(record_finder);
    // }

    // let child_read_query = utils::read_ids_infallible(&child_model, finders);
    // let child_node = graph.create_node(child_read_query);

    // graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
    // // connect::connect_records_node(graph, &parent_node, &child_node, &parent_relation_field, None, None)?;

    // let set = WriteQuery::SetRecords(SetRecords {
    //     parent: None,
    //     wheres: vec![],
    //     relation_field: Arc::clone(&parent_relation_field),
    // });

    // let set_node = graph.create_node(Query::Write(set));

    // // Edge from parent to set.
    // graph.create_edge(
    //     &parent_node,
    //     &set_node,
    //     QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
    //         let len = parent_ids.len();
    //         if len == 0 {
    //             Err(QueryGraphBuilderError::AssertionError(format!(
    //                 "Required exactly one parent ID to be present for set query, found none."
    //             )))
    //         } else if len > 1 {
    //             Err(QueryGraphBuilderError::AssertionError(format!(
    //                 "Required exactly one parent ID to be present for set query, found {}.",
    //                 len
    //             )))
    //         } else {
    //             if let Node::Query(Query::Write(WriteQuery::SetRecords(ref mut x))) = child_node {
    //                 let parent_id = parent_ids.pop().unwrap();
    //                 x.parent = Some(parent_id.try_into()?);
    //             }

    //             Ok(child_node)
    //         }
    //     })),
    // )?;

    // // Edge from child to set.
    // graph.create_edge(
    //     &child_node,
    //     &set_node,
    //     QueryGraphDependency::ParentIds(Box::new(|mut child_node, parent_ids| {
    //         if let Node::Query(Query::Write(WriteQuery::SetRecords(ref mut x))) = child_node {
    //             x.wheres = parent_ids
    //                 .iter()
    //                 .map(|x| x.try_into().expect("Prisma Value was not a GraphqlId"))
    //                 .collect();
    //         }

    //         Ok(child_node)
    //     })),
    // )?;

    // Ok(())

    unimplemented!()
}
