use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue,
};
use connector::{Filter, RecordFilter};
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

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
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().primary_identifier();

    if parent_relation_field.is_list {
        let filters: Vec<Filter> = utils::coerce_vec(value)
            .into_iter()
            .map(|value: ParsedInputValue| {
                let value: ParsedInputMap = value.try_into()?;
                extract_unique_filter(value, &child_model)
            })
            .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

        let filter_len = filters.len();
        let or_filter = Filter::Or(filters);
        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: Arc::clone(&child_model),
            record_filter: or_filter.clone().into(),
        });

        let delete_many_node = graph.create_node(Query::Write(delete_many));
        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, or_filter)?;

        utils::insert_deletion_checks(graph, child_model, &find_child_records_node, &delete_many_node)?;

        let relation_name = parent_relation_field.relation().name.clone();
        let parent_name = parent_relation_field.model().name.clone();
        let child_name = child_model.name.clone();

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ParentProjection(
                child_model_identifier,
                Box::new(move |mut delete_many_node, child_ids| {
                    if child_ids.len() != filter_len {
                        return Err(QueryGraphBuilderError::RecordsNotConnected {
                            relation_name,
                            parent_name,
                            child_name,
                        });
                    }

                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_many_node {
                        dmr.record_filter = child_ids.into();
                    }

                    Ok(delete_many_node)
                }),
            ),
        )?;
    } else {
        let val: PrismaValue = value.try_into()?;
        let should_delete = if let PrismaValue::Boolean(b) = val { b } else { false };

        if should_delete {
            let find_child_records_node =
                utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

            let delete_record_node = graph.create_node(Query::Write(WriteQuery::DeleteRecord(DeleteRecord {
                model: Arc::clone(&child_model),
                record_filter: None,
            })));

            utils::insert_deletion_checks(graph, child_model, &find_child_records_node, &delete_record_node)?;

            graph.create_edge(
                 &find_child_records_node,
                 &delete_record_node,
                 QueryGraphDependency::ParentProjection(child_model_identifier, Box::new(move |mut delete_record_node, mut child_ids| {
                     let child_id = match child_ids.pop() {
                         Some(pid) => Ok(pid),
                         None => Err(QueryGraphBuilderError::AssertionError(format!(
                             "[Query Graph] Expected a valid parent ID to be present for a nested delete on a one-to-many relation."
                         ))),
                     }?;

                     if let Node::Query(Query::Write(WriteQuery::DeleteRecord(ref mut dq))) = delete_record_node {
                         dq.record_filter = Some(child_id.into());
                     }

                     Ok(delete_record_node)
                 })),
             )?;
        }
    }

    Ok(())
}

pub fn nested_delete_many(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().primary_identifier();

    for value in utils::coerce_vec(value) {
        let as_map: ParsedInputMap = value.try_into()?;
        let filter = extract_filter(as_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;

        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: Arc::clone(&child_model),
            record_filter: RecordFilter::empty(),
        });

        let delete_many_node = graph.create_node(Query::Write(delete_many));
        utils::insert_deletion_checks(graph, child_model, &find_child_records_node, &delete_many_node)?;

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ParentProjection(
                child_model_identifier.clone(),
                Box::new(move |mut delete_many_node, child_ids| {
                    if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_many_node {
                        dmr.record_filter = child_ids.into();
                    }

                    Ok(delete_many_node)
                }),
            ),
        )?;
    }

    Ok(())
}
