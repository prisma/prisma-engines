use super::*;
use crate::{
    query_ast::*,
    query_graph::{NodeRef, QueryGraph},
    ArgumentList, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use prisma_models::ModelRef;
use std::{convert::TryInto, sync::Arc};
use write_args_parser::*;

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_record(graph: &mut QueryGraph, model: &ModelRef, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let data_argument = field.arguments.pluck_required("data");
    let data_map: ParsedInputMap = data_argument.value.try_into()?;
    let create_node = create::create_record_node(graph, Arc::clone(model), data_map)?;

    dbg!(&field);

    // Follow-up read query on the write
    let read_query = ReadOneRecordBuilder::new(field, Arc::clone(model)).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.mark_result_node(&read_node);
    graph.create_edge(
        &create_node,
        &read_node,
        Some(inject_filter(model.primary_identifier())),
    );
    graph.flag_transactional();

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

    // for (relation_field, data_map) in create_args.nested {
    //     // nested::connect_nested_query(graph, create_node, relation_field, data_map)?;
    //     todo!()
    // }

    Ok(create_node)
}
