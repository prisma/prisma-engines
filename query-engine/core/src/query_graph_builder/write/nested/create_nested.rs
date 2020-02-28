use super::utils::IdFilter;
use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::Filter;
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested create cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn connect_nested_create(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    // Build all create nodes upfront.
    let creates: Vec<NodeRef> = utils::coerce_vec(value)
        .into_iter()
        .map(|value| create::create_record_node(graph, Arc::clone(child_model), value.try_into()?))
        .collect::<QueryGraphBuilderResult<Vec<NodeRef>>>()?;

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, creates)?;
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, creates)?;
    } else {
        handle_one_to_one(graph, parent_node, parent_relation_field, creates)?;
    }

    Ok(())
}

/// Handles a many-to-many nested create.
/// This is the least complicated case, as it doesn't involve
/// checking for relation violations or updating inlined relations.
///
/// Example for 2 children being created:
///```text
///    ┌ ─ ─ ─ ─ ─ ─
/// ┌──    Parent   │──────────┬────────┐─ ─ ─ ─ ┐
/// │  └ ─ ─ ─ ─ ─ ─           │        │
/// │         │                │        │        │
/// │         ▼                ▼        │        ▼
/// │  ┌────────────┐   ┌────────────┐  │  ┌ ─ ─ ─ ─ ─
/// │  │Create Child│   │Create Child│  │     Result  │
/// │  └────────────┘   └────────────┘  │  └ ─ ─ ─ ─ ─
/// │         │                │        │
/// │         ▼                ▼        │
/// │  ┌────────────┐   ┌────────────┐  │
/// └─▶│  Connect   │   │  Connect   │◀─┘
///    └────────────┘   └────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    // Todo optimize with createMany
    for create_node in create_nodes {
        graph.create_edge(&parent_node, &create_node, QueryGraphDependency::ExecutionOrder)?;
        connect::connect_records_node(graph, &parent_node, &create_node, &parent_relation_field, 1)?;
    }

    Ok(())
}

/// Handles a one-to-many nested create.
/// There are two main cases: Either the relation side is inlined on the parent or the child.
///
/// Concerning `create_nodes` parameter:
/// - If the relation side is inlined on the parent, `create_nodes` can only be of length 1,
///   because there can only be one possible child being created in that direction in the API.
///
/// - If the relation side is inlined on the child, `create_nodes` can be of any size greater or equal 1.
///   Opposite to the above reasoning, an indefinite amount of children can be created.
///
/// ## Inlined on the parent
/// We need to create a graph that has a create node for the child first and then the parent operation
/// to have the child ID ready if needed.
///
/// Example finalized graph:
/// ```text
/// ┌────────────────┐
/// │  Child Create  │
/// └────────────────┘
///          │
///          │
///          │
///          ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─
///       Parent     │
/// └ ─ ─ ─ ─ ─ ─ ─ ─
///          │
///          │
///          │
///          ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─
///       Result     │
/// └ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
///
/// ## Inlined on the child
/// We can have the parent operation first, then do the child create(s) and
/// insert the parent ID into the inline relation field.
///
/// Example graph for 2 children:
/// ```text
///                 ┌ ─ ─ ─ ─ ─ ─
///        ┌────────    Parent   │─ ─ ─ ─ ─
///        │        └ ─ ─ ─ ─ ─ ─          │
///        │               │
///        │               │               │
///        │               │
///        ▼               ▼               ▼
/// ┌────────────┐  ┌────────────┐  ┌ ─ ─ ─ ─ ─ ─
/// │Create Child│  │Create Child│      Result   │
/// └────────────┘  └────────────┘  └ ─ ─ ─ ─ ─ ─
/// ```
fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    if parent_relation_field.is_inlined_on_enclosing_model() {
        let child_node = create_nodes
            .pop()
            .expect("[Query Graph] Expected one nested create node on a 1:m relation with inline IDs on the parent.");

        // We need to swap the create node and the parent because the inlining is done in the parent.
        graph.mark_nodes(&parent_node, &child_node);

        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        // We extract the child linking fields in the edge, because after the swap, the child is the new parent.
        graph.create_edge(
            &parent_node,
            &child_node,
            QueryGraphDependency::ParentIds(child_link, Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!(
                        "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-many relation."
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_id_into_args(parent_link.assimilate(parent_id)?);
                }

                Ok(child_node)
            })),
        )?;
    } else {
        for create_node in create_nodes {
            let parent_link = parent_relation_field.linking_fields();
            let child_link = parent_relation_field.related_field().linking_fields();

            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ParentIds(parent_link, Box::new(move |mut child_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::AssertionError(format!(
                            "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-many relation."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = child_node {
                        wq.inject_id_into_args(child_link.assimilate(parent_id)?);
                    }

                    Ok(child_node)
                })))?;
        }
    };

    Ok(())
}

