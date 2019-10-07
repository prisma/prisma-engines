use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::sync::Arc;

/// Handles nested connect cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on thte graph shapes can be found on the individual handlers.
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

    graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder);
    connect::connect_records_node(graph, &parent_node, &child_node, &parent_relation_field, None, None);

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
    let (parent_node, child_node, relation_field_name) = if parent_relation_field.relation_is_inlined_in_parent() {
        let record_finder = extract_record_finder(value, &child_model)?;
        let read_query = utils::id_read_query_infallible(&child_model, record_finder);
        let child_node = graph.create_node(read_query);

        // For the injection, we need the name of the field on the inlined side, in this case the parent.
        let relation_field_name = parent_relation_field.name.clone();

        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the ID first.
        let (parent_node, child_node) = utils::swap_nodes(graph, parent_node, child_node);

        (parent_node, child_node, relation_field_name)
    } else {
        let update_node = utils::update_record_node_placeholder(graph, None, Arc::clone(child_model));

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
    );

    Ok(())
}

/// Handles a one-to-one nested connect.
/// Most complex case as there are plenty of cases involved where we need to make sure
/// that we don't violate relation requirements.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let parent_model = parent_relation_field.model();
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

    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let record_finder = extract_record_finder(value, &child_model)?;
    let read_query = utils::id_read_query_infallible(&child_model, record_finder);
    let read_new_child_node = graph.create_node(read_query);

    // We always start with the read node in a nested connect 1:1 scenario, so swap the read node into the existing hierarchy.
    // Preserve naming to keep the code readable.
    let (read_new_child_node, parent_node) = utils::swap_nodes(graph, parent_node, read_new_child_node);

    // Next is the check for and possible disconnect of an existing parent in between `read_new_child_node` and `parent_node`.
    insert_existing_parent_checks(graph, &read_new_child_node, &parent_node, &parent_relation_field);

    let relation_field_name = parent_relation_field.related_field().name.clone();

    graph.create_edge(
        &read_new_child_node,
        &parent_node,
        QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                "[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation."
            ))),
            }?;

            // This takes care of cases where the relation is inlined on the parent
            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.inject_non_list_arg(relation_field_name, parent_id);
            }

            Ok(child_node)
        })),
    );

    unimplemented!()
}

fn insert_existing_parent_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) {
    let parent_model = parent_relation_field.model();
    let parent_model_id_field = parent_model.fields().id();
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required;
    // let child_side_required = child_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();

    // Now check and disconnect the old parent, if necessary.
    let read_old_parent_query_node =
        utils::insert_find_children_by_parent_node(graph, &child_relation_field, &parent_node, None);
    let rf = Arc::clone(&parent_relation_field);

    if relation_inlined_parent {
        // If the relation is inlined in the parent, we need to update the old parent and null out the relation (i.e. "disconnect").
        // However, if the parent side is required, we fail during runtime before disconnecting, as that would violate the parent relation side.
        let update_old_parent_node = utils::update_record_node_placeholder(graph, None, parent_model);
        let relation_field_name = parent_relation_field.name.clone();

        graph.create_edge(&read_old_parent_query_node, &update_old_parent_node, QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
            // If the parent requires the connection, we need to make sure that there isn't a parent already connected
            // to the existing child, as that would violate the other parent's relation side.
            if parent_ids.len() > 0 && parent_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating previous parent."))),
            }?;

            /// Need noop node, remove or whatever concept here.
            let finder = RecordFinder {
                field: parent_model_id_field,
                value: parent_id,
            };

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.inject_record_finder(finder);
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(child_node)
        })));

        graph.create_edge(
            &update_old_parent_node,
            &parent_node,
            QueryGraphDependency::ExecutionOrder,
        );
    } else {
        graph.create_edge(&read_old_parent_query_node, &parent_node, QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
            // If the parent requires the connection, we need to make sure that there isn't a parent already connected
            // to the existing child, as that would violate the other parent's relation side.
            if parent_ids.len() > 0 && parent_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            Ok(node)
        };
    }
}

// // First, we need to check if the parent operation and the read new child operation need to be swapped.
// // This also ensures that >>> the inlined relation side is always on `child_node` <<<,which simplifies the code structure/
// let (parent_node, child_node, parent_relation_field) = if parent_relation_field.relation_is_inlined_in_parent() {
//     let (new_parent, new_child) = utils::swap_nodes(graph, parent_node, child_node);
//     (new_parent, new_child, parent_relation_field.related_field())
// } else {
//     (parent_node, child_node, Arc::clone(parent_relation_field))
// };

