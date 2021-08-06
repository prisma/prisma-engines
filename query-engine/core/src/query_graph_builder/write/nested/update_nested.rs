use super::*;
use crate::{
    constants::args,
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::Filter;
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested update (single record) cases.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─
/// ┌──    Parent   │─ ─ ─ ─ ─
/// │  └ ─ ─ ─ ─ ─ ─          │
/// │         │
/// │         ▼               ▼
/// │  ┌────────────┐   ┌ ─ ─ ─ ─ ─
/// │  │   Check    │      Result  │
/// │  └────────────┘   └ ─ ─ ─ ─ ─
/// │         │
/// │         ▼
/// │  ┌────────────┐
/// └─▶│   Update   │
///    └────────────┘
/// ```
#[tracing::instrument(skip(graph, parent, parent_relation_field, value, child_model))]
pub fn nested_update(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let (data, filter) = if parent_relation_field.is_list {
            // We have to have a single record filter in "where".
            // This is used to read the children first, to make sure they're actually connected.
            // The update itself operates on the record found by the read check.
            let mut map: ParsedInputMap = value.try_into()?;
            let where_arg: ParsedInputMap = map.remove(args::WHERE).unwrap().try_into()?;

            let filter = extract_unique_filter(where_arg, &child_model)?;
            let data_value = map.remove(args::DATA).unwrap();

            (data_value, filter)
        } else {
            (value, Filter::empty())
        };

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter)?;

        let update_node = update::update_record_node(
            graph,
            connector_ctx,
            Filter::empty(),
            Arc::clone(child_model),
            data.try_into()?,
        )?;

        let child_model_identifier = parent_relation_field.related_model().primary_identifier();

        graph.create_edge(
            &find_child_records_node,
            &update_node,
            QueryGraphDependency::ParentProjection(
                child_model_identifier.clone(),
                Box::new(move |mut update_node, mut child_ids| {
                    let child_id = match child_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::AssertionError(
                            "Expected a valid parent ID to be present for nested update to-one case.".to_string(),
                        )),
                    }?;

                    if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = update_node {
                        ur.record_filter = child_id.into();
                    }

                    Ok(update_node)
                }),
            ),
        )?;
    }

    Ok(())
}

#[tracing::instrument(skip(graph, parent, parent_relation_field, value, child_model))]
pub fn nested_update_many(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let mut map: ParsedInputMap = value.try_into()?;
        let where_arg = map.remove(args::WHERE).unwrap();
        let data_value = map.remove(args::DATA).unwrap();
        let data_map: ParsedInputMap = data_value.try_into()?;
        let where_map: ParsedInputMap = where_arg.try_into()?;
        let child_model_identifier = parent_relation_field.related_model().primary_identifier();

        let filter = extract_filter(where_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter)?;

        let update_many_node =
            update::update_many_record_node(graph, connector_ctx, Filter::empty(), Arc::clone(child_model), data_map)?;

        graph.create_edge(
            &find_child_records_node,
            &update_many_node,
            QueryGraphDependency::ParentProjection(
                child_model_identifier.clone(),
                Box::new(move |mut update_many_node, child_ids| {
                    if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = update_many_node {
                        // ur.set_filter(Filter::and(vec![ur.filter.clone(), child_ids.filter()]));
                        ur.record_filter = child_ids.into();
                    }

                    Ok(update_many_node)
                }),
            ),
        )?;
    }

    Ok(())
}