/// Handles a one-to-one nested create.
/// Most complex case as there are edge cases where we need to make sure
/// that we don't violate relation requirements.
///
/// The full graph that can be created by this handler depends on the inline relation side.
///
/// If the relation is inlined in the child:
/// ```text
///                 ┌────────────────┐
///        ┌────────│     Parent     │─────────┐
///        │        └────────────────┘         │
///        │                 │                 │
///        │                 │  ┌ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///        │                 │                 ▼
///        │                 │  │ ┌────────────────────────┐                            │
///        │                 │    │     Read ex. child     │──┐
///        │                 │  │ └────────────────────────┘  │                         │
///        │                 │                 │              │
///        │                 │  │              ▼              │(Fail on c > 0 if child  │
///        │                 │    ┌────────────────────────┐  │     side required)
///        │                 │  │ │ If c > 0 && c. inlined │  │                         │
///        │                 │    └────────────────────────┘  │
///        │                 │  │              │              │                         │
///        │                 │                 ▼              │
///        │                 │  │ ┌────────────────────────┐  │                         │
///        │                 │    │    Update ex. child    │◀─┘
///        │                 │  │ └────────────────────────┘                            │
///        │                 │
///        │                 │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///        ▼                 ▼
/// ┌────────────┐  ┌────────────────┐
/// │   Result   │  │  Child Create  │
/// └────────────┘  └────────────────┘
/// ```
///
/// If the relation is inlined in the parent:
/// ```text
///    ┌────────────────┐
/// ┌──│  Child Create  │
/// │  └────────────────┘
/// │           │
/// │           ▼
/// │  ┌────────────────┐
/// ├──│     Parent     │─────────┐
/// │  └────────────────┘         │
/// │           │                 │
/// │           │                 │
/// │           │  ┌ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
/// │           │                 ▼
/// │           │  │ ┌────────────────────────┐                            │
/// │           │    │     Read ex. child     │──┐
/// │           │  │ └────────────────────────┘  │                         │
/// │           │                 │              │
/// │           │  │              ▼              │(Fail on c > 0 if child  │
/// │           │    ┌────────────────────────┐  │     side required)
/// │           │  │ │ If c > 0 && c. inlined │  │                         │
/// │           │    └────────────────────────┘  │
/// │           │  │              │              │                         │
/// │           │                 ▼              │
/// │           │  │ ┌────────────────────────┐  │                         │
/// │           │    │    Update ex. child    │◀─┘
/// │           │  │ └────────────────────────┘                            │
/// │           │
/// │           │  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │           ▼
/// │    ┌────────────┐
/// │    │   Result   │
/// │    └────────────┘
/// │    ┌────────────┐
/// └───▶│   Update   │ (if non-create)
///      └────────────┘
/// ```
///
/// Important: We can not inject from `Child Create` to `Parent` if `Parent` is a non-create, as it would cause
/// the following issue (example):
/// - Parent is an update, doesn't have a connected child on relation x.
/// - Parent gets injected with a child on x, because that's what the nested create is supposed to do.
/// - The update runs, the relation is updated.
/// - Now the check runs, because it's dependent on the parent's ID... but the check finds an existing child and fails...
/// ... because we just updated the relation.
///
/// For these reasons, we need to have an extra update at the end if it's inlined on the parent and a non-create.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required;
    let child_side_required = child_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();

    let parent_link = parent_relation_field.linking_fields();
    let child_link = child_relation_field.linking_fields();

    // Build-time check
    if !parent_is_create && (parent_side_required && child_side_required) {
        // Both sides are required, which means that we know that there already has to be a parent connected a child (it must exist).
        // Creating a new child for the parent would disconnect the other child, violating the required side of the existing child.
        return Err(QueryGraphBuilderError::RelationViolation(
            (parent_relation_field).into(),
        ));
    }

    let create_node = create_nodes
        .pop()
        .expect("[Query Graph] Expected only one nested create node on a 1:m relation with inline IDs on the parent.");

    // If the parent node is not a create, we need to do additional checks and potentially disconnect an already existing child,
    // because we know that the parent node has to exist already.
    // If the parent is a create, we can be sure that there's no existing relation to anything, and we don't need checks,
    // especially because we are in a nested create scenario - the child also can't exist yet, so no checks are needed for an
    // existing parent, either.
    // For the above reasons, the checks always live on `parent_node`.
    if !parent_is_create {
        utils::insert_existing_1to1_related_model_checks(graph, &parent_node, parent_relation_field)?;
    }

    // If the relation is inlined on the parent, we swap the create and the parent to have the child ID for inlining.
    // Swapping changes the extraction model identifier as well.
    let (extractor, assimilator) = if relation_inlined_parent {
        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the ID first.
        graph.mark_nodes(&parent_node, &create_node);
        (child_link.clone(), parent_link)
    } else {
        (parent_link, child_link.clone())
    };

    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ParentIds(extractor, Box::new(move |mut child_node, mut parent_ids| {
            let parent_id = match parent_ids.pop() {
            Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!(
                    "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation."
                ))),
            }?;

            // We ONLY inject for creates here. Check end of doc comment for explanation.
            if let Node::Query(Query::Write(ref mut q @ WriteQuery::CreateRecord(_))) = child_node {
                q.inject_id_into_args(assimilator.assimilate(parent_id)?);
            }

            Ok(child_node)
        })),
    )?;

    // Relation is inlined on the Parent and a non-create.
    // Create an update node for Parent to set the connection to the child.
    // For explanation see end of doc comment.
    if relation_inlined_parent && !parent_is_create {
        let parent_model = parent_relation_field.model();
        let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);
        let parent_link = parent_relation_field.linking_fields();

        graph.create_edge(
            &create_node,
            &update_node,
            QueryGraphDependency::ParentIds(child_link, Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_id_into_args(parent_link.assimilate(parent_id)?);
                }

                Ok(child_node)
            })),
         )?;

        let parent_model_identifier = parent_relation_field.model().primary_identifier();

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ParentIds(parent_model_identifier, Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation, updating inlined on parent."))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.add_filter(parent_id.filter());
                }

                Ok(child_node)
            })),
         )?;
    }

    Ok(())
}
