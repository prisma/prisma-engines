use super::*;
use crate::{
    ParsedInputValue,
    inputs::{
        IfInput, LeftSideDiffInput, RightSideDiffInput, UpdateManyRecordsSelectorsInput, UpdateOrCreateArgsInput,
    },
    query_ast::*,
    query_graph::*,
};
use itertools::Itertools;
use query_structure::{Filter, Model, RelationFieldRef, SelectionResult, WriteArgs};
use std::convert::TryInto;

/// Only for x-to-many relations.
///
/// Handles nested set cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_set(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

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

    let filter = Filter::or(filters);

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, filter)
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, filter)
    } else {
        panic!("Set is not supported on one-to-one relations.");
    }
}

/// Handles a set on a many-to-many relation.
///
/// The resulting graph:
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌──      Parent       ──┬ ─ ─ ─ ─ ┐
/// │  └ ─ ─ ─ ─ ─ ─ ─ ─ ┘  │
/// │           │           │         │
/// │           │           │
/// │           │           │         │
/// │           ▼           │         ▼
/// │  ┌─────────────────┐  │  ┌ ─ ─ ─ ─ ─ ─ ┐
/// │  │Read old children│  │      Result
/// │  └─────────────────┘  │  └ ─ ─ ─ ─ ─ ─ ┘
/// │           │           │
/// │           │           │
/// │           │           │
/// │           ▼           │
/// │  ┌─────────────────┐  │
/// │  │   Disconnect    │◀─┘
/// │  └─────────────────┘
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// │  │Read new children│
/// │  └─────────────────┘
/// │           │
/// │           │
/// │           │
/// │           ▼
/// │  ┌─────────────────┐
/// └─▶│     Connect     │
///    └─────────────────┘
/// ```
///
/// Connects only happen if the query specifies at least one record to be connected.
/// If none are specified, set effectively acts as a "disconnect all".
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let parent_model_identifier = parent_relation_field.model().shard_aware_primary_identifier();
    let child_model = parent_relation_field.related_model();
    let child_model_identifier = child_model.shard_aware_primary_identifier();
    let read_old_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    let disconnect = WriteQuery::DisconnectRecords(DisconnectRecords {
        parent_id: None,
        child_ids: vec![],
        relation_field: parent_relation_field.clone(),
    });

    let disconnect_node = graph.create_node(Query::Write(disconnect));

    // Edge from parent to disconnect
    graph.create_edge(
        parent_node,
        &disconnect_node,
        QueryGraphDependency::ProjectedDataDependency(
            parent_model_identifier,
            Box::new(move |mut disconnect_node, mut parent_ids| {
                let parent_id = parent_ids.pop().expect("parent id should be present");

                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                    c.parent_id = Some(parent_id);
                }

                Ok(disconnect_node)
            }),
            Some(DataExpectation::non_empty_rows(
                MissingRelatedRecord::builder()
                    .model(&parent_relation_field.model())
                    .relation(&parent_relation_field.relation())
                    .needed_for(DependentOperation::disconnect_records())
                    .operation(DataOperation::NestedSet)
                    .build(),
            )),
        ),
    )?;

    // Edge from read to disconnect.
    graph.create_edge(
        &read_old_node,
        &disconnect_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(|mut disconnect_node, child_ids| {
                // todo: What if there are no connected nodes to disconnect?
                if let Node::Query(Query::Write(WriteQuery::DisconnectRecords(ref mut c))) = disconnect_node {
                    c.child_ids = child_ids;
                }

                Ok(disconnect_node)
            }),
            None,
        ),
    )?;

    if filter.size() > 0 {
        let expected_connects = filter.size();
        let read_new_query = utils::read_ids_infallible(child_model, child_model_identifier, filter);
        let read_new_node = graph.create_node(read_new_query);

        graph.create_edge(&disconnect_node, &read_new_node, QueryGraphDependency::ExecutionOrder)?;

        connect::connect_records_node(
            graph,
            parent_node,
            &read_new_node,
            parent_relation_field,
            expected_connects,
        )?;
    }

    Ok(())
}

