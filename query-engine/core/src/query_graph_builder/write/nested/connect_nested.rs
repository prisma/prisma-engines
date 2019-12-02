use super::*;
use crate::{
    query_ast::*,
    query_document::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, QueryResult,
};
use connector::{Filter, ScalarCompare};
use itertools::Itertools;
use prisma_models::{ModelRef, RelationFieldRef};
use std::sync::Arc;

/// Handles nested connect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn connect_nested_connect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();
    let values = utils::coerce_vec(value);

    // Build all finders upfront.
    // let filters: Vec<Filter> = utils::coerce_vec(value)
    //     .into_iter()
    //     .unique()
    //     .map(|value: ParsedInputValue| extract_filter(value.try_into()?, &child_model)?.assert_size(1)?)
    //     .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
    //     .into_iter()
    //     .collect();

    // if relation.is_many_to_many() {
    //     handle_many_to_many(graph, parent_node, parent_relation_field, finders, child_model)
    // } else if relation.is_one_to_many() {
    //     handle_one_to_many(graph, parent_node, parent_relation_field, finders, child_model)
    // } else {
    //     handle_one_to_one(graph, parent_node, parent_relation_field, finders, child_model)
    // }
    //
    unimplemented!()
}

/// Handles a many-to-many nested connect.
/// This is the least complicated case, as it doesn't involve
/// checking for relation violations or updating inlined relations.
///
///```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ─ ─ ─ ─ ─ ┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │           │                   │
/// │
/// │           │                   │
/// │           ▼                   ▼
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐   ┌ ─ ─ ─ ─ ─ ─
/// │         Child              Result   │
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘   └ ─ ─ ─ ─ ─ ─
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// └─▶│     Connect     │
///    └─────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    unimplemented!()
    // let child_read_query = utils::read_ids_infallible(&child_model, finders);
    // let child_node = graph.create_node(child_read_query);

    // graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
    // connect::connect_records_node(
    //     graph,
    //     &parent_node,
    //     &child_node,
    //     &parent_relation_field,
    //     expected_connects,
    // )?;

    // Ok(())
}

/// Handles a one-to-many nested connect.
/// There are two cases: Either the relation side is inlined on the parent or the child.
/// It is always assumed that side of inlining is the many side. This means that the operation
/// coming first (as shown in the graphs below) can only ever return one record, to be injected into all
/// records returned from the second operation.
///
/// If the relation is inlined in the parent, we need to create a graph that has a read node
/// for the child first and then the parent operation to have the child ID ready:
/// ```text
/// ┌─────────────────┐
/// │  Read Children  │
/// └─────────────────┘
///          │
///          │
///          ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///       Parent
/// └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///          │
///
///          ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///       Result
/// └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// ```
/// The ID of the child is injected into the parent operation. This can be more than one record getting updated.
///
/// ---
///
/// In case the relation is inline in the child, we can have the parent execute first,
/// then do an update on the child to insert the parent ID into the inline relation field.
/// ```text
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///       Parent       ─ ─ ─ ─ ─ ┐
/// └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///          │                   │
///          │
///          ▼                   ▼
/// ┌─────────────────┐   ┌ ─ ─ ─ ─ ─ ─
/// │ Update Children │       Result   │
/// └─────────────────┘   └ ─ ─ ─ ─ ─ ─
///          │
///          │ Check
///          ▼
/// ┌─────────────────┐
/// │      Empty      │
/// └─────────────────┘
/// ```
/// The ID of the parent is injected into the child operation. This can be more than one record getting updated.
///
/// Checks are performed to ensure that the correct number of records got connected.
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut child_finders: Vec<RecordFinder>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    if parent_relation_field.relation_is_inlined_in_parent() {
        let read_query = utils::read_ids_infallible(&child_model, child_finders.pop());
        let child_node = graph.create_node(read_query);

        // For the injection, we need the name of the field on the inlined side, in this case the parent.
        let relation_field_name = parent_relation_field.name.clone();

        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the IDs first.
        graph.mark_nodes(&parent_node, &child_node);

        graph.create_edge(
                &parent_node,
                &child_node,
                QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
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
    } else {
        let expected_id_count = child_finders.len();
        let update_node = utils::update_records_node_placeholder(graph, child_finders, Arc::clone(child_model));
        let check_node = graph.create_node(Node::Empty);

        // For the injection, we need the name of the field on the inlined side, in this case the child.
        let relation_field_name = parent_relation_field.related_field().name.clone();

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
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

        // Check that all specified children have been updated.
        graph.create_edge(
            &update_node,
            &check_node,
            QueryGraphDependency::ParentResult(Box::new(move |node, parent_result| {
                let query_result = parent_result.as_query_result().unwrap();

                if let QueryResult::Count(c) = query_result {
                    if c != &expected_id_count {
                        return Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "Expected {} records to be connected, found {}.",
                            expected_id_count, c,
                        )));
                    }
                }

                Ok(node)
            })),
        )?;
    };

    Ok(())
}

