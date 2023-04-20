use super::*;
use crate::query_graph_builder::write::write_args_parser::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap,
};
use connector::{Filter, IntoFilter};
use prisma_models::ModelRef;
use schema::ConnectorContext;
use schema_builder::constants::args;
use std::convert::TryInto;

/// Creates an update record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn update_record(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    // "where"
    let where_arg: ParsedInputMap = field.arguments.lookup(args::WHERE).unwrap().value.try_into()?;
    let filter = extract_unique_filter(where_arg, &model)?;

    // "data"
    let data_argument = field.arguments.lookup(args::DATA).unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;

    let update_node = update_record_node(graph, connector_ctx, filter.clone(), model.clone(), data_map)?;

    let read_query = read::find_unique(field, model.clone())?;
    let read_node = graph.create_node(Query::Read(read_query));

    if connector_ctx.relation_mode.is_prisma() {
        let read_parent_node = graph.create_node(utils::read_ids_infallible(
            model.clone(),
            model.primary_identifier(),
            filter,
        ));

        utils::insert_emulated_on_update(graph, connector_ctx, &model, &read_parent_node, &update_node)?;

        graph.create_edge(
            &read_parent_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(
                model.primary_identifier(),
                Box::new(move |mut update_node, parent_ids| {
                    if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = update_node {
                        ur.record_filter = parent_ids.into();
                    }

                    Ok(update_node)
                }),
            ),
        )?;
    }

    graph.add_result_node(&read_node);
    graph.create_edge(
        &update_node,
        &read_node,
        QueryGraphDependency::ProjectedDataDependency(
            model.primary_identifier(),
            Box::new(move |mut read_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::RecordNotFound(
                        "Record to update not found.".to_string(),
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

/// Creates an update many record query and adds it to the query graph.
pub fn update_many_records(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    // "where"
    let filter = match field.arguments.lookup(args::WHERE) {
        Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
        None => Filter::empty(),
    };

    // "data"
    let data_argument = field.arguments.lookup(args::DATA).unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;

    if connector_ctx.relation_mode.uses_foreign_keys() {
        update_many_record_node(graph, connector_ctx, filter, model, data_map)?;
    } else {
        let pre_read_node = graph.create_node(utils::read_ids_infallible(
            model.clone(),
            model.primary_identifier(),
            filter,
        ));
        let update_many_node = update_many_record_node(graph, connector_ctx, Filter::empty(), model.clone(), data_map)?;

        utils::insert_emulated_on_update(graph, connector_ctx, &model, &pre_read_node, &update_many_node)?;

        graph.create_edge(
            &pre_read_node,
            &update_many_node,
            QueryGraphDependency::ProjectedDataDependency(
                model.primary_identifier(),
                Box::new(move |mut update_node, parent_ids| {
                    if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = update_node {
                        ur.record_filter = parent_ids.into();
                    }

                    Ok(update_node)
                }),
            ),
        )?;
    }

    Ok(())
}

/// Creates an update record query node and adds it to the query graph.
pub fn update_record_node<T: Clone>(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    filter: T,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<Filter>,
{
    graph.flag_transactional();

    let update_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = update_args.args;

    args.update_datetimes(&model);

    let filter: Filter = filter.into();
    let update_parent = Query::Write(WriteQuery::UpdateRecord(UpdateRecord {
        model: model.clone(),
        record_filter: filter.into(),
        args,
    }));
    let update_node = graph.create_node(update_parent);

    for (relation_field, data_map) in update_args.nested {
        nested::connect_nested_query(graph, connector_ctx, update_node, relation_field, data_map)?;
    }

    Ok(update_node)
}

/// Creates an update many record query node and adds it to the query graph.
pub fn update_many_record_node<T>(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    filter: T,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<Filter>,
{
    graph.flag_transactional();

    let filter = filter.into();
    let record_filter = filter.into();
    let update_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = update_args.args;

    args.update_datetimes(&model);

    let update_many = UpdateManyRecords {
        model,
        record_filter,
        args,
    };

    let update_many_node = graph.create_node(Query::Write(WriteQuery::UpdateManyRecords(update_many)));

    for (relation_field, data_map) in update_args.nested {
        nested::connect_nested_query(graph, connector_ctx, update_many_node, relation_field, data_map)?;
    }

    Ok(update_many_node)
}
