use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    DataExpectation, ParsedInputMap, ParsedInputValue,
};
use query_structure::{Filter, Model, PrismaValue, RecordFilter, RelationFieldRef};
use std::convert::TryInto;

/// Adds a delete (single) record node to the graph and connects it to the parent.
///
/// If the relation is a list:
/// - Delete specific record from the list, a record finder must be present in the data.
///
/// If the relation is not a list:
/// - Just delete the one node that can be present, if desired (as it is a non-list, aka 1-to-1 relation).
/// - The relation HAS to be inlined, because it is 1-to-1.
/// - If the relation is inlined in the parent, we need to generate a read query to grab the ID of the record we want to delete.
/// - If the relation is inlined but not in the parent, we can directly generate a delete on the record with the parent ID.
///
/// We always need to make sure that the records are connected before deletion.
pub fn nested_delete(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();

    if parent_relation_field.is_list() {
        let filters: Vec<Filter> = utils::coerce_vec(value)
            .into_iter()
            .map(|value: ParsedInputValue<'_>| {
                let value: ParsedInputMap<'_> = value.try_into()?;
                extract_unique_filter(value, child_model)
            })
            .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

        let filter_len = filters.len();
        let or_filter = Filter::Or(filters);
        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: child_model.clone(),
            record_filter: or_filter.clone().into(),
            limit: None,
        });

        let delete_many_node = graph.create_node(Query::Write(delete_many));
        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, or_filter)?;

        let dependencies =
            utils::insert_emulated_on_delete(graph, query_schema, child_model, &find_child_records_node)?;
        utils::create_execution_order_edges(graph, dependencies, delete_many_node)?;

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier,
                Box::new(move |mut delete_many_node, child_ids| {
                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_many_node {
                        dmr.record_filter = child_ids.into();
                    }

                    Ok(delete_many_node)
                }),
                Some(DataExpectation::exact_row_count(
                    filter_len,
                    RecordsNotConnected::builder()
                        .child(child_model.clone())
                        .parent(parent_relation_field.model())
                        .relation(parent_relation_field.relation())
                        .build(),
                )),
            ),
        )?;
    } else {
        let should_delete = match &value {
            ParsedInputValue::Single(PrismaValue::Boolean(b)) => *b,
            ParsedInputValue::Map(_) => true,
            _ => false,
        };

        if should_delete {
            let filter = match value {
                ParsedInputValue::Map(map) => extract_filter(map, child_model)?,
                _ => Filter::empty(),
            };

            let find_child_records_node =
                utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, filter.clone())?;

            let delete_record_node = graph.create_node(Query::Write(WriteQuery::DeleteRecord(DeleteRecord {
                name: String::new(),
                model: child_model.clone(),
                record_filter: filter.into(),
                selected_fields: None,
            })));

            let dependencies =
                utils::insert_emulated_on_delete(graph, query_schema, child_model, &find_child_records_node)?;
            utils::create_execution_order_edges(graph, dependencies, delete_record_node)?;

            graph.create_edge(
                &find_child_records_node,
                &delete_record_node,
                QueryGraphDependency::ProjectedDataDependency(
                    child_model_identifier,
                    Box::new(move |mut delete_record_node, mut child_ids| {
                        let child_id = child_ids.pop().expect("child id should be present");

                        if let Node::Query(Query::Write(WriteQuery::DeleteRecord(ref mut dq))) = delete_record_node {
                            dq.set_selectors(vec![child_id]);
                        }

                        Ok(delete_record_node)
                    }),
                    Some(DataExpectation::non_empty_rows(
                        MissingRelatedRecord::builder()
                            .model(child_model)
                            .relation(&parent_relation_field.relation())
                            .operation(DataOperation::NestedDelete)
                            .build(),
                    )),
                ),
            )?;
        }
    }

    Ok(())
}

pub fn nested_delete_many(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();

    for value in utils::coerce_vec(value) {
        let as_map: ParsedInputMap<'_> = value.try_into()?;
        let filter = extract_filter(as_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;

        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: child_model.clone(),
            record_filter: RecordFilter::empty(),
            limit: None,
        });

        let delete_many_node = graph.create_node(Query::Write(delete_many));
        let dependencies =
            utils::insert_emulated_on_delete(graph, query_schema, child_model, &find_child_records_node)?;
        utils::create_execution_order_edges(graph, dependencies, delete_many_node)?;

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                Box::new(move |mut delete_many_node, child_ids| {
                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_many_node {
                        dmr.record_filter = child_ids.into();
                    }

                    Ok(delete_many_node)
                }),
                None,
            ),
        )?;
    }

    Ok(())
}
