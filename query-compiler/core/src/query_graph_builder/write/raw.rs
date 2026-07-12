use super::*;
use crate::{ParsedField, query_ast::*, query_graph::QueryGraph};
use query_structure::{Model, PrismaValue};
use std::{collections::HashMap, convert::TryInto};

pub(crate) fn execute_raw(graph: &mut QueryGraph, field: ParsedField<'_>) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::ExecuteRaw(raw_query(None, None, field)?));

    graph.create_node(raw_query);
    Ok(())
}

pub(crate) fn query_raw(
    graph: &mut QueryGraph,
    model: Option<Model>,
    query_type: Option<String>,
    field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    let raw_query = Query::Write(WriteQuery::QueryRaw(raw_query(model, query_type, field)?));

    graph.create_node(raw_query);
    Ok(())
}

fn raw_query(
    model: Option<Model>,
    query_type: Option<String>,
    field: ParsedField<'_>,
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
