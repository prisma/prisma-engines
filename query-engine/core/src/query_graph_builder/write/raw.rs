use super::*;
use crate::{query_ast::*, query_graph::QueryGraph, ParsedField};
use prisma_models::ModelRef;
use prisma_value::PrismaValue;
use std::{collections::HashMap, convert::TryInto};

pub fn execute_raw(graph: &mut QueryGraph, field: ParsedField) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::ExecuteRaw(raw_query(None, None, field)?));

    graph.create_node(raw_query);
    Ok(())
}

pub fn query_raw(
    graph: &mut QueryGraph,
    model: Option<ModelRef>,
    query_type: Option<String>,
    field: ParsedField,
) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::QueryRaw(raw_query(model, query_type, field)?));

    graph.create_node(raw_query);
    Ok(())
}

fn raw_query(
    model: Option<ModelRef>,
    query_type: Option<String>,
    field: ParsedField,
) -> QueryGraphBuilderResult<RawQuery> {
    let inputs = field
        .arguments
        .into_iter()
        .map(|arg| {
            let parsed_arg_value: PrismaValue = arg.value.try_into()?;

            Ok((arg.name, parsed_arg_value))
        })
        .collect::<QueryGraphBuilderResult<HashMap<_, _>>>()?;

    Ok(RawQuery {
        model,
        inputs,
        query_type,
    })
}
