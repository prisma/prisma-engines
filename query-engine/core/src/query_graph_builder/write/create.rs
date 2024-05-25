use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputList, ParsedInputMap,
};
use connector::WriteArgs;
use psl::{datamodel_connector::ConnectorCapability, parser_database::RelationFieldId};
use query_structure::{IntoFilter, Model, Zipper};
use schema::{constants::args, QuerySchema};
use std::convert::TryInto;
use write_args_parser::*;

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub(crate) fn create_record(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    mut field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    let data_map = match field.arguments.lookup(args::DATA) {
        Some(data) => data.value.try_into()?,
        None => ParsedInputMap::default(),
    };

    if can_use_atomic_create(query_schema, &model, &data_map, &field) {
        let create_node = create::atomic_create_record_node(graph, query_schema, model, data_map, field)?;

        graph.add_result_node(&create_node);
    } else {
        graph.flag_transactional();

        let create_node = create::create_record_node(graph, query_schema, model.clone(), data_map)?;

        // Follow-up read query on the write
        let read_query = read::find_unique(field, model.clone(), query_schema)?;
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
    }

    Ok(())
}

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub(crate) fn create_many_records(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    with_field_selection: bool,
    mut field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    let data_list: ParsedInputList<'_> = match field.arguments.lookup(args::DATA) {
        Some(data) => utils::coerce_vec(data.value),
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

    let selected_fields = if with_field_selection {
        let (selected_fields, selection_order, nested_read) =
            super::read::utils::extract_selected_fields(field.nested_fields.unwrap().fields, &model, query_schema)?;

        Some(CreateManyRecordsFields {
            fields: selected_fields,
            order: selection_order,
            nested: nested_read,
        })
    } else {
        None
    };

    let query = CreateManyRecords {
        name: field.name,
        model,
        args,
        skip_duplicates,
        selected_fields,
        split_by_shape: !query_schema.has_capability(ConnectorCapability::SupportsDefaultInInsert),
    };

    graph.create_node(Query::Write(WriteQuery::CreateManyRecords(query)));

    Ok(())
}

pub fn create_record_node(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    data_map: ParsedInputMap<'_>,
) -> QueryGraphBuilderResult<NodeRef> {
    let mut parser = WriteArgsParser::from(&model, data_map)?;
    parser.args.add_datetimes(&model);
    create_record_node_from_args(graph, query_schema, model, parser.args, parser.nested)
}

pub(crate) fn create_record_node_from_args(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    args: WriteArgs,
    nested: Vec<(Zipper<RelationFieldId>, ParsedInputMap<'_>)>,
) -> QueryGraphBuilderResult<NodeRef> {
    let selected_fields = model.primary_identifier();
    let selection_order = selected_fields.db_names().collect();

    let cr = CreateRecord {
        // A regular create record is never used as a result node. Therefore, it's never serialized, so we don't need a name.
        name: String::new(),
        model,
        args,
        selected_fields,
        selection_order,
    };

    let create_node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in nested {
        nested::connect_nested_query(graph, query_schema, create_node, relation_field, data_map)?;
    }

    Ok(create_node)
}

/// An atomic create is a create performed in a single operation.
/// It uses `INSERT ... RETURNING` when the connector supports it.
/// We only perform such create when:
/// 1. There's no nested operations
/// 2. The selection set contains no relation
fn can_use_atomic_create(
    query_schema: &QuerySchema,
    model: &Model,
    data_map: &ParsedInputMap<'_>,
    field: &ParsedField<'_>,
) -> bool {
    // If the connector does not support RETURNING at all
    if !query_schema.has_capability(ConnectorCapability::InsertReturning) {
        return false;
    }

    // If the operation has nested operations
    if WriteArgsParser::has_nested_operation(model, data_map) {
        return false;
    }

    // If the operation has nested selection sets
    if field.has_nested_selection() {
        return false;
    }

    true
}

/// Creates a create record query that's done in a single operation and adds it to the query graph.
/// Translates to an `INSERT ... RETURNING` under the hood.
fn atomic_create_record_node(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    data_map: ParsedInputMap<'_>,
    field: ParsedField<'_>,
) -> QueryGraphBuilderResult<NodeRef> {
    let create_args = WriteArgsParser::from(&model, data_map)?;
    let mut args = create_args.args;

    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order: Vec<String> = read::utils::collect_selection_order(&nested_fields);
    let selected_fields = read::utils::collect_selected_scalars(&nested_fields, &model);

    args.add_datetimes(&model);

    let cr = CreateRecord {
        name: field.name.clone(),
        model,
        args,
        selected_fields,
        selection_order,
    };

    let create_node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in create_args.nested {
        nested::connect_nested_query(graph, query_schema, create_node, relation_field, data_map)?;
    }

    Ok(create_node)
}
