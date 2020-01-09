use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use connector::ScalarCompare;
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};
use write_arguments::*;

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let id_field = model.identifier();
    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;
    let create_node = create::create_record_node(graph, Arc::clone(&model), data_map)?;

    // Follow-up read query on the write
    let read_query = ReadOneRecordBuilder::new(field, model).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &create_node,
        &read_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "Expected a valid parent ID to be present for create follow-up read query."
                ))),
            }?;

            if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                rq.add_filter(id_field.equals(parent_id));
            };

            Ok(node)
        })),
    )?;

    Ok(())
}

pub fn create_record_node(
    graph: &mut QueryGraph,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef> {
    let create_args = WriteArguments::from(&model, data_map)?;
    let mut args = create_args.args;

    args.add_datetimes(Arc::clone(&model));

    let cr = CreateRecord { model, args };

    let create_node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in create_args.nested {
        nested::connect_nested_query(graph, create_node, relation_field, data_map)?;
    }

    Ok(create_node)
}
