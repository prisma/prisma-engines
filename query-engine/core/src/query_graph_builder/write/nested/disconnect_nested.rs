use super::*;
use crate::{
    inputs::UpdateManyRecordsSelectorsInput,
    query_graph::{NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue, RowSink,
};
use itertools::Itertools;
use query_structure::{Filter, Model, PrismaValue, RelationCompare, RelationFieldRef, SelectionResult, WriteArgs};
use std::convert::TryInto;

/// Handles nested disconnect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_disconnect(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    if relation.is_many_to_many() {
        // Build all filters upfront.
        let filters: Vec<Filter> = utils::coerce_vec(value)
            .into_iter()
            .map(|value: ParsedInputValue<'_>| {
                let value: ParsedInputMap<'_> = value.try_into()?;
                extract_unique_filter(value, child_model)
            })
            .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?
            .into_iter()
            .unique()
            .collect();

        handle_many_to_many(graph, &parent_node, parent_relation_field, Filter::or(filters))
    } else {
        let filter: Filter = if relation.is_one_to_one() {
            // One-to-one relations can simply specify if they want to disconnect the child or not as a bool.
            if let ParsedInputValue::Single(PrismaValue::Boolean(should_disconnect)) = value {
                if !should_disconnect {
                    return Ok(());
                }

                Filter::empty()
            } else {
                let value: ParsedInputMap<'_> = value.try_into()?;

                extract_filter(value, child_model)?
            }
        } else {
            // One-to-many specify a number of finders if the parent side is the to-one.
            // todo check if this if else is really still required.
            if parent_relation_field.is_list() {
                let filters = utils::coerce_vec(value)
                    .into_iter()
                    .map(|value: ParsedInputValue<'_>| {
                        let value: ParsedInputMap<'_> = value.try_into()?;
                        extract_unique_filter(value, child_model)
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
    if filter.size() == 0 {
        return Ok(());
    }

    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, filter)?;

    disconnect::disconnect_records_node(graph, parent_node, &find_child_records_node, parent_relation_field)?;
    Ok(())
}

/// Handles a nested one to many or one to one disconnect.
///
/// Depending on where the relation is inlined, an update node will be inserted:
/// (dashed indicates that those nodes and edges are not created in this function)
/// ```text
/// Inlined on parent:             Inlined on child:
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐            ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent                ┌──      Parent
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │           │                  │           │
/// │           │                  │           │
/// │           │                  │           │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// │  │  Find Children  │         │  │  Find Children  │
/// │  └─────────────────┘         │  └─────────────────┘
/// │           │                  │           │
/// │           │                  │           │
/// │           │                  │           │
/// │           ▼                  │           ▼
/// │  ┌─────────────────┐         │  ┌─────────────────┐
/// └─▶│  Update Parent  │         └─▶│ Update Children │
///    └─────────────────┘            └─────────────────┘
/// ```
///
/// Assumes that both `Parent` and `Child` return the necessary IDs.
///
/// Todo pretty sure it's better do redo this code with separate handlers.
fn handle_one_to_x(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    // Fetches the children to be disconnected.
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, filter.clone())?;

    let child_relation_field = parent_relation_field.related_field();

    // If we're in a 1:m scenario and either relation side is required, a disconnect is impossible, as some
    // relation requirement would be violated with the disconnect.
    if parent_relation_field.is_required() || child_relation_field.is_required() {
        return Err(QueryGraphBuilderError::RelationViolation(parent_relation_field.into()));
    }

    // Depending on where the relation is inlined, we update the parent or the child nodes.
    let (node_to_attach, model_to_update, extractor_model_id, null_record_id, filter) =
        if parent_relation_field.is_inlined_on_enclosing_model() {
            // Inlined on parent
            let parent_model = parent_relation_field.model();
            let extractor_model_id = parent_model.shard_aware_primary_identifier();
            let null_record_id = SelectionResult::from(&parent_relation_field.linking_fields());
            // If the relation is inlined on the parent and a filter is applied on the child then update is done on the parent table.
            // Therefore, the filter applied on the child needs to be converted to a "relational" filter so that the connector renders the adequate SQL to join the Child table.
            let filter = if !filter.is_empty() {
                parent_relation_field.to_one_related(filter)
            } else {
                filter
            };

            (parent_node, parent_model, extractor_model_id, null_record_id, filter)
        } else {
            // Inlined on child
            let child_model = child_relation_field.model();
            let extractor_model_id = child_model.shard_aware_primary_identifier();
            let null_record_id = SelectionResult::from(&child_relation_field.linking_fields());

            (
                &find_child_records_node,
                child_model,
                extractor_model_id,
                null_record_id,
                filter,
            )
        };

    let write_args = WriteArgs::from_result(null_record_id, crate::executor::get_request_now());
    let update_node = utils::update_records_node_placeholder_with_args(graph, filter, model_to_update, write_args);

    // Edge to inject the correct data into the update (either from the parent or child).
    graph.create_edge(
        node_to_attach,
        &update_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            extractor_model_id,
            RowSink::All(&UpdateManyRecordsSelectorsInput),
            None,
        ),
    )?;

    Ok(())
}
