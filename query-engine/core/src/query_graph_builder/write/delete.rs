use super::{write_args_parser::WriteArgsParser, *};
use crate::{
    query_ast::*,
    query_graph::{Node, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, FilteredQuery, ParsedField, ParsedInputMap,
};
use psl::datamodel_connector::ConnectorCapability;
use query_structure::{Filter, Model};
use schema::{constants::args, QuerySchema};
use std::convert::TryInto;

/// Creates a top level delete record query and adds it to the query graph.
pub(crate) fn delete_record(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    mut field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    graph.flag_transactional();

    let where_arg = field.arguments.lookup(args::WHERE).unwrap();
    let filter = extract_unique_filter(where_arg.value.try_into()?, &model)?;

    if can_use_atomic_delete(query_schema, &field) {
        // Database supports returning the deleted row, so just the delete node will suffice.
        let nested_fields = field.nested_fields.unwrap().fields;
        let selected_fields = read::utils::collect_selected_scalars(&nested_fields, &model);
        let selection_order = read::utils::collect_selection_order(&nested_fields);
        let delete_query = Query::Write(WriteQuery::DeleteRecord(DeleteRecord {
            name: field.name,
            model,
            record_filter: Some(filter.into()),
            selected_fields: Some(DeleteRecordFields {
                fields: selected_fields,
                order: selection_order,
            }),
        }));
        let delete_node = graph.create_node(delete_query);

        // TODO laplab: figure out how do we update relations here.
        graph.add_result_node(&delete_node);
    } else {
        // In case database does not support returning the deleted row, we need to emulate that
        // behaviour by first reading the row and only then deleting it.
        let mut read_query = read::find_unique(field, model.clone(), query_schema)?;
        read_query.add_filter(filter.clone());
        let read_node = graph.create_node(Query::Read(read_query));

        let delete_query = Query::Write(WriteQuery::DeleteRecord(DeleteRecord {
            name: String::new(),
            model: model.clone(),
            record_filter: Some(filter.into()),
            selected_fields: None,
        }));
        let delete_node = graph.create_node(delete_query);

        // Ensure relevant relations are updated after delete.
        utils::insert_emulated_on_delete(graph, query_schema, &model, &read_node, &delete_node)?;

        // If the read node did not find the row, we know for sure that the delete node also won't
        // find it because:
        //  1. Both nodes use the same filter
        //  2. Whole operation is executed in a transaction
        // We insert a "fake" dependency between the nodes to avoid executing the delete if read
        // failed. Delete node does not actually need primary identifier from read operation - it
        // just needs to know that we read something.
        graph.create_edge(
            &read_node,
            &delete_node,
            // TODO laplab: should this be `DataDependency`? Currently, `DeleteRecord` only ever
            // returns count. Now it returns the record as well, but this is not in 100% of cases.
            QueryGraphDependency::ProjectedDataDependency(
                model.primary_identifier(),
                Box::new(|delete_node, parent_ids| {
                    if !parent_ids.is_empty() {
                        Ok(delete_node)
                    } else {
                        Err(QueryGraphBuilderError::RecordNotFound(
                            "Record to delete does not exist.".to_owned(),
                        ))
                    }
                }),
            ),
        )?;

        // Read node is the result one, because it returns the row we just deleted.
        graph.add_result_node(&read_node);
    }

    Ok(())
}

/// Creates a top level delete many records query and adds it to the query graph.
pub fn delete_many_records(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model: Model,
    mut field: ParsedField<'_>,
) -> QueryGraphBuilderResult<()> {
    let filter = match field.arguments.lookup(args::WHERE) {
        Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
        None => Filter::empty(),
    };

    let model_id = model.primary_identifier();
    let record_filter = filter.clone().into();
    let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
        model: model.clone(),
        record_filter,
    });

    let delete_many_node = graph.create_node(Query::Write(delete_many));

    if query_schema.relation_mode().is_prisma() {
        graph.flag_transactional();

        let read_query = utils::read_ids_infallible(model.clone(), model_id.clone(), filter);
        let read_query_node = graph.create_node(read_query);

        utils::insert_emulated_on_delete(graph, query_schema, &model, &read_query_node, &delete_many_node)?;

        graph.create_edge(
            &read_query_node,
            &delete_many_node,
            QueryGraphDependency::ProjectedDataDependency(
                model_id,
                Box::new(|mut delete_many_node, ids| {
                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_many_node {
                        dmr.record_filter = ids.into();
                    }

                    Ok(delete_many_node)
                }),
            ),
        )?;
    }

    graph.add_result_node(&delete_many_node);

    Ok(())
}

/// An atomic delete is a delete performed in a single operation. It uses `DELETE ... RETURNING` or
/// similar statement.
/// We only perform such delete when:
/// 1. Connector supports such operations
/// 2. The selection set contains no relation
fn can_use_atomic_delete(query_schema: &QuerySchema, field: &ParsedField<'_>) -> bool {
    // TODO laplab: check that the filter does not contain any predicates on relations.
    if !query_schema.has_capability(ConnectorCapability::DeleteReturning) {
        return false;
    }

    if field.has_nested_selection() {
        return false;
    }

    true
}
