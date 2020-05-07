use super::*;
use crate::{query_ast::*, query_graph::QueryGraph, ArgumentListLookup, ParsedField};
use prisma_value::PrismaValue;
use std::convert::TryInto;

pub fn raw_query(graph: &mut QueryGraph, mut field: ParsedField) -> QueryGraphBuilderResult<()> {
    let query = field.arguments.lookup("query").unwrap();
    let parameters = field.arguments.lookup("parameters").unwrap();

    let query_value: PrismaValue = query.value.try_into()?;
    let parameter_value: PrismaValue = parameters.value.try_into()?;

    let raw_query = Query::Write(WriteQuery::Raw(RawQuery {
        query: query_value.into_string().unwrap(),
        parameters: parameter_value.into_list().unwrap(),
    }));

    graph.create_node(raw_query);
    Ok(())
}
