use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{Filter, ScalarCompare};
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
/// - If the relation side is inlined on the child, `create_nodes` can be of any size greater 1.
///   Opposite to the above reasoning, an indefinite amount of children can be created.
///
/// ## Inlined on the parent
/// We need to create a graph that has a create node for the child first and then the parent operation
/// to have the child ID ready if needed.
/// In case the relation link used to satisfy the relation is NOT the same as the primary ID of the
/// parent model (i.e. `Create Child` here) and it can be determined that the parent operation does
/// not return the necessary set of fields, we need to reload (`Reload Child`) the created record to
/// be able to write the correct inlined data into `Parent`. NOTE: Currently we always insert the reload!
///
/// Example finalized (with swaps already performed) graph:
/// ```text
/// ┌────────────────┐             ┌────────────────┐
/// │  Child Create  │          ┌──│  Child Create  │
/// └────────────────┘          │  └────────────────┘
///          │                  │           │
///          │                  │           │
///          │                  │           │
///          ▼                  │           ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─           │  ┌────────────────┐
///       Parent     │          │  │  Reload Child  │
/// └ ─ ─ ─ ─ ─ ─ ─ ─           │  └────────────────┘
///          │                  │           │
///          │                  │           │
///          │                  │           │
///          ▼                  │           ▼
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─           │  ┌ ─ ─ ─ ─ ─ ─ ─ ─
///       Result     │          └─▶      Parent     │
/// └ ─ ─ ─ ─ ─ ─ ─ ─              └ ─ ─ ─ ─ ─ ─ ─ ─
///                                         │
///                                         │
///                                         │
///                                         ▼
///                                ┌ ─ ─ ─ ─ ─ ─ ─ ─
///                                      Result     │
///                                └ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
///
/// ## Inlined on the child
/// We can have the parent operation first, then do the child create(s) and
/// insert the parent ID into the inline relation field. We need to reload the
/// parent node if it doesn't yield the necessary fields in the result to satisfy
/// the relation inlining. (Todo same as above, we are always reloading right now)
///
/// Example graph for 2 children:
/// ```text
///                 ┌ ─ ─ ─ ─ ─ ─                                       ┌ ─ ─ ─ ─ ─ ─ ─ ─
///        ┌────────    Parent   │─ ─ ─ ─ ─                                   Parent     │─ ─ ─ ─ ─
///        │        └ ─ ─ ─ ─ ─ ─          │                            └ ─ ─ ─ ─ ─ ─ ─ ─          │
///        │               │                                                     │
///        │               │               │                                     │                 │
///        │               │                                                     │
///        ▼               ▼               ▼                                     ▼                 ▼
/// ┌────────────┐  ┌────────────┐  ┌ ─ ─ ─ ─ ─ ─                       ┌────────────────┐  ┌ ─ ─ ─ ─ ─ ─
/// │Create Child│  │Create Child│      Result   │           ┌──────────│ Reload Parent  │      Result   │
/// └────────────┘  └────────────┘  └ ─ ─ ─ ─ ─ ─            │          └────────────────┘  └ ─ ─ ─ ─ ─ ─
///                                                          │                   │
///                                                          │                   │
///                                                          │                   │
///                                                          ▼                   ▼
///                                                 ┌────────────────┐  ┌────────────────┐
///                                                 │  Create Child  │  │  Create Child  │
///                                                 └────────────────┘  └────────────────┘
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

        // Because we're swapping the parent and child model, we need to look at the related field instead of the parent relation field.
        let reload_child_node =
            utils::insert_node_reload(graph, &parent_relation_field.related_field(), child_node.clone())?;

        // We need to swap the create node (with reload) and the parent because the inlining is done in the parent.
        graph.mark_nodes(&parent_node, &child_node);
        // FIXME: AUMFIDARR
        //        graph.mark_nodes(&parent_node, &reload_child_node);

        let child_model_id = parent_relation_field.related_field().linking_fields();
        let parent_model_id = parent_relation_field.linking_fields();

        // FIXME: AUMFIDARR
        //        graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;

        // We extract the child linking fields in the edge, because after the swap, the child is the new parent.
        graph.create_edge(
            &parent_node,
            &reload_child_node,
            QueryGraphDependency::ParentIds(child_model_id, Box::new(move |mut child_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::AssertionError(format!(
                        "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-many relation."
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_id_into_args(parent_model_id.assimilate(parent_id)?);
                }

                Ok(child_node)
            })),
        )?;
    } else {
        let reload_node = utils::insert_node_reload(graph, parent_relation_field, parent_node.clone())?;

        for create_node in create_nodes {
            let parent_model_id = parent_relation_field.linking_fields();
            let child_model_id = parent_relation_field.related_field().linking_fields();

            graph.create_edge(
                &reload_node,
                &create_node,
                QueryGraphDependency::ParentIds(parent_model_id, Box::new(move |mut child_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::AssertionError(format!(
                            "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-many relation."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = child_node {
                        wq.inject_id_into_args(child_model_id.assimilate(parent_id)?);
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
/// - Parent gets injected with a child on x, because that's what the neste dcreate is supposed to do.
/// - The update runs, the relation is updated.
/// - Now the check runs, because it's dependent on the parent's ID... but the check finds an existing child and fails...
/// ... because we just updated the relation.
///
/// This is why we need to have an extra update at the end if it's inlined on the parent and a non-create.
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    let parent_model_identifier = parent_relation_field.model().primary_identifier();
    let child_model_identifier = parent_relation_field.related_model().primary_identifier();

    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required;
    let child_side_required = child_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();

    // Build-time check
    if !parent_is_create && (parent_side_required && child_side_required) {
        // Both sides are required, which means that we know that there has to be already a parent connected a child (as it exists).
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
    let (parent_node, child_node, relation_field_name) = if relation_inlined_parent {
        // For the injection, we need the name of the field on the inlined side, in this case the parent.
        let relation_field_name = parent_relation_field.name.clone();

        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the ID first.
        graph.mark_nodes(&parent_node, &create_node);
        // let (parent_node, child_node) = utils::swap_nodes(graph, parent_node, create_node)?;

        (parent_node, create_node, relation_field_name)
    } else {
        // For the injection, we need the name of the field on the inlined side, in this case the child.
        let relation_field_name = parent_relation_field.related_field().name.clone();

        (parent_node, create_node, relation_field_name)
    };

    let relation_field_name_pc = relation_field_name.clone();
    graph.create_edge(
         &parent_node,
         &child_node,
         QueryGraphDependency::ParentIds(child_model_identifier.clone(), Box::new(move |mut child_node, mut parent_ids| {
             let parent_id = match parent_ids.pop() {
                 Some(pid) => Ok(pid),
                 None => Err(QueryGraphBuilderError::AssertionError(format!(
                     "[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation."
                 ))),
             }?;

             // We ONLY inject creates here. Check doc comment for explanation.
             if let Node::Query(Query::Write(WriteQuery::CreateRecord(ref mut cr))) = child_node {
                 for (dsf, value) in parent_id.pairs {
                     // FIXME: use relation_field_name_pc works only if there's only one iteration in the loop
                    cr.args.insert(relation_field_name_pc.clone(), value);
                 }
             }

             Ok(child_node)
         })),
     )?;

    // Relation is inlined on the Parent and a non-create.
    // Create an update node for Parent to set the connection to the child.
    // For explanation see doc comment.
    if relation_inlined_parent && !parent_is_create {
        let parent_model = parent_relation_field.model();
        let parent_model_id = parent_model
            .fields()
            .find_singular_id()
            .expect("No id field found")
            .upgrade()
            .unwrap();
        let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);

        graph.create_edge(
             &child_node,
             &update_node,
             QueryGraphDependency::ParentIds(child_model_identifier, Box::new(|mut child_node, mut parent_ids| {
                 let parent_id = match parent_ids.pop() {
                     Some(pid) => Ok(pid),
                     None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation, updating inlined on parent."))),
                 }?;

                 if let Node::Query(Query::Write(ref mut wq)) = child_node {
//                     wq.inject_id_into_args(parent_id);
                     wq.inject_field_arg(relation_field_name, parent_id.single_value());
                 }

                 Ok(child_node)
             })),
         )?;

        graph.create_edge(
             &parent_node,
             &update_node,
             QueryGraphDependency::ParentIds(parent_model_identifier, Box::new(move |mut child_node, mut parent_ids| {
                 let parent_id = match parent_ids.pop() {
                     Some(pid) => Ok(pid),
                     None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested create on a one-to-one relation, updating inlined on parent."))),
                 }?;

                 if let Node::Query(Query::Write(ref mut wq)) = child_node {
//                     wq.inject_id_into_args(parent_id);
                     wq.add_filter(parent_model_id.data_source_field().equals(parent_id.single_value()));
                 }

                 Ok(child_node)
             })),
         )?;
    }

    Ok(())
}
