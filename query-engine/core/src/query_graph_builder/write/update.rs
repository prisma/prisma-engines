use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use connector::{filter::Filter, ScalarCompare};
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};
use write_arguments::*;

/// Creates an update record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn update_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let id_field = model.fields().id();

    // "where"
    let where_arg = field.arguments.lookup("where").unwrap();
    let record_finder = extract_record_finder(where_arg.value, &model)?;

    // "data"
    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;

    let update_node = update_record_node(graph, Some(record_finder), Arc::clone(&model), data_map)?;

    let read_query = ReadOneRecordBuilder::new(field, model).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &update_node,
        &read_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "Record to update not found."
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
    let update_args = WriteArguments::from(&model, data_map)?;

    let list_causes_update = !update_args.list.is_empty();
    let mut non_list_args = update_args.non_list;

    non_list_args.update_datetimes(Arc::clone(&model), list_causes_update);

    let update_many = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        model,
        filter,
        non_list_args,
        list_args: update_args.list,
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
    let update_args = WriteArguments::from(&model, data_map)?;
    let list_causes_update = !update_args.list.is_empty();
    let mut non_list_args = update_args.non_list;

    non_list_args.update_datetimes(Arc::clone(&model), list_causes_update);

    let ur = UpdateRecord {
        model,
        where_: filter.into(),
        non_list_args,
        list_args: update_args.list,
    };

    let node = graph.create_node(Query::Write(WriteQuery::UpdateRecord(ur)));
    for (relation_field, data_map) in update_args.nested {
        nested::connect_nested_query(graph, node, relation_field, data_map)?;
    }

    Ok(node)
}