// insert_relation_checks(graph, &parent_node, &child_node, &parent_relation_field)?;

// // Find out which possible update strategy we need to do for the given parent.
// // The case "Parent holds the inlined ID && parent IS a create" is covered by the flip.
// //
// match (parent_is_create, relation_inlined_parent) {
//     // Parent holds the inlined ID && parent IS NOT a create => Ok, update on parent works. (Update node on parent ID, requires extra fetch on parent id, or requires parent to return an ID)
//     // Optimization: If the parent is already an update, we can merge the inline update into the parent and use a read before to get the child ID.
//     (false, true) => unimplemented!(),

//     // Child holds the inlined ID => Update on child node.
//     (_, false) => {
//         let update_node = utils::update_record_node_placeholder(graph, None, Arc::clone(child_model));
//         let relation_field_name = parent_relation_field.related_field().name.clone();

//         graph.create_edge(
//             &child_node,
//             &update_node,
//             QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
//                 let parent_id = match parent_ids.pop() {
//                     Some(pid) => Ok(pid),
//                     None => Err(QueryGraphBuilderError::AssertionError(format!(
//                         "Expected a valid parent ID to be present for a nested connect."
//                     ))),
//                 }?;

//                 if let Node::Query(Query::Write(ref mut wq)) = child_node {
//                     wq.inject_non_list_arg(relation_field_name, parent_id);
//                 }

//                 Ok(child_node)
//             })),
//         );
//     }

//     _ => unimplemented!(),
// };

// let (parent_node, child_node, parent_relation_field) =
//     utils::ensure_query_ordering(graph, parent_node, &child_node, parent_relation_field);

// // If the parent is a create we have to make sure that we have the ID available - do the flip.

// // let model_to_update = if parent_relation_field.relation_is_inlined_in_parent() {
// //     parent_relation_field.model()
// // } else {
// //     Arc::clone(child_model)
// // };

// // Prepare an empty update query.
// //

// let relation_field_name = parent_relation_field.name.clone();

// // In case the relation is 1:1, we need to insert additional checks between
// // the parent and the child to guarantee 1:1 relation integrity.
// if parent_relation_field.relation().is_one_to_one() {
//     insert_relation_checks(graph, parent_node, child_node, &parent_relation_field)?;
// }

// / Inserts one to one relation checks into the graph in between the given
// / `parent_node` and `child_node`.
// /
// / Those checks will either cause the graph building to fail immediately, or
// / will fail at graph runtime.
// fn insert_relation_checks(
//     graph: &mut QueryGraph,
//     parent_node: &NodeRef,
//     child_node: &NodeRef,
//     parent_relation_field: &RelationFieldRef,
// ) -> QueryGraphBuilderResult<()> {
//     let child_relation_field = parent_relation_field.related_field();
//     let parent_is_create = utils::node_is_create(graph, &parent_node);
//     // let relation_field_name = parent_relation_field.name.clone();

//     match (parent_relation_field.is_required, child_relation_field.is_required) {
//         // Both sides are required, which means that know that there has to be already a parent connected to the child.
//         // A connect to the child would disconnect the other connection, violating the required side of the old parent.
//         (true, true) => Err(QueryGraphBuilderError::RelationViolation(
//             (parent_relation_field).into(),
//         )),

//         // Parent requires the connection, child doesn't. If we connect the child, we need to make sure
//         // that there isn't a parent already connected, as that would violate the parent's relation side.
//         (true, false) => {
//             // find old parent by new
//         },

//         // This case is only interesting if the parent is NOT a create, because
//         (false, true) if !parent_is_create => Err(QueryGraphBuilderError::RelationViolation(
//             (parent_relation_field).into(),
//         )),

//         (false, false) => unimplemented!(),

//         // Todo come up with a plan to handle (false, true) if parent is create
//         _ => unimplemented!(),
//     }?;

//     unimplemented!()
// }

// let child = create::create_record_node(graph, Arc::clone(child_model), value.try_into()?)?;

// Perform additional 1:1 relation checks.
// match (p.is_required, c.is_required) {
// (true, true) => Err(self.relation_violation()),
// (true, false) => Ok(Some(self.check_for_old_parent_by_child(&self.where_))),

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
