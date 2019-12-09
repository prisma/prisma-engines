use super::write_arguments::WriteArguments;
use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{Filter, ScalarCompare};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested update (one) cases.
/// The graph is expanded with the `Check` and `Update` nodes.
///
/// (illustration simplified, `Parent` / `Read Result` exemplary)
///
/// ```text
///    ┌──────┐
/// ┌──│Parent│────────┐
/// │  └──────┘        │
/// │      │           │
/// │      ▼           ▼
/// │  ┌──────┐  ┌───────────┐
/// │  │Check │  │Read result│
/// │  └──────┘  └───────────┘
/// │      │
/// │      ▼
/// │  ┌──────┐
/// └─▶│Update│
///    └──────┘
/// ```
pub fn connect_nested_update(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let (data, finder) = if parent_relation_field.is_list {
            // We have to have a record specified as a record finder in "where".
            // This finder is used to read the children first, to make sure they're actually connected.
            // The update itself operates on the ID found by the read check.
            let mut map: ParsedInputMap = value.try_into()?;
            let where_arg = map.remove("where").unwrap();
            let record_finder = extract_record_finder(where_arg, &child_model)?;
            let data_value = map.remove("data").unwrap();

            (data_value, Some(record_finder))
        } else {
            (value, None)
        };

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, finder)?;

        let update_node = update::update_record_node(graph, None, Arc::clone(child_model), data.try_into()?)?;
        let id_field = child_model.fields().id();

        graph.create_edge(
            &find_child_records_node,
            &update_node,
            QueryGraphDependency::ParentIds(Box::new(|mut node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!(
                        "Expected a valid parent ID to be present for nested update to-one case."
                    ))),
                }?;

                if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = node {
                    ur.where_ = Some(RecordFinder {
                        field: id_field,
                        value: parent_id,
                    });
                }

                Ok(node)
            })),
        )?;
    }

    Ok(())
}

pub fn connect_nested_update_many(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let mut map: ParsedInputMap = value.try_into()?;
        let where_arg = map.remove("where").unwrap();
        let data_value = map.remove("data").unwrap();
        let data_map: ParsedInputMap = data_value.try_into()?;
        let where_map: ParsedInputMap = where_arg.try_into()?;

        let filter = extract_filter(where_map, child_model)?;
        let update_args = WriteArguments::from(&child_model, data_map)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;

        // TODO: this looks like some duplication from write/update.rs
        let update_many = WriteQuery::UpdateManyRecords(UpdateManyRecords {
            model: Arc::clone(&child_model),
            filter,
            args: update_args.args,
        });

        let update_many_node = graph.create_node(Query::Write(update_many));
        let id_field = child_model.fields().id();

        graph.create_edge(
            &find_child_records_node,
            &update_many_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut node, parent_ids| {
                if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = node {
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
