mod create_nested;
mod update_nested;

use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue,
};
use connector::filter::RecordFinder;
use create_nested::*;
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};
use update_nested::*;

pub fn connect_nested_query(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: RelationFieldRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();

    for (field_name, value) in data_map {
        match field_name.as_str() {
            "create" => connect_nested_create(graph, parent, &parent_relation_field, value, &child_model)?,
            "update" => connect_nested_update(graph, parent, &parent_relation_field, value, &child_model)?,
            "connect" => connect_nested_connect(graph, parent, &parent_relation_field, value, &child_model)?,
            "delete" => connect_nested_delete(graph, parent, &parent_relation_field, value, &child_model)?,
            _ => (),
        };
    }

    Ok(())
}

pub fn connect_nested_connect(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        // First, we need to build a read query on the record to be conneted.
        let record_finder = extract_record_finder(value, &model)?;
        let child_read_query = utils::id_read_query_infallible(&model, record_finder);
        let child_node = graph.create_node(child_read_query);

        // Flip the read node and parent node if necessary.
        let (parent, child, relation_field) = utils::flip_nodes(graph, parent, &child_node, relation_field);
        let relation_field_name = relation_field.name.clone();

        // Edge from parent to child (e.g. Create to ReadQuery).
        graph.create_edge(
            parent,
            child,
            QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!(
                        "Expected a valid parent ID to be present for nested connect pre read."
                    ))),
                }?;

                // If the child is a write query, inject the parent id.
                // This covers cases of inlined relations.
                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_non_list_arg(relation_field_name, parent_id)
                }

                Ok(child_node)
            })),
        );

        // Detect if a connect is actually necessary between the nodes.
        // A connect is necessary if the nested connect is done on a relation that
        // is a list (x-to-many) and where the relation is not inlined (aka manifested as an
        // actual join table, for example).
        if relation_field.is_list && !relation_field.relation().is_inline_relation() {
            connect::connect_records_node(graph, parent, child, &relation_field, None, None);
        }
    }

    Ok(())
}

/// Adds a delete (single) record node to the graph and connects it to the parent.
/// Auxiliary nodes may be added to support the deletion process, e.g. extra read nodes.
///
/// If the relation is a list:
/// - Delete specific record from the list, a record finder must be present in the data.
///
/// If the relation is not a list:
/// - Just delete the one node that can be present, if desired (as it is a non-list, aka 1-to-1 relation).
/// - The relation HAS to be inlined, because it is 1-to-1.
/// - If the relation is inlined in the parent, we need to generate a read query to grab the id of the record we want to delete.
/// - If the relation is inlined but not in the parent, we can directly generate a delete on the record with the parent ID.
pub fn connect_nested_delete(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        if relation_field.is_list {
            // Todo:
            // - we need to make sure the records are actually connected...
            // - What about the checks currently performed in `DeleteActions`?
            let record_finder = extract_record_finder(value, &model)?;
            let delete_node = delete::delete_record_node(graph, Some(record_finder), Arc::clone(&model));

            // graph.create_edge(parent, to: &NodeRef, content: QueryGraphDependency);
            unimplemented!()
        } else {
            // if relation_field.relation_is_inlined_in_parent() {
            //     let delete_node = delete::delete_record_node(graph, None, Arc::clone(&model));
            //     let find_child_records_node = utils::find_ids_by_parent(graph, relation_field, parent);

            //     None
            // } else {
            //     None
            // };

            let val: PrismaValue = value.try_into()?;
            match val {
                PrismaValue::Boolean(b) if b => unimplemented!(),
                // vec.push(NestedDeleteRecord {
                //     relation_field: Arc::clone(&relation_field),
                //     where_: None,
                // }),
                _ => (),
            };
        }
    }

    unimplemented!()
}