/// Handles a one-to-one nested connect.
/// Most complex case as there are plenty of cases involved where we need to make sure
/// that we don't violate relation requirements.
///
/// The full graph that can be created by this handler looks like this:
/// ```text
///    ┌────────────────────────┐
/// ┌──│     Read New Child     │───────┐
/// │  └────────────────────────┘       │
/// │               │                   │
/// │               │                   │
/// │               │    ┌─── ──── ──── ▼─── ──── ──── ──── ──── ──── ──── ──── ─┐
/// │               │    │ ┌────────────────────────┐                            │
/// │               │    │ │    Read ex. Parent     │──┐                         │
/// │               │    │ └────────────────────────┘  │
/// │               │    │              │              │                         │
/// │               │                   ▼              │(Fail on p > 0 if parent │
/// │               │    │ ┌────────────────────────┐  │     side required)      │
/// │               │    │ │ If p > 0 && p. inlined │  │                         │
/// │               │    │ └────────────────────────┘  │
/// │               │    │              │              │                         │
/// │               │                   ▼              │                         │
/// │               │    │ ┌────────────────────────┐  │                         │
/// │               │    │ │   Update ex. parent    │◀─┘                         │
/// │               │    │ └────────────────────────┘                      ┌───┐
/// │               │    │         then                                    │ 1 │ │
/// │               │                                                      └───┘ │
/// │               ▼    └─── ──── ──── ──── ──── ──── ──── ──── ──── ──── ──── ─┘
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ├──          Parent         │───────┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─        │
/// │               │                   │
/// │                    ┌───  ────  ───┼  ────  ────  ────  ────  ────  ────  ──┐
/// │               │                   ▼                                        │
/// │                      ┌────────────────────────┐
/// │               │    │ │     Read ex. child     │──┐
/// │                    │ └────────────────────────┘  │                         │
/// │               │    │              │              │                         │
/// │                    │              ▼              │(Fail on c > 0 if child  │
/// │               │      ┌────────────────────────┐  │     side required)      │
/// │                      │ If c > 0 && c. inlined │  │
/// │               │    │ └────────────────────────┘  │
/// │                    │         then │              │                         │
/// │               │    │              ▼              │                         │
/// │                    │ ┌────────────────────────┐  │                         │
/// │               │      │    Update ex. child    │◀─┘                   ┌───┐ │
/// │                      └────────────────────────┘                      │ 2 │
/// │               │    │                                                 └───┘
/// │               ▼    └──  ────  ────  ────  ────  ────  ────  ────  ────  ───┘
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │         Read Result       │
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │
/// │  ┌────────────────────────┐
/// ├─▶│      Update Child      │  (if inlined on the child)
/// │  └────────────────────────┘
/// │
/// │  ┌────────────────────────┐
/// └─▶│     Update Parent      │  (if inlined on the parent and non-create)
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
///
/// Important: We can not inject from `Read New Child` to `Parent` if `Parent` is a non-create, as it would cause
/// the following issue (example):
/// - Parent is an update, doesn't have a connected child on relation x.
/// - Parent gets injected with a child on x, because that's what the connect is supposed to do.
/// - The update runs, the relation is updated.
/// - Now the check runs, because it's dependent on the parent's ID... but the check finds an existing child and fails...
/// ... because we just updated the relation.
///
/// This is why we need to have an extra update at the end if it's inlined on the parent and a non-create.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut finders: Vec<RecordFinder>,
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

    let record_finder = finders.pop();
    let read_query = utils::read_ids_infallible(&child_model, record_finder);
    let read_new_child_node = graph.create_node(read_query);

    // We always start with the read node in a nested connect 1:1 scenario.
    graph.mark_nodes(&parent_node, &read_new_child_node);

    // Next is the check for (and possible disconnect of) an existing parent.
    // Those checks are performed on the new child node, hence we use the child relation field side ("backrelation").
    if parent_side_required || relation_inlined_parent {
        utils::insert_existing_1to1_related_model_checks(graph, &read_new_child_node, &child_relation_field)?;
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

            // This takes care of cases where the relation is inlined, CREATE ONLY. See doc comment for explanation.
            if relation_inlined_parent && parent_is_create {
                if let Node::Query(Query::Write(ref mut wq)) = child_node {
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
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, parent_relation_field)?;
    }

    // If the relation is inlined on the child, we also need to update the child to connect it to the parent.
    if !relation_inlined_parent {
        let update_node = utils::update_records_node_placeholder(graph, None, Arc::clone(child_model));
        let relation_field_name = child_relation_field.name.clone();
        let child_model_id = child_model.fields().id();

        graph.create_edge(
            &read_new_child_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating inlined on child."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.add_filter(child_model_id.equals(parent_id));
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
    } else if relation_inlined_parent && !parent_is_create {
        // Relation is inlined on the Parent and a non-create.
        // Create an update node for Parent to set the connection to the child.
        let parent_model = parent_relation_field.model();
        let relation_field_name = parent_relation_field.name.clone();
        let parent_model_id = parent_model.fields().id();
        let update_node = utils::update_records_node_placeholder(graph, None, parent_model);

        graph.create_edge(
            &read_new_child_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_non_list_arg(relation_field_name, parent_id);
                }

                Ok(child_node)
            })),
        )?;

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.add_filter(parent_model_id.equals(parent_id));
                }

                Ok(child_node)
            })),
        )?;
    }

    Ok(())
}
