use super::*;
use crate::{
    constants::args,
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    write::write_args_parser::WriteArgsParser,
    ParsedInputList, ParsedInputValue,
};
use connector::{Filter, IntoFilter};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested create one cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, value, child_model))]
pub fn nested_create(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    // Build all create nodes upfront.
    let creates: Vec<NodeRef> = utils::coerce_vec(value)
        .into_iter()
        .map(|value| create::create_record_node(graph, connector_ctx, Arc::clone(child_model), value.try_into()?))
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
#[tracing::instrument]
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, create_nodes))]
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
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, create_nodes))]
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

        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_relation_field.model().name.clone();
        let child_model_name = parent_relation_field.related_model().name.clone();

        // We extract the child linking fields in the edge, because after the swap, the child is the new parent.
        graph.create_edge(
            &parent_node,
            &child_node,
            QueryGraphDependency::ProjectedDataDependency(child_link, Box::new(move |mut parent_node, mut child_links| {
                let child_link = match child_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to inline the relation on '{}' record) was found for a nested create on one-to-many relation '{}'.",
                        child_model_name, parent_model_name, relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = parent_node {
                    wq.inject_result_into_args(parent_link.assimilate(child_link)?);
                }

                Ok(parent_node)
            })),
        )?;
    } else {
        for create_node in create_nodes {
            let parent_link = parent_relation_field.linking_fields();
            let child_link = parent_relation_field.related_field().linking_fields();

            let relation_name = parent_relation_field.relation().name.clone();
            let parent_model_name = parent_relation_field.model().name.clone();
            let child_model_name = parent_relation_field.related_model().name.clone();

            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ProjectedDataDependency(parent_link, Box::new(move |mut create_node, mut parent_links| {
                    let parent_link = match parent_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record (needed to inline the relation on '{}' record) was found for a nested create on one-to-many relation '{}'.",
                            parent_model_name, child_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = create_node {
                        wq.inject_result_into_args(child_link.assimilate(parent_link)?);
                    }

                    Ok(create_node)
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
/// Important: We cannot inject from `Child Create` to `Parent` if `Parent` is a non-create, as it would cause
/// the following issue (example):
/// - Parent is an update, doesn't have a connected child on relation x.
/// - Parent gets injected with a child on x, because that's what the nested create is supposed to do.
/// - The update runs, the relation is updated.
/// - Now the check runs, because it's dependent on the parent's ID... but the check finds an existing child and fails...
/// ... because we just updated the relation.
///
/// For these reasons, we need to have an extra update at the end if it's inlined on the parent and a non-create.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, create_nodes))]
fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    mut create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    let parent_is_create = utils::node_is_create(graph, &parent_node);
    let child_relation_field = parent_relation_field.related_field();
    let parent_side_required = parent_relation_field.is_required();
    let child_side_required = child_relation_field.is_required();
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
    let ((extractor_model, extractor), (assimilator_model, assimilator)) = if relation_inlined_parent {
        // We need to swap the read node and the parent because the inlining is done in the parent, and we need to fetch the ID first.
        graph.mark_nodes(&parent_node, &create_node);
        (
            (child_relation_field.model(), child_link.clone()),
            (parent_relation_field.model(), parent_link),
        )
    } else {
        (
            (parent_relation_field.model(), parent_link),
            (child_relation_field.model(), child_link.clone()),
        )
    };

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_model_name = extractor_model.name.clone();
    let child_model_name = assimilator_model.name.clone();

    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ProjectedDataDependency(extractor, Box::new(move |mut child_node, mut links| {
            let link = match links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{}' record (needed to inline the relation with create on '{}' record) was found for a nested create on one-to-one relation '{}'.",
                    parent_model_name, child_model_name, relation_name
                ))),
            }?;

            // We ONLY inject for creates here. Check end of doc comment for explanation.
            if let Node::Query(Query::Write(ref mut q @ WriteQuery::CreateRecord(_))) = child_node {
                q.inject_result_into_args(assimilator.assimilate(link)?);
            }

            Ok(child_node)
        })),
    )?;

    // Relation is inlined on the Parent and a non-create.
    // Create an update node for Parent to set the connection to the child.
    // For explanation see end of doc comment.
    if relation_inlined_parent && !parent_is_create {
        let parent_model = parent_relation_field.model();
        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_model.name.clone();
        let child_model_name = parent_relation_field.related_model().name.clone();
        let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);
        let parent_link = parent_relation_field.linking_fields();

        graph.create_edge(
            &create_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(child_link, Box::new(move |mut update_node, mut child_links| {
                let child_link = match child_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to inline the relation with an update on '{}' record) was found for a nested create on one-to-one relation '{}'.",
                        child_model_name, parent_model_name, relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_node {
                    wq.inject_result_into_args(parent_link.assimilate(child_link)?);
                }

                Ok(update_node)
            })),
         )?;

        let parent_model_identifier = parent_relation_field.model().primary_identifier();
        let relation_name = parent_relation_field.relation().name.clone();
        let parent_model_name = parent_relation_field.model().name.clone();

        graph.create_edge(
            &parent_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(parent_model_identifier, Box::new(move |mut update_node, mut parent_ids| {
                let parent_id = match parent_ids.pop() {
                    Some(pid) => Ok(pid),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to update the relation on '{}' record) was found for a nested create on one-to-one relation '{}'.",
                        &parent_model_name, parent_model_name, relation_name
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_node {
                    wq.add_filter(parent_id.filter());
                }

                Ok(update_node)
            })),
         )?;
    }

    Ok(())
}

#[tracing::instrument(skip(graph, parent_node, parent_relation_field, value, child_model))]
pub fn nested_create_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    // Nested input is an object of { data: [...], skipDuplicates: bool }
    let mut obj: ParsedInputMap = value.try_into()?;

    let data_list: ParsedInputList = obj.remove(args::DATA).unwrap().try_into()?;
    let skip_duplicates: bool = match obj.remove(args::SKIP_DUPLICATES) {
        Some(val) => val.try_into()?,
        None => false,
    };

    let args = data_list
        .into_iter()
        .map(|data_value| {
            let data_map = data_value.try_into()?;
            let mut args = WriteArgsParser::from(&child_model, data_map)?.args;

            args.add_datetimes(&child_model);
            Ok(args)
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    let query = CreateManyRecords {
        model: Arc::clone(child_model),
        args,
        skip_duplicates,
    };

    let create_node = graph.create_node(Query::Write(WriteQuery::CreateManyRecords(query)));

    // We know that the id must be inlined on the child, so we need the parent link to inline it.
    let linking_fields = parent_relation_field.linking_fields();
    let child_linking_fields = parent_relation_field.related_field().linking_fields();

    let relation_name = parent_relation_field.relation().name.clone();
    let parent_model_name = parent_relation_field.model().name.clone();
    let child_model_name = child_model.name.clone();

    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ProjectedDataDependency(
            linking_fields,
            Box::new(move |mut create_many_node, mut parent_links| {
                // There can only be one parent.
                let parent_link = match parent_links.pop() {
                    Some(p) => Ok(p),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{}' record (needed to inline the relation on '{}' record) was found for a nested createMany on relation '{}'.",
                        parent_model_name, child_model_name, relation_name
                    ))),
                }?;

                // Inject the parent id into all nested records.
                if let Node::Query(Query::Write(WriteQuery::CreateManyRecords(ref mut cmr))) = create_many_node {
                    cmr.inject_result_into_all(child_linking_fields.assimilate(parent_link)?);
                }

                Ok(create_many_node)
            }),
        ),
    )?;

    Ok(())
}
