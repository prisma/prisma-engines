use super::*;
use crate::{
    query_ast::*,
    query_graph::{QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ReadOneRecordBuilder,
};
use connector::{
    filter::Filter,
};
use prisma_models::{ModelRef};
use std::{convert::TryInto, sync::Arc};

/// Creates a delete record query and adds it to the query graph.
pub fn delete_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<()> {
    let where_arg = field.arguments.lookup("where").unwrap();
    let record_finder = extract_record_finder(where_arg.value, &model)?;

    // Prefetch read query for the delete
    let mut read_query = ReadOneRecordBuilder::new(field, Arc::clone(&model)).build()?;
    read_query.inject_record_finder(record_finder.clone());

    let read_node = graph.create_node(Query::Read(read_query));
    let delete_query = WriteQuery::DeleteRecord(DeleteRecord {
        model,
        where_: record_finder,
    });
    let delete_node = graph.create_node(Query::Write(delete_query));

    graph.add_result_node(&read_node);
    graph
        .create_edge(&read_node, &delete_node, QueryGraphDependency::ExecutionOrder);

    Ok(())
}

/// Creates a delete many records query and adds it to the query graph.
pub fn delete_many_records(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<()> {
    let filter = match field.arguments.lookup("where") {
        Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
        None => Filter::empty(),
    };

    let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords { model, filter });

    graph.create_node(Query::Write(delete_many));
    Ok(())
}