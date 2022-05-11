use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputList, ParsedInputMap,
};
use connector::IntoFilter;
use prisma_models::ModelRef;
use schema::ConnectorContext;
use schema_builder::constants::args;
use std::{convert::TryInto, sync::Arc};
use write_args_parser::*;

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_record(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    let data_map = match field.arguments.lookup(args::DATA) {
        Some(data) => data.value.try_into()?,
        None => ParsedInputMap::default(),
    };

    let create_node = create::create_record_node(graph, connector_ctx, Arc::clone(&model), data_map)?;

    // Follow-up read query on the write
    let read_query = read::find_unique(field, model.clone())?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &create_node,
        &read_node,
        QueryGraphDependency::ProjectedDataDependency(
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

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_many_records(
    graph: &mut QueryGraph,
    _connector_ctx: &ConnectorContext,
    model: ModelRef,
    mut field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    let data_list: ParsedInputList = match field.arguments.lookup(args::DATA) {
        Some(data) => data.value.try_into()?,
        None => vec![],
    };

    let skip_duplicates: bool = match field.arguments.lookup(args::SKIP_DUPLICATES) {
        Some(arg) => arg.value.try_into()?,
        None => false,
    };

    let args = data_list
        .into_iter()
        .map(|data_value| {
            let data_map = data_value.try_into()?;
            let mut args = WriteArgsParser::from(&model, data_map)?.args;

            args.add_datetimes(&model);
            Ok(args)
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    let query = CreateManyRecords {
        model,
        args,
        skip_duplicates,
    };

    graph.create_node(Query::Write(WriteQuery::CreateManyRecords(query)));
    Ok(())
}

pub fn create_record_node(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<NodeRef> {
    let create_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = create_args.args;

    args.add_datetimes(&model);

    let cr = CreateRecord { model, args };
    let create_node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in create_args.nested {
        nested::connect_nested_query(graph, connector_ctx, create_node, relation_field, data_map)?;
    }

    Ok(create_node)
}
