use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::sync::Arc;

/// Handles nested connect cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn connect_nested_connect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let relation = parent_relation_field.relation();

        if relation.is_many_to_many() {
            handle_many_to_many(graph, parent_node, parent_relation_field, value, child_model)?;
        } else if relation.is_one_to_many() {
            handle_one_to_many(graph, parent_node, parent_relation_field, value, child_model)?;
        } else {
            handle_one_to_one(graph, parent_node, parent_relation_field, value, child_model)?;
        }
    }

    Ok(())
}

/// Handles a many-to-many nested connect.
/// This is the least complicated case, as it doesn't involve
/// checking for relation violations or updating inlined relations.
///
/// (illustration simplified, `Parent` / `Result` exemplary)
///
///```text
///    ┌────────────┐
/// ┌──│   Parent   │───────┐
/// │  └────────────┘       │
/// │         │             │
/// │         ▼             ▼
/// │  ┌────────────┐   ┌──────┐
/// │  │ Read Child │   │Result│
/// │  └────────────┘   └──────┘
/// │         │
/// │         ▼
/// │  ┌────────────┐
/// └─▶│  Connect   │
///    └────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let record_finder = extract_record_finder(value, &child_model)?;
    let child_read_query = utils::id_read_query_infallible(&child_model, record_finder);
    let child_node = graph.create_node(child_read_query);

    graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
    connect::connect_records_node(graph, &parent_node, &child_node, &parent_relation_field, None, None)?;

    Ok(())
}

/// Handles a one-to-many nested connect.
/// There are two cases: Either the relation side is inlined on the parent or the child.
///
/// (illustrations simplified, `Parent` / `Result` exemplary)
///
/// In case of the parent, we need to create a graph that has a read node
/// for the child first and then the parent operation to have the child ID ready:
///
/// ```text
/// ┌────────────┐
/// │ Read Child │
/// └────────────┘
///        │
///        ▼
/// ┌────────────┐
/// │   Parent   │
/// └────────────┘
///        │
///        ▼
/// ┌────────────┐
/// │   Result   │
/// └────────────┘
/// ```
///
/// In case of the child, we can have the parent first, then do an update on the child to
/// insert the parent ID into the inline relation field.
/// ```text
/// ┌────────────┐
/// │   Parent   │─────────┐
/// └────────────┘         │
///        │               │
///        ▼               ▼
/// ┌────────────┐  ┌────────────┐
/// │Update Child│  │   Result   │
/// └────────────┘  └────────────┘
/// ```
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let record_finder = extract_record_finder(value, &child_model)?;
    let (parent_node, child_node, relation_field_name) = if parent_relation_field.relation_is_inlined_in_parent() {
        let read_query = utils::id_read_query_infallible(&child_model, record_finder);
        let child_node = graph.create_node(read_query);

        // For the injection, we need the name of the field on the inlined side, in this case the parent.
        let relation_field_name = parent_relation_field.name.clone();

        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the ID first.
        graph.mark_nodes(&parent_node, &child_node);
        // let (parent_node, child_node) = utils::swap_nodes(graph, parent_node, child_node)?;

        (parent_node, child_node, relation_field_name)
    } else {
        let update_node = utils::update_record_node_placeholder(graph, Some(record_finder), Arc::clone(child_model));

        // For the injection, we need the name of the field on the inlined side, in this case the child.
        let relation_field_name = parent_relation_field.related_field().name.clone();

        (parent_node, update_node, relation_field_name)
    };

    graph.create_edge(
        &parent_node,
        &child_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-many relation."
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.inject_non_list_arg(relation_field_name, parent_id);
            }

            Ok(child_node)
        })),
    )?;

    Ok(())
}