/// Handles a nested many-to-one set scenario.
/// Set only works on lists.
/// This implies that `parent` can only ever be the to-one side, and child can only be the many (inlined) side.
///
/// ```text
///      ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// ┌────      Parent       ─ ─ ─ ─ ─ ─ ┐
/// │    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │             │                     │
/// │             │
/// │             │                     │
/// │             ▼                     ▼
/// │    ┌─────────────────┐     ┌ ─ ─ ─ ─ ─ ─ ┐
/// │ ┌──│Read old children│         Result
/// │ │  └─────────────────┘     └ ─ ─ ─ ─ ─ ─ ┘
/// │ │           │
/// │ │           │
/// │ │           │
/// │ │           ▼
/// │ │  ┌─────────────────┐
/// │ │  │Read new children│
/// │ │  └─────────────────┘
/// │ │           │
/// │ │           │
/// │ │           │
/// │ │           ▼
/// │ │  ┌─────────────────┐
/// │ └─▶│      Diff       │──────────────────────────┐
/// │    └─────────────────┘                          │
/// │             │                                   │
/// │ ┌───────────┼───────────────────────┐           │
/// │ │           │                       │           │
/// │ │           ▼                       ▼           │
/// │ │  ┌─────────────────┐     ┌─────────────────┐  │
/// │ │  │  If (left > 0)  │     │ If (right > 0)  │  │
/// │ │  └─────────────────┘     └─────────────────┘  │
/// │ │           │                       │           │
/// │ │           │                       │           │
/// │ │           │                       │           │
/// │ │           ▼                       ▼           │
/// │ │  ┌─────────────────┐     ┌─────────────────┐  │
/// │ │  │ Update children │     │ Update children │  │
/// └─┴─▶│   ("connect")   │     │ ("disconnect")  │◀─┘
///      └─────────────────┘     └─────────────────┘
/// ```
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: Filter,
) -> QueryGraphBuilderResult<()> {
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();
    let child_link = parent_relation_field.related_field().linking_fields();
    let parent_link = parent_relation_field.linking_fields();
    let empty_child_link = SelectionResult::from(&child_link);

    let child_model = parent_relation_field.related_model();
    let read_old_node =
        utils::insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    let read_new_query = utils::read_ids_infallible(child_model.clone(), child_model_identifier.clone(), filter);
    let read_new_node = graph.create_node(read_new_query);
    let diff_left_to_right_node = graph.create_node(Node::Computation(Computation::empty_diff_left_to_right()));
    let diff_right_to_left_node = graph.create_node(Node::Computation(Computation::empty_diff_right_to_left()));

    graph.create_edge(&read_old_node, &read_new_node, QueryGraphDependency::ExecutionOrder)?;

    // The new IDs that are not yet connected will be on the `left` side of the diff.
    graph.create_edge(
        &read_new_node,
        &diff_left_to_right_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier.clone(),
            RowSink::All(&LeftSideDiffInput),
            None,
        ),
    )?;
    graph.create_edge(
        &read_new_node,
        &diff_right_to_left_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier.clone(),
            RowSink::All(&LeftSideDiffInput),
            None,
        ),
    )?;

    // The old IDs that must be disconnected will be on the `right` side of the diff.
    graph.create_edge(
        &read_old_node,
        &diff_left_to_right_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier.clone(),
            RowSink::All(&RightSideDiffInput),
            None,
        ),
    )?;
    graph.create_edge(
        &read_old_node,
        &diff_right_to_left_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier.clone(),
            RowSink::All(&RightSideDiffInput),
            None,
        ),
    )?;

    // Update (connect) case: Check left diff IDs
    let connect_if_node = graph.create_node(Node::Flow(Flow::if_non_empty()));
    let update_connect_node = utils::update_records_node_placeholder(graph, Filter::empty(), child_model.clone());

    graph.create_edge(
        &diff_left_to_right_node,
        &connect_if_node,
        QueryGraphDependency::ProjectedDataSinkDependency(child_model_identifier.clone(), RowSink::All(&IfInput), None),
    )?;

    // Connect to the if node, the parent node (for the inlining ID) and the diff node (to get the IDs to update)
    graph.create_edge(&connect_if_node, &update_connect_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        parent_node,
        &update_connect_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            parent_link,
            RowSink::ExactlyOneWriteArgs(child_link, &UpdateOrCreateArgsInput),
            Some(DataExpectation::non_empty_rows(
                MissingRelatedRecord::builder()
                    .model(&parent_relation_field.model())
                    .relation(&parent_relation_field.relation())
                    .operation(DataOperation::NestedSet)
                    .build(),
            )),
        ),
    )?;

    graph.create_edge(
        &diff_left_to_right_node,
        &update_connect_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |mut update_connect_node, diff_left_result| {
                if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = update_connect_node {
                    ur.record_filter = diff_left_result.to_vec().into();
                }

                Ok(update_connect_node)
            }),
            None,
        ),
    )?;

    // Update (disconnect) case: Check right diff IDs.
    let disconnect_if_node = graph.create_node(Node::Flow(Flow::if_non_empty()));
    let write_args = WriteArgs::from_result(empty_child_link, crate::executor::get_request_now());
    let update_disconnect_node =
        utils::update_records_node_placeholder_with_args(graph, Filter::empty(), child_model, write_args);

    let child_side_required = parent_relation_field.related_field().is_required();
    let rf = parent_relation_field.clone();

    graph.create_edge(
        &diff_right_to_left_node,
        &disconnect_if_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier.clone(),
            RowSink::All(&IfInput),
            child_side_required.then(|| DataExpectation::empty_rows(RelationViolation::from(rf))),
        ),
    )?;

    // Connect to the if node and the diff node (to get the IDs to update)
    graph.create_edge(&disconnect_if_node, &update_disconnect_node, QueryGraphDependency::Then)?;
    graph.create_edge(
        &diff_right_to_left_node,
        &update_disconnect_node,
        QueryGraphDependency::ProjectedDataSinkDependency(
            child_model_identifier,
            RowSink::All(&UpdateManyRecordsSelectorsInput),
            None,
        ),
    )?;

    Ok(())
}
