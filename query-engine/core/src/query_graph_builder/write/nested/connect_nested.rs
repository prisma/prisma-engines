use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue, QueryResult,
};
use connector::{Filter, IntoFilter};
use itertools::Itertools;
use prisma_models::{ModelRef, RelationFieldRef};
use std::convert::TryInto;
use std::sync::Arc;

/// Handles nested connect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, value, child_model))]
pub fn nested_connect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    // Build all filters upfront.
    let filters: Vec<Filter> = utils::coerce_vec(value)
        .into_iter()
        .map(|value: ParsedInputValue| {
            let value: ParsedInputMap = value.try_into()?;
            extract_unique_filter(value, &child_model)
        })
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
        .into_iter()
        .unique()
        .collect();

    if !filters.is_empty() {
        let filter = Filter::or(filters);

        if relation.is_many_to_many() {
            handle_many_to_many(graph, parent_node, parent_relation_field, filter, child_model)
        } else if relation.is_one_to_many() {
            handle_one_to_many(graph, parent_node, parent_relation_field, filter, child_model)
        } else {
            handle_one_to_one(graph, parent_node, parent_relation_field, filter, child_model)
        }
    } else {
        Ok(())
    }
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
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, filter, child_model))]
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let expected_connects = filter.size();
    let child_read_query = utils::read_ids_infallible(child_model.clone(), child_model.primary_identifier(), filter);
    let child_node = graph.create_node(child_read_query);

    graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
    connect::connect_records_node(
        graph,
        &parent_node,
        &child_node,
        &parent_relation_field,
        expected_connects,
    )?;

    Ok(())
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
/// In case the relation is inlined in the child, we execute the parent operation first,
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
/// If the check fails a runtime error occurs.
///
/// We do not need to do relation requirement checks because the many side of the relation can't be required,
/// and if the one side is required it's automatically satisfied because the record to connect has to exist.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, child_filter, child_model))]
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    child_filter: Filter,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    if parent_relation_field.relation_is_inlined_in_parent() {
        let read_query = utils::read_ids_infallible(child_model.clone(), child_link.clone(), child_filter);
        let read_children_node = graph.create_node(read_query);
        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_relation_field.model().name.clone();
        let child_model_name = child_model.name.clone();

        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the IDs first.
        graph.mark_nodes(&parent_node, &read_children_node);

        graph.create_edge(
            &parent_node,
            &read_children_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_link,
                Box::new(move |mut read_children_node, mut child_links| {
                    let child_link = match child_links.pop() {
                        Some(cl) => Ok(cl),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record(s) (needed to inline the relation on '{}' record(s)) was found for a nested connect on one-to-many relation '{}'.",
                            child_model_name, parent_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = read_children_node {
                        wq.inject_result_into_args(parent_link.assimilate(child_link)?);
                    }

                    Ok(read_children_node)
                }),
            ),
        )?;
    } else {
        let expected_id_count = child_filter.size();
        let update_node = utils::update_records_node_placeholder(graph, child_filter, Arc::clone(child_model));
        let check_node = graph.create_node(Node::Empty);
        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_relation_field.model().name.clone();
        let child_model_name = child_model.name.clone();

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(
                parent_link,
                Box::new(move |mut update_node, mut parent_links| {
                    let parent_link = match parent_links.pop() {
                        Some(pl) => Ok(pl),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record(s) (needed to inline the relation on '{}' record(s)) was found for a nested connect on one-to-many relation '{}'.",
                            parent_model_name, child_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_result_into_args(child_link.assimilate(parent_link)?)
                    }

                    Ok(update_node)
                }),
            ),
        )?;

        let relation_name = parent_relation_field.relation().name.clone();

        // Check that all specified children have been updated.
        graph.create_edge(
            &update_node,
            &check_node,
            QueryGraphDependency::DataDependency(Box::new(move |check_node, parent_result| {
                let query_result = parent_result.as_query_result().unwrap();

                if let QueryResult::Count(c) = query_result {
                    if c != &expected_id_count {
                        return Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "Expected {} records to be connected after connect operation on one-to-many relation '{}', found {}.",
                            expected_id_count, relation_name, c,
                        )));
                    }
                }

                Ok(check_node)
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
/// ├─▶│      Update Child      │  (if inlined in the child)
/// │  └────────────────────────┘
/// │
/// │  ┌────────────────────────┐
/// └─▶│     Update Parent      │  (if inlined in the parent and non-create)
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
/// Important: We cannot inject from `Read New Child` to `Parent` if `Parent` is a non-create, as it would cause
/// the following issue (example):
/// - Parent is an update, doesn't have a connected child on relation x.
/// - Parent gets injected with a child on x, because that's what the connect is supposed to do.
/// - The update runs, the relation is updated.
/// - Now the check runs, because it's dependent on the parent's ID... but the check finds an existing child and fails...
/// ... because we just updated the relation.
///
/// This is why we need to have an extra update at the end if it's inlined on the parent and a non-create.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, filter, child_model))]
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_linking_fields = parent_relation_field.linking_fields();
    let child_linking_fields = parent_relation_field.related_field().linking_fields();

    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required();
    let child_side_required = child_relation_field.is_required();
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let relation_inlined_child = !relation_inlined_parent;

    // Build-time check
    if parent_side_required && child_side_required {
        // Both sides are required, which means that we know that there has to be already a parent connected to the child (as it exists).
        // A connect to the child would disconnect the other parent connection, violating the required side of the existing parent.
        return Err(QueryGraphBuilderError::RelationViolation(
            (parent_relation_field).into(),
        ));
    }

    let read_query = utils::read_ids_infallible(child_model.clone(), child_linking_fields.clone(), filter);
    let read_new_child_node = graph.create_node(read_query);

    // We always start with the read node in a nested connect 1:1 scenario.
    graph.mark_nodes(&parent_node, &read_new_child_node);

    // Next is the check for (and possible disconnect of) an existing parent.
    // Those checks are performed on the new child node, hence we use the child relation field side ("backrelation").
    if parent_side_required || relation_inlined_parent {
        utils::insert_existing_1to1_related_model_checks(graph, &read_new_child_node, &child_relation_field)?;
    }

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_model_name = parent_relation_field.model().name.clone();
    let child_model_name = child_model.name.clone();

    graph.create_edge(
        &parent_node,
        &read_new_child_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_linking_fields.clone(),
            Box::new(move |mut read_new_child_node, mut child_links| {
                // This takes care of cases where the relation is inlined, CREATE ONLY. See doc comment for explanation.
                if relation_inlined_parent && parent_is_create {
                    let child_link = match child_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record (needed to inline connect on create for '{}' record) was found for a nested connect on one-to-one relation '{}'.",
                            child_model_name, parent_model_name, relation_name
                        ))),
                    }?;


                    if let Node::Query(Query::Write(ref mut wq)) = read_new_child_node {
                        wq.inject_result_into_args(parent_linking_fields.assimilate(child_link)?);
                    }
                }

                Ok(read_new_child_node)
            }),
        ),
    )?;

    // Finally, insert the check for (and possible disconnect of) an existing child record.
    // Those checks are performed on the parent node model.
    // We only need to do those checks if the parent operation is not a create, the reason being that
    // if the parent is a create, it can't have an existing child already.
    if !parent_is_create && (child_side_required || !relation_inlined_parent) {
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, parent_relation_field)?;
    }

    // If the relation is inlined on the child, we also need to update the child to connect it to the parent.
    if relation_inlined_child {
        let update_children_node =
            utils::update_records_node_placeholder(graph, Filter::empty(), Arc::clone(child_model));

        let parent_linking_fields = parent_relation_field.linking_fields();
        let child_linking_fields = parent_relation_field.related_field().linking_fields();
        let child_model_identifier = parent_relation_field.related_field().model().primary_identifier();
        let relation_name = parent_relation_field.relation().name.clone();
        let child_model_name = child_model.name.clone();

        graph.create_edge(
            &read_new_child_node,
            &update_children_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier,
                Box::new(move |mut update_children_node, mut child_ids| {
                    let child_id = match child_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record to connect was found was found for a nested connect on one-to-one relation '{}'.",
                            child_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_children_node {
                        wq.add_filter(child_id.filter());
                    }

                    Ok(update_children_node)
                }),
            ),
        )?;

        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_relation_field.model().name.clone();
        let child_model_name = child_model.name.clone();

        graph.create_edge(
             &parent_node,
             &update_children_node,
             QueryGraphDependency::ProjectedDataDependency(parent_linking_fields, Box::new(move |mut update_children_node, mut parent_links| {
                 let parent_link = match parent_links.pop() {
                     Some(link) => Ok(link),
                     None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to update inlined relation on '{}') was found for a nested connect on one-to-one relation '{}'.",
                        parent_model_name,
                        child_model_name,
                        relation_name
                    ))),
                 }?;

                 if let Node::Query(Query::Write(ref mut wq)) = update_children_node {
                     wq.inject_result_into_args(child_linking_fields.assimilate(parent_link)?);
                 }

                 Ok(update_children_node)
             })),
         )?;
    } else if relation_inlined_parent && !parent_is_create {
        // Relation is inlined on the parent and a non-create.
        // Create an update node for parent record to set the connection to the child.
        let parent_model = parent_relation_field.model();
        let parent_model_name = parent_model.name.clone();
        let child_model_name = child_model.name.clone();
        let update_parent_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model.clone());
        let parent_linking_fields = parent_relation_field.linking_fields();
        let relation_name = parent_relation_field.relation().name.clone();

        graph.create_edge(
            &read_new_child_node,
            &update_parent_node,
            QueryGraphDependency::ProjectedDataDependency(child_linking_fields, Box::new(move |mut update_parent_node, mut child_links| {
                let child_link = match child_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to update inlined relation on '{}') was found for a nested connect on one-to-one relation '{}'.",
                        parent_model_name,
                        child_model_name,
                        relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_parent_node {
                    wq.inject_result_into_args(parent_linking_fields.assimilate(child_link)?);
                }

                Ok(update_parent_node)
            })),
         )?;

        let parent_model_identifier = parent_relation_field.model().primary_identifier();
        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_model.name.clone();
        let child_model_name = child_model.name.clone();

        graph.create_edge(
            &parent_node,
            &update_parent_node,
            QueryGraphDependency::ProjectedDataDependency(parent_model_identifier, Box::new(move |mut update_parent_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to update inlined relation on '{}') was found for a nested connect on relation '{}'.",
                        parent_model_name,
                        child_model_name,
                        relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_parent_node {
                    wq.add_filter(parent_id.filter());
                }

                Ok(update_parent_node)
            })),
        )?;
    }

    Ok(())
}
