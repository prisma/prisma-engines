use super::*;
use crate::{
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    FilteredQuery, InputAssertions, ParsedInputMap, ParsedInputValue, Query, WriteQuery,
};
use connector::{Filter, IdFilter};
use itertools::Itertools;
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::convert::TryInto;

/// Handles nested disconnect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_disconnect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    if relation.is_many_to_many() {
        // Build all filters upfront.
        let filters: Vec<Filter> = utils::coerce_vec(value)
            .into_iter()
            .map(|value: ParsedInputValue| {
                let value: ParsedInputMap = value.try_into()?;

                value.assert_size(1)?;
                value.assert_non_null()?;

                extract_unique_filter(value, &child_model)
            })
            .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
            .into_iter()
            .unique()
            .collect();

        handle_many_to_many(graph, &parent_node, parent_relation_field, Filter::or(filters))
    } else {
        let filter: Filter = if relation.is_one_to_one() {
            // One-to-one relations simply specify if they want to disconnect the child or not as a bool.
            let val: PrismaValue = value.try_into()?;
            let should_delete = if let PrismaValue::Boolean(b) = val { b } else { false };

            if !should_delete {
                return Ok(());
            }

            Filter::empty()
        } else {
            // One-to-many specify a number of finders if the parent side is the to-one.
            // todo check if this if else is really still required.
            if parent_relation_field.is_list {
                let filters = utils::coerce_vec(value)
                    .into_iter()
                    .map(|value: ParsedInputValue| {
                        let value: ParsedInputMap = value.try_into()?;

                        value.assert_size(1)?;
                        value.assert_non_null()?;

                        extract_unique_filter(value, &child_model)
                    })
                    .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
                    .into_iter()
                    .unique()
                    .collect();

                Filter::or(filters)
            } else {
                Filter::empty()
            }
        };

        handle_one_to_x(graph, &parent_node, parent_relation_field, filter)
    }
}

/// Handles a nested many-to-many disconnect.
///
/// Creates a disconnect node in the graph and creates edges to `parent_node` and `child_node`.
/// The disconnect edges assume that both the parent and the child node results
/// are convertible to IDs, as the edges perform a transformation on the disconnect node to
/// inject the required IDs after the parents executed.
///
/// The resulting graph:
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ─ ─ ─ ─ ─
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │
/// │           │
/// │                              │
/// │           │
/// │           ▼                  ▼
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐  ┌ ─ ─ ─ ─ ─ ─
/// │         Child             Result   │
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘  └ ─ ─ ─ ─ ─ ─
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// └─▶│   Disconnect    │
///    └─────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let expected_disconnects = std::cmp::max(filter.size(), 1);
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, filter)?;

    disconnect::disconnect_records_node(
        graph,
        parent_node,
        &find_child_records_node,
        &parent_relation_field,
        expected_disconnects,
    )?;

    Ok(())
}

/// Handles a nested one to many or one to one disconnect.
///
/// Depending on where the relation is inlined, an update node will be inserted:
/// (dashed indicates that those nodes and edges are not created in this function)
/// ```text
/// Inlined on parent:             Inlined on child:
///
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐            ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent                ┌──      Parent
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │           │                  │           │
/// │           │             Fail if !=       │
/// │           │              expected        │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// │  │  Find Children  │         │  │  Find Children  │
/// │  └─────────────────┘         │  └─────────────────┘
/// │      Fail if !=              │           │
/// │       expected               │           │
/// │           │                  │           │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// └─▶│  Update Parent  │         └─▶│ Update Children │
///    └─────────────────┘            └─────────────────┘
/// ```
///
/// Assumes that both `Parent` and `Child` return the necessary IDs.
/// We need to check that _both_ actually do return IDs to ensure that they're connected,
/// regardless of which ID is used in the end to perform the update.
///
/// Todo pretty sure it's better do redo this code with separate handlers.
fn handle_one_to_x(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let filter_size = filter.size();

    // Fetches the children to be disconnected.
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, filter)?;

    let child_relation_field = parent_relation_field.related_field();

    // If we're in a 1:m scenario and either relation side is required, a disconnect is impossible, as some
    // relation requirement would be violated with the disconnect.
    if parent_relation_field.is_required || child_relation_field.is_required {
        return Err(QueryGraphBuilderError::RelationViolation(parent_relation_field.into()));
    }

    // Depending on where the relation is inlined, we update the parent or the child and check the other one for ID presence.
    let (
        node_to_attach,
        node_to_check,
        model_to_update,
        expected_disconnects,
        extractor_model_id,
        null_record_id,
        check_model_id,
    ) = if parent_relation_field.is_inlined_on_enclosing_model() {
        let parent_model = parent_relation_field.model();
        let extractor_model_id = parent_model.primary_identifier();
        let null_record_id = parent_relation_field.linking_fields().empty_record_projection();
        let check_model_id = child_relation_field.model().primary_identifier();

        (
            parent_node,
            &find_child_records_node,
            parent_model,
            std::cmp::max(filter_size, 1),
            extractor_model_id,
            null_record_id,
            check_model_id,
        )
    } else {
        let child_model = child_relation_field.model();
        let extractor_model_id = child_model.primary_identifier();
        let null_record_id = child_relation_field.linking_fields().empty_record_projection();
        let check_model_id = parent_relation_field.model().primary_identifier();

        (
            &find_child_records_node,
            parent_node,
            child_model,
            1,
            extractor_model_id,
            null_record_id,
            check_model_id,
        )
    };

    let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), model_to_update);
    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    // Edge to inject the correct data into the update (either from the parent or child).
    graph.create_edge(
        node_to_attach,
        &update_node,
        QueryGraphDependency::ParentProjection(
            extractor_model_id,
            Box::new(move |mut update_node, links| {
                if links.len() == 0 {
                    return Err(QueryGraphBuilderError::RecordsNotConnected {
                        relation_name,
                        parent_name,
                        child_name,
                    });
                }

                // Handle filter & arg injection
                if let Node::Query(Query::Write(ref mut wq @ WriteQuery::UpdateManyRecords(_))) = update_node {
                    wq.set_filter(links.filter());
                    wq.inject_projection_into_args(null_record_id);
                };

                Ok(update_node)
            }),
        ),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_name = parent_relation_field.model().name.clone();
    let child_name = parent_relation_field.related_model().name.clone();

    // Edge to check that IDs have been returned.
    graph.create_edge(
        node_to_check,
        &update_node,
        QueryGraphDependency::ParentProjection(
            check_model_id,
            Box::new(move |update_node, ids| {
                if ids.len() != expected_disconnects {
                    return Err(QueryGraphBuilderError::RecordsNotConnected {
                        relation_name,
                        parent_name,
                        child_name,
                    });
                }

                Ok(update_node)
            }),
        ),
    )?;

    Ok(())
}
