use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{Filter, ScalarCompare};
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Adds a delete (single) record node to the graph and connects it to the parent.
/// Auxiliary nodes may be added to support the deletion process, e.g. extra read nodes.
///
/// If the relation is a list:
/// - Delete specific record from the list, a record finder must be present in the data.
///
/// If the relation is not a list:
/// - Just delete the one node that can be present, if desired (as it is a non-list, aka 1-to-1 relation).
/// - The relation HAS to be inlined, because it is 1-to-1.
/// - If the relation is inlined in the parent, we need to generate a read query to grab the ID of the record we want to delete.
/// - If the relation is inlined but not in the parent, we can directly generate a delete on the record with the parent ID.
pub fn connect_nested_delete(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        if relation_field.is_list {
            // Todo:
            // - we need to make sure the records are actually connected...
            // - What about the checks currently performed in `DeleteActions`?
            let record_finder = extract_record_finder(value, &child_model)?;
            //            let delete_record_node = delete::delete_record_node(graph, Some(record_finder), Arc::clone(&child_model))?;
            let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
                model: Arc::clone(&child_model),
                filter: record_finder.into(),
            });

            let delete_many_node = graph.create_node(Query::Write(delete_many));
            let id_field = child_model.fields().id();
            let find_child_records_node =
                utils::insert_find_children_by_parent_node(graph, parent, relation_field, Filter::empty())?;

            graph.create_edge(
                &find_child_records_node,
                &delete_many_node,
                QueryGraphDependency::ParentIds(Box::new(move |mut node, parent_ids| {
                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut ur))) = node {
                        let ids_filter = id_field.is_in(Some(parent_ids));
                        let new_filter = Filter::and(vec![ur.filter.clone(), ids_filter]);

                        ur.filter = new_filter;
                    }

                    Ok(node)
                })),
            )?;
        } else {
            let val: PrismaValue = value.try_into()?;
            let should_delete = if let PrismaValue::Boolean(b) = val { b } else { false };

            if should_delete {
                let find_child_records_node =
                    utils::insert_find_children_by_parent_node(graph, parent, relation_field, Filter::empty())?;

                let delete_record_node = graph.create_node(Query::Write(WriteQuery::DeleteRecord(DeleteRecord {
                    model: Arc::clone(&child_model),
                    where_: None,
                })));

                let id_field = child_model.fields().id();

                graph.create_edge(
                    &find_child_records_node,
                    &delete_record_node,
                    QueryGraphDependency::ParentIds(Box::new(move |mut node, mut parent_ids| {
                        let parent_id = match parent_ids.pop() {
                            Some(pid) => Ok(pid),
                            None => Err(QueryGraphBuilderError::AssertionError(format!(
                                "[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-many relation."
                            ))),
                        }?;

                        if let Node::Query(Query::Write(ref mut wq)) = node {
                            wq.inject_record_finder(RecordFinder {
                                field: id_field,
                                value: parent_id,
                            });
                        }

                        Ok(node)
                    })),
                )?;
            }
        }
    }

    Ok(())
}

pub fn connect_nested_delete_many(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let as_map: ParsedInputMap = value.try_into()?;
        let filter = extract_filter(as_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;

        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: Arc::clone(&child_model),
            filter,
        });

        let delete_many_node = graph.create_node(Query::Write(delete_many));
        let id_field = child_model.fields().id();

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut node, parent_ids| {
                if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut ur))) = node {
                    let ids_filter = id_field.is_in(Some(parent_ids));
                    let new_filter = Filter::and(vec![ur.filter.clone(), ids_filter]);

                    ur.filter = new_filter;
                }

                Ok(node)
            })),
        )?;
    }
    Ok(())
}
