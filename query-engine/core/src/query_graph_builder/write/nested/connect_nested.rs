use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::sync::Arc;

/// Handles nested connect cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model:
///
/// (illustration simplified)
///
/// 1:1 relation case                               n:m relation case
///
pub fn connect_nested_connect(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        // First, we need to build a read query on the record to be connected.
        let record_finder = extract_record_finder(value, &child_model)?;
        let child_read_query = utils::id_read_query_infallible(&child_model, record_finder);
        let child_node = graph.create_node(child_read_query);

        // If the connect is on a many-to-many relation, we need a connect (i.e. create "join table" entry).
        // Otherweise, we don't need to do an actual connect, but an update on the inlined relation,
        // depending on the parent.
        if parent_relation_field.relation().is_many_to_many() {
            graph.create_edge(parent_node, &child_node, QueryGraphDependency::ExecutionOrder);
            connect::connect_records_node(graph, parent_node, &child_node, &parent_relation_field, None, None);
        } else {
            // If the parent is a create we have to make sure that we have the ID available - do the flip.

            // Find out which possible update strategy we need to do for the given parent.
            // Todo - Cases here:
            // - Parent holds the inlined ID && parent IS NOT a create => Ok, update on parent works. (Update node on parent ID, requires extra fetch on parent id, or requires parent to return an ID)
            // - Parent holds the inlined ID && parent IS a create => Doesn't work, need to flip. (first read, then create, inject id into args)
            // - Child holds the inlined ID => Ok, update on child works. (Update node on child ID)
            //
            // If we have
            let child_node = if utils::node_is_create(graph, parent_node) {
                unimplemented!()
            } else {
                unimplemented!()
            };

            // let model_to_update = if parent_relation_field.relation_is_inlined_in_parent() {
            //     parent_relation_field.model()
            // } else {
            //     Arc::clone(child_model)
            // };

            // Prepare an empty update query.
            // let update_node = utils::update_record_node_placeholder(graph, None, model_to_update);

            let (parent_node, child_node, parent_relation_field) =
                utils::ensure_query_ordering(graph, parent_node, &child_node, parent_relation_field);

            let relation_field_name = parent_relation_field.name.clone();

            // In case the relation is 1:1, we need to insert additional checks between
            // the parent and the child to guarantee 1:1 relation integrity.
            if parent_relation_field.relation().is_one_to_one() {
                // insert_relation_checks(graph, parent, child, &parent_relation_field)?;
                unimplemented!()
            }

            // Flip the read node and parent node if necessary.
            // let (parent_node, child_node, relation_field) = utils::flip_nodes(graph, parent, &child_node, parent_relation_field);
            // let relation_field_name = relation_field.name.clone();

            // The update needs to be done on the model where the relation is inlined.
            unimplemented!()
        };

        // let child = create::create_record_node(graph, Arc::clone(child_model), value.try_into()?)?;

        // Perform additional 1:1 relation checks.
        // match (p.is_required, c.is_required) {
        // (true, true) => Err(self.relation_violation()),
        // (true, false) => Ok(Some(self.check_for_old_parent_by_child(&self.where_))),
    }

    // Edge from parent to child (read query).
        // graph.create_edge(
        //     parent,
        //     child_node,
        //     QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
        //         let parent_id = match parent_ids.pop() {
        //             Some(pid) => Ok(pid),
        //             None => Err(QueryGraphBuilderError::AssertionError(format!(
        //                 "Expected a valid parent ID to be present for nested connect pre read."
        //             ))),
        //         }?;

        //         if let Node::Query(Query::Read(ref mut rq)) = node {
        //             rq.inject_record_finder()
        //         }

        //         // // If the child is a write query, inject the parent id.
        //         // // This covers cases of inlined relations.
        //         // if let Node::Query(Query::Write(ref mut wq)) = child_node {
        //         //     wq.inject_non_list_arg(relation_field_name, parent_id)
        //         // }

        //         Ok(child_node)
        //     })),
        // );

    Ok(())
}

fn insert_relation_checks(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    child: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<()> {
    unimplemented!()
}