/// Handles a one-to-one nested connect.
/// Most complex case as there are plenty of cases involved where we need to make sure
/// that we don't violate relation requirements.
///
/// The full graph that can be created by this handler looks like this:
/// (Either [1] or [2] are in the graph at the same time, not both)
/// ```text
///    ┌────────────────────────┐
/// ┌──│     Read New Child     │───────┐
/// │  └────────────────────────┘       │
/// │               │                   │
/// │               │    ┌ ─ ─ ─ ─ ─ ─ ─▼─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// │               │      ┌────────────────────────┐
/// │               │    │ │    Read ex. Parent     │──┐                         │
/// │               │      └────────────────────────┘  │
/// │               │    │              │              │                         │
/// │               │                   ▼              │(Fail on p > 0 if parent
/// │               │    │ ┌────────────────────────┐  │     side required)      │
/// │               │      │ If p > 0 && p. inlined │  │
/// │               │    │ └────────────────────────┘  │                         │
/// │               │                   │              │
/// │               │    │              ▼              │                         │
/// │               │      ┌────────────────────────┐  │
/// │               │    │ │   Update ex. parent    │◀─┘                         │
/// │               │      └────────────────────────┘                      ┌───┐
/// │               │    │                                                 │ 1 │ │
/// │               │                                                      └───┘
/// │               ▼    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │  ┌────────────────────────┐
/// ├──│    Parent operation    │───────┐
/// │  └────────────────────────┘       │
/// │               │                   │
/// │               │    ┌ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// │               │                   ▼
/// │               │    │ ┌────────────────────────┐                            │
/// │               │      │     Read ex. child     │──┐
/// │               │    │ └────────────────────────┘  │                         │
/// │               │                   │              │
/// │               │    │              ▼              │(Fail on c > 0 if child  │
/// │               │      ┌────────────────────────┐  │     side required)
/// │               │    │ │ If c > 0 && c. inlined │  │                         │
/// │               │      └────────────────────────┘  │
/// │               │    │         then │              │                         │
/// │               │                   ▼              │
/// │               │    │ ┌────────────────────────┐  │                         │
/// │               │      │    Update ex. child    │◀─┘                   ┌───┐
/// │               │    │ └────────────────────────┘                      │ 2 │ │
/// │               │                                                      └───┘
/// │               ▼    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │  ┌────────────────────────┐
/// │  │      Read Result       │
/// │  └────────────────────────┘
/// │
/// │  ┌────────────────────────┐
/// └─▶│      Update Child      │ (if inlined on the child)
///    └────────────────────────┘
/// ```
/// Where [1] and [2] are checks and disconnects inserted into the graph based
/// on the requirements of the relation connecting the parent and child models.
///
/// [1]: Checks and disconnects an existing parent. This block is necessary if:
/// - The parent side is required, to make sure that a connect does not violate those requirements
///   when disconnecting an already connected parent.
/// - The relation is inlined on the parent record. Even if the parent side is not required, we then need
///   to update the previous parent to not point to the child anymore ("disconnect").
///
/// [2]: Checks and disconnects an existing child. This block is necessary if the parent is not a create and:
/// - The child side is required, to make sure that a connect does not violate those requirements
///   when disconnecting an already connected child.
/// - The relation is inlined on the child record. Even if the child side is not required, we then need
///   to update the previous child to not point to the parent anymore ("disconnect").
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required;
    let child_side_required = child_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();

    // Build-time check
    if parent_side_required && child_side_required {
        // Both sides are required, which means that we know that there has to be already a parent connected to the child (as it exists).
        // A connect to the child would disconnect the other parent connection, violating the required side of the existing parent.
        return Err(QueryGraphBuilderError::RelationViolation(
            (parent_relation_field).into(),
        ));
    }

    let record_finder = extract_record_finder(value, &child_model)?;
    let read_query = utils::id_read_query_infallible(&child_model, record_finder);
    let read_new_child_node = graph.create_node(read_query);

    // We always start with the read node in a nested connect 1:1 scenario.
    graph.mark_nodes(&parent_node, &read_new_child_node);

    // Next is the check for (and possible disconnect of) an existing parent.
    // Those checks are performed on the new child node, hence we use the child relation field side ("backrelation").
    if parent_side_required || relation_inlined_parent {
        utils::insert_existing_1to1_related_model_checks(graph, &read_new_child_node, &child_relation_field, false)?;
    }

    let relation_field_name = if relation_inlined_parent {
        parent_relation_field.name.clone()
    } else {
        child_relation_field.name.clone()
    };

    graph.create_edge(
        &parent_node,
        &read_new_child_node,
        QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation."))),
            }?;

            // This takes care of cases where the relation is inlined.
            if relation_inlined_parent {
                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    info!("Injecting Parent -> ReadNewChild, Field: {}, Parent ID: {:?}, Evaluated child query: {:?}", relation_field_name, parent_id, wq);
                    wq.inject_non_list_arg(relation_field_name, parent_id);
                }
            }

            Ok(child_node)
        })),
    )?;

    // Finally, insert the check for (and possible disconnect of) an existing child record.
    // Those checks are performed on the parent node model.
    // We only need to do those checks if the parent operation is not a create, the reason being that
    // if the parent is a create, it can't have an existing child already.
    if !parent_is_create && (child_side_required || !relation_inlined_parent) {
        dbg!(">>>>>>>>>>>>>>>>>>>>>>>>>>>><<<<<<<<<<<<<<<<<<<<<<<<<<<<<");
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, parent_relation_field, false)?;
    }

    // If the relation is inlined on the child, we also need to update the child to connect it to the parent.
    if !relation_inlined_parent {
        let update_node = utils::update_record_node_placeholder(graph, None, Arc::clone(child_model));
        let relation_field_name = child_relation_field.name.clone();
        let child_model_id = child_model.fields().id();

        graph.create_edge(
            &read_new_child_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating inlined on child."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_record_finder(RecordFinder {
                        field: child_model_id,
                        value: parent_id,
                    });
                }

                Ok(child_node)
            })),
        )?;

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating inlined on child."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_non_list_arg(relation_field_name, parent_id);
                }

                Ok(child_node)
            })),
        )?;
    }

    Ok(())
}
