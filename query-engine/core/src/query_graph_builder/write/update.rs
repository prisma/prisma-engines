use super::*;
use crate::query_graph_builder::write::write_args_parser::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use connector::{Filter, IdFilter};
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};

/// Creates an update record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn update_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    // "where"
    let where_arg: ParsedInputMap = field.arguments.lookup("where").unwrap().value.try_into()?;
    let filter = extract_unique_filter(where_arg, &model)?;

    // "data"
    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;

    let update_node = update_record_node(graph, filter, Arc::clone(&model), data_map)?;

    let read_query = ReadOneRecordBuilder::new(field, model.clone()).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &update_node,
        &read_node,
        QueryGraphDependency::ParentProjection(
            model.primary_identifier(),
            Box::new(move |mut read_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::RecordNotFound("Record to update not found.".to_string())),
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

/// Creates an update many record query and adds it to the query graph.
pub fn update_many_records(
    graph: &mut QueryGraph,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    let filter = match field.arguments.lookup("where") {
        Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
        None => Filter::empty(),
    };

    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;
    let update_args = WriteArgsParser::from(&model, data_map)?;

    let mut args = update_args.args;
    args.update_datetimes(Arc::clone(&model));

    let record_filter = filter.into();
    let update_many = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        model,
        record_filter,
        args,
    });
    graph.create_node(Query::Write(update_many));

    Ok(())
}

/// Creates an update record query node and adds it to the query graph.
pub fn update_record_node<T>(
    graph: &mut QueryGraph,
    filter: T,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<Filter>,
{
    let update_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = update_args.args;

    args.update_datetimes(Arc::clone(&model));

    let filter = filter.into();
    let record_filter = filter.into();
    let ur = UpdateRecord {
        model,
        record_filter,
        args,
    };

    let node = graph.create_node(Query::Write(WriteQuery::UpdateRecord(ur)));
    for (relation_field, data_map) in update_args.nested {
        nested::connect_nested_query(graph, node, relation_field, data_map)?;
    }

    Ok(node)
}
