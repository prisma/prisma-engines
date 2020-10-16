use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use connector::IdFilter;
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};
use write_args_parser::*;

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let data_map = match field.arguments.lookup("data") {
        Some(data) => data.value.try_into()?,
        None => ParsedInputMap::new(),
    };

    let create_node = create::create_record_node(graph, Arc::clone(&model), data_map)?;

    // Follow-up read query on the write
    let read_query = ReadOneRecordBuilder::new(field, model.clone()).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &create_node,
        &read_node,
        QueryGraphDependency::ParentProjection(
            model.primary_identifier(),
            Box::new(move |mut read_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(
                        "Expected a valid parent ID to be present for create follow-up read query.".to_string(),
                    )),
                }?;

                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = read_node {
                    rq.add_filter(parent_id.filter());
                };

                Ok(read_node)
            }),
        ),
    )?;

    Ok(())
}

pub fn create_record_node(
    graph: &mut QueryGraph,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef> {
    let create_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = create_args.args;

    args.add_datetimes(Arc::clone(&model));

    let cr = CreateRecord { model, args };
    let create_node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in create_args.nested {
        nested::connect_nested_query(graph, create_node, relation_field, data_map)?;
    }

    Ok(create_node)
}
