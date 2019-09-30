use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested create cases.
/// The resulting graph can take two forms, based on the relation type to the parent model:
///
/// (illustration simplified)
///
///  1:1 relation case            n:m relation case
///    ┌──────┐                      ┌──────┐
/// ┌──│Parent│────────┐          ┌──│Parent│────────┐
/// │  └──────┘        │          │  └──────┘        │
/// │      │           │          │      │           │
/// │      ▼           ▼          │      ▼           ▼
/// │  ┌──────┐  ┌───────────┐    │  ┌──────┐  ┌───────────┐
/// │  │Check │  │Read result│    │  │Create│  │Read result│
/// │  └──────┘  └───────────┘    │  └──────┘  └───────────┘
/// │      │                      │      │
/// │      ▼                      │      ▼
/// │  ┌──────┐                   │  ┌───────┐
/// └─▶│Create│                   └─▶│Connect│
///    └──────┘                      └───────┘
///
/// Where `Parent` with `Read result` is examplary for a typical nested create use case.
/// The actual parent graph can differ. The pieces added by this handler are `Check`, `Create` and `Connect`.
pub fn connect_nested_create(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let child = create::create_record_node(graph, Arc::clone(child_model), value.try_into()?)?;

        // Make sure the creation is done in correct order.
        let (parent, child, parent_relation_field) = utils::flip_nodes(graph, parent, &child, parent_relation_field);
        let relation_field_name = parent_relation_field.name.clone();

        // We need to perform additional 1:1 relation checks if the parent of a nested create is not a create as well.
        // Why? If the top is a create, we don't have to consider already existing relation connections,
        // or other relation requirements from parent to child, as they can't exist yet.
        if !utils::node_is_create(graph, &parent) && parent_relation_field.relation().is_one_to_one() {
            insert_relation_checks(graph, parent, child, &parent_relation_field)?;
        }

        // Connect parent and child.
        graph.create_edge(
            parent,
            child,
            QueryGraphDependency::ParentIds(Box::new(|mut node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!(
                        "Expected a valid parent ID to be present for a nested create."
                    ))),
                }?;

                // The following injection is necessary for cases where the relation is inlined.
                // The injection won't do anything in other (read: m:n) cases, as those are handled separately further down.
                // Todo: This makes some assumptions about the connector implementation and needs to be handled cleaner with more
                //       connectors coming in. This implementation can be considered temporary.
                if let Node::Query(Query::Write(ref mut wq)) = node {
                    wq.inject_non_list_arg(relation_field_name, parent_id);
                }

                Ok(node)
            })),
        );

        // Detect if a connect is necessary between the nodes.
        // A connect is necessary if the nested create is done on a relation that
        // is a many-to-many (aka manifested as an actual join table in SQL, for example).
        if parent_relation_field.relation().is_many_to_many() {
            connect::connect_records_node(graph, parent, child, &parent_relation_field, None, None);
        }
    }

    Ok(())
}

fn insert_relation_checks(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    child: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<()> {
    let child_relation_field = parent_relation_field.related_field();

    match (parent_relation_field.is_required, child_relation_field.is_required) {
        // Both sides required. Results in a violation, because the parent is already existing (== not a create!)
        // with a connected child that would be disconnected by creating a new one, violating the required relation side of the child.
        (true, true) => Err(QueryGraphBuilderError::RelationViolation(
            (parent_relation_field).into(),
        )),

        // Two remaining possibilities:
        // 1) Child requires a parent node, but not vice versa.
        // 2) Both do not require the relation.
        //
        // For case 1): If the child needs the parent to be connected, we can't create
        // a new child without violating the required relation side of the child if there's already a child existing.
        // Hence, we have to check if there's already an existing child for the parent and error out if found.
        //
        // For case 2): Any existing child record has to be disconnected first (e.g. unset the column with an update).
        // However, we actually don't have to do an explicit disconnect:
        // - We know that the inlined field has to be on the child, because the flip check guarantees that if the
        //   child is a create (which it obv. is in a nested create case), and the parent has the inline ID, a flip is performed.
        // - Hence, we only need to insert a check for existing nodes and the handling of the "disconnect / connect" is done on the
        //   edge from `parent` to `child` (see inline inject further down).
        //
        // The parent IDs edge transformation from `check_node` to `child` either fails if `child_side_required` is true (case 1),
        // passes through successfully ("noop", case 2).
        (false, child_side_required) => {
            let check_node = utils::find_ids_by_parent_node(graph, parent_relation_field, parent, None);
            let parent_relation_field = Arc::clone(parent_relation_field);

            // Connect check and child
            graph.create_edge(
                &check_node,
                child,
                QueryGraphDependency::ParentIds(Box::new(move |child_node, parent_ids| {
                    if !parent_ids.is_empty() && child_side_required {
                        return Err(QueryGraphBuilderError::RelationViolation(
                            (&parent_relation_field).into(),
                        ));
                    }

                    Ok(child_node)
                })),
            );

            Ok(())
        }

        // The only other case is (true, false), i.e. the parent requires, the child doesn't, can be ignored here.
        _ => Ok(()),
    }
}
