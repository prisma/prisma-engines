use super::*;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap,
};
use connector::IdFilter;
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};

pub fn upsert_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    let where_arg: ParsedInputMap = field.arguments.lookup("where").unwrap().value.try_into()?;

    let filter = extract_unique_filter(where_arg, &model)?;
    let model_id = model.primary_identifier();

    let create_argument = field.arguments.lookup("create").unwrap();
    let update_argument = field.arguments.lookup("update").unwrap();

    let read_parent_records = utils::read_ids_infallible(model.clone(), model_id.clone(), filter.clone());
    let read_parent_records_node = graph.create_node(read_parent_records);

    let create_node = create::create_record_node(graph, Arc::clone(&model), create_argument.value.try_into()?)?;
    let update_node = update::update_record_node(graph, filter, Arc::clone(&model), update_argument.value.try_into()?)?;

    let read_query = read::find_one(field, Arc::clone(&model))?;
    let read_node_create = graph.create_node(Query::Read(read_query.clone()));
    let read_node_update = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node_create);
    graph.add_result_node(&read_node_update);

    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_parent_records_node,
        &if_node,
        QueryGraphDependency::ParentProjection(
            model_id.clone(),
            Box::new(|if_node, parent_ids| {
                if let Node::Flow(Flow::If(_)) = if_node {
                    // Todo: This looks super unnecessary
                    Ok(Node::Flow(Flow::If(Box::new(move || !parent_ids.is_empty()))))
                } else {
                    Ok(if_node)
                }
            }),
        ),
    )?;

    graph.create_edge(&if_node, &update_node, QueryGraphDependency::Then)?;
    graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;
    graph.create_edge(
        &update_node,
        &read_node_update,
        QueryGraphDependency::ParentProjection(
            model_id.clone(),
            Box::new(move |mut read_node_update, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(
                        "Expected a valid parent ID to be present for create follow-up for upsert query.".to_string(),
                    )),
                }?;

                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = read_node_update {
                    rq.add_filter(parent_id.filter());
                };

                Ok(read_node_update)
            }),
        ),
    )?;

    graph.create_edge(
        &create_node,
        &read_node_create,
        QueryGraphDependency::ParentProjection(
            model_id,
            Box::new(move |mut read_node_create, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(
                        "Expected a valid parent ID to be present for update follow-up for upsert query.".to_string(),
                    )),
                }?;

                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = read_node_create {
                    rq.add_filter(parent_id.filter());
                };

                Ok(read_node_create)
            }),
        ),
    )?;

    Ok(())
}
