use super::*;
use crate::{query_ast::*, query_graph::QueryGraph, ArgumentListLookup, ParsedField};
use prisma_value::PrismaValue;
use std::convert::TryInto;

pub fn execute_raw(graph: &mut QueryGraph, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::ExecuteRaw(raw_query(field)?));

    graph.create_node(raw_query);
    Ok(())
}

pub fn query_raw(graph: &mut QueryGraph, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::QueryRaw(raw_query(field)?));

    graph.create_node(raw_query);
    Ok(())
}

fn raw_query(mut field: ParsedField) -> QueryGraphBuilderResult<RawQuery> {
    let query_arg = field.arguments.lookup("query").unwrap().value;
    let parameters_arg = field.arguments.lookup("parameters").unwrap().value;

    let query_value: PrismaValue = query_arg.try_into()?;
    let parameter_value: PrismaValue = parameters_arg.try_into()?;

    Ok(RawQuery {
        query: query_value.into_string().unwrap(),
        parameters: parameter_value.into_list().unwrap(),
    })
}
