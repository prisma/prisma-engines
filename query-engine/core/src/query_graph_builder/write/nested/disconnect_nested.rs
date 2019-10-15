use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::Filter;
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested connect cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn connect_nested_disconnect(
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
    disconnect::disconnect_records_node(graph, &parent_node, &child_node, &parent_relation_field, None, None)?;

    Ok(())
}

fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let filter: Filter = if parent_relation_field.is_list {
        let record_finder = extract_record_finder(value, &child_model)?;
        record_finder.into()
    } else {
        Filter::empty()
    };
    let find_child_records_node =
        utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, filter)?;

    disconnect::disconnect_records_node(
        graph,
        &parent_node,
        &find_child_records_node,
        &parent_relation_field,
        None,
        None,
    )?;
    Ok(())
}

fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let val: PrismaValue = value.try_into()?;
    let should_delete = if let PrismaValue::Boolean(b) = val { true } else { false };

    if should_delete {
        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, Filter::empty())?;

        disconnect::disconnect_records_node(
            graph,
            &parent_node,
            &find_child_records_node,
            &parent_relation_field,
            None,
            None,
        )?;
    }

    Ok(())
}
