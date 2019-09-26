use super::*;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ReadOneRecordBuilder,
};
use connector::filter::RecordFinder;
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};

pub fn upsert_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let where_arg = field.arguments.lookup("where").unwrap();
    let record_finder = extract_record_finder(where_arg.value, &model)?;

    let create_argument = field.arguments.lookup("create").unwrap();
    let update_argument = field.arguments.lookup("update").unwrap();

    let child_read_query = utils::id_read_query_infallible(&model, record_finder.clone());
    let initial_read_node = graph.create_node(child_read_query);

    let create_node = create::create_record_node(graph, Arc::clone(&model), create_argument.value.try_into()?)?;
    let update_node = update::update_record_node(
        graph,
        Some(record_finder),
        Arc::clone(&model),
        update_argument.value.try_into()?,
    )?;

    let read_query = ReadOneRecordBuilder::new(field, Arc::clone(&model)).build()?;
    let read_node_create = graph.create_node(Query::Read(read_query.clone()));
    let read_node_update = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node_create);
    graph.add_result_node(&read_node_update);

    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &initial_read_node,
        &if_node,
        QueryGraphDependency::ParentId(Box::new(|node, parent_id| {
            if let Node::Flow(Flow::If(_)) = node {
                // Todo: This looks super unnecessary
                Node::Flow(Flow::If(Box::new(move || parent_id.is_some())))
            } else {
                node
            }
        })),
    );

    graph.create_edge(&if_node, &update_node, QueryGraphDependency::Then);

    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else);

    let id_field = model.fields().id();
    graph.create_edge(
        &update_node,
        &read_node_update,
        QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
            if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                let finder = RecordFinder {
                    field: id_field,
                    value: parent_id.unwrap(),
                };

                rq.record_finder = Some(finder);
            };

            node
        })),
    );

    let id_field = model.fields().id();
    graph.create_edge(
        &create_node,
        &read_node_create,
        QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
            if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                let finder = RecordFinder {
                    field: id_field,
                    value: parent_id.unwrap(),
                };

                rq.record_finder = Some(finder);
            };

            node
        })),
    );

    Ok(())
}
