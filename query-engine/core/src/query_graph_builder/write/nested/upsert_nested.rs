use super::*;
use crate::query_graph_builder::write::utils::coerce_vec;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue,
};
use connector::{Filter, IntoFilter};
use prisma_models::RelationFieldRef;
use schema_builder::constants::args;
use std::{convert::TryInto, sync::Arc};

/// Handles a nested upsert.
/// The constructed query graph can have different shapes based on the relation
/// of parent and child and where relations are inlined:
///
/// Many-to-many relation:
/// ```text
///                           ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///                                 Parent       ───────────────────┐
///                           └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │         │
///                                    │                            │
///                                    │                  │         │
///                                    ▼                            │
///                           ┌─────────────────┐         │         │
/// ┌───────────┬─────────────│   Read Child    │                   │
/// │           │             └─────────────────┘         │         │
/// │           │                      │                  ▼         │
/// │           │                      │           ┌ ─ ─ ─ ─ ─ ─    │
/// │           │                      │               Result   │   │
/// │           │                      │           └ ─ ─ ─ ─ ─ ─    │
/// │           │                      ▼                            │
/// │           │             ┌─────────────────┐                   │
/// │           │             │   If (exists)   │───────┐           │
/// │ ┌ ─ ─ ─ ─ ▼ ─ ─ ─ ─ ┐   └─────────────────┘       │           │
/// │  ┌─────────────────┐             │                │           │
/// │ ││    Join Node    │◀────Then────┘                │           │
/// │  └─────────────────┘                              │           │
/// │ │         │         │                             │           │
/// │           │                                       │           │
/// │ │         ▼         │                             │           │
/// │  ┌─────────────────┐                              ▼           │
/// │ ││ Insert onUpdate ││                    ┌─────────────────┐  │
/// │  │emulation subtree│                     │  Create Child   │  │
/// │ ││for all relations││                    └─────────────────┘  │
/// │  │ pointing to the │                              │           │
/// │ ││   Child model   ││                             │           │
/// │  └─────────────────┘                              │           │
/// │ └ ─ ─ ─ ─ ┬ ─ ─ ─ ─ ┘                             ▼           │
/// │           │                              ┌─────────────────┐  │
/// │           │                              │     Connect     │◀─┘
/// │           ▼                              └─────────────────┘
/// │  ┌─────────────────┐
/// └─▶│  Update Child   │
///    └─────────────────┘
/// ```
///
/// One-to-x relation:
/// ```text
///    Inlined in parent:                                                                      Inlined in child:
///                                                                                                                          ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///                                     ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐                                                                        Parent       ────────────────────┐
///                                           Parent       ────────┬──────────┐                                              └ ─ ─ ─ ─ ─ ─ ─ ─ ┘           │        │
///                                     └ ─ ─ ─ ─ ─ ─ ─ ─ ┘                   │                                                       │                             │
///                                              │                 │          │                                                       │                    │        │
///                                              │                            │                                                       │                             │
///                                              │                 │          │                                                       ▼                    ▼        │
///                                              ▼                 ▼          │                                              ┌─────────────────┐    ┌ ─ ─ ─ ─ ─ ─   │
///                                     ┌─────────────────┐ ┌ ─ ─ ─ ─ ─ ─     │                ┌─────────────────────────────│   Read Child    │        Result   │  │
/// ┌───────────────────────────────────│   Read Child    │     Result   │    │                │                             └─────────────────┘    └ ─ ─ ─ ─ ─ ─   │
/// │                                   └─────────────────┘ └ ─ ─ ─ ─ ─ ─     │                │                                      │                             │
/// │                                            │                            │                │                                      │                             │
/// │                                            │                            │                │ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐                ▼                             │
/// │                                            │                            │                │  ┌─────────────────┐        ┌─────────────────┐                    │
/// │ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐                      ▼                            │                │ ││    Join node    │◀───────│   If (exists)   │────────┐           │
/// │  ┌─────────────────┐              ┌─────────────────┐                   │                │  └─────────────────┘        └─────────────────┘        │           │
/// │ ││    Join node    │◀────Then─────│   If (exists)   │───────┐           │                │ │         │         │                                  │           │
/// │  └─────────────────┘              └─────────────────┘       │           │                │           ▼                                            │           │
/// │ │         │         │                                       │           │                │ │┌─────────────────┐│                                  │           │
/// │           ▼                                                 │           │                │  │ Insert onUpdate │                                   │           │
/// │ │┌─────────────────┐│                                       │           │                │ ││emulation subtree││                                  │           │
/// │  │ Insert onUpdate │                                        ▼           │                │  │for all relations│                                   ▼           │
/// │ ││emulation subtree││                              ┌─────────────────┐  │                │ ││ pointing to the ││                         ┌─────────────────┐  │
/// │  │for all relations│                               │  Create Child   │  │                │  │   Child model   │                          │  Create Child   │◀─┘
/// │ ││ pointing to the ││                              └─────────────────┘  │                │ │└─────────────────┘│                         └─────────────────┘
/// │  │   Child model   │                                        │           │                │  ─ ─ ─ ─ ─│─ ─ ─ ─ ─
/// │ │└─────────────────┘│                                       │           │                │           │
/// │  ─ ─ ─ ─ ─│─ ─ ─ ─ ─                                        │           │                │           │
/// │           │                                                 ▼           │                │           ▼
/// │           ▼                                        ┌─────────────────┐  │                │  ┌─────────────────┐
/// │  ┌─────────────────┐                               │  Update Parent  │◀─┘                └─▶│  Update Child   │
/// └─▶│  Update Child   │                               └─────────────────┘                      └─────────────────┘
///    └─────────────────┘
/// ```
/// Todo split this mess up and clean up the code.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, value))]
pub fn nested_upsert(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let child_model_identifier = child_model.primary_identifier();

    for value in coerce_vec(value) {
        let parent_link = parent_relation_field.linking_fields();
        let child_link = parent_relation_field.related_field().linking_fields();

        let mut as_map: ParsedInputMap = value.try_into()?;
        let create_input = as_map.remove(args::CREATE).expect("create argument is missing");
        let update_input = as_map.remove(args::UPDATE).expect("update argument is missing");

        // Read child(ren) node
        let filter: Filter = if parent_relation_field.is_list() {
            let where_input: ParsedInputMap = as_map
                .remove(args::WHERE)
                .expect("where argument is missing")
                .try_into()?;
            extract_unique_filter(where_input, &child_model)?
        } else {
            Filter::empty()
        };

        let read_children_node =
            utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, filter)?;

        let if_node = graph.create_node(Flow::default_if());
        let create_node =
            create::create_record_node(graph, connector_ctx, Arc::clone(&child_model), create_input.try_into()?)?;
        let update_node = update::update_record_node(
            graph,
            connector_ctx,
            Filter::empty(),
            Arc::clone(&child_model),
            update_input.try_into()?,
        )?;

        graph.create_edge(
            &read_children_node,
            &if_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                Box::new(|if_node, child_ids| {
                    if let Node::Flow(Flow::If(_)) = if_node {
                        Ok(Node::Flow(Flow::If(Box::new(move || !child_ids.is_empty()))))
                    } else {
                        Ok(if_node)
                    }
                }),
            ),
        )?;

        let relation_name = parent_relation_field.relation().name.clone();
        let child_model_name = child_model.name.clone();

        graph.create_edge(
            &read_children_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(
                child_model_identifier.clone(),
                Box::new(move |mut update_node, mut child_ids| {
                    if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut wq))) = update_node {
                        let child_id = match child_ids.pop() {
                            Some(id) => Ok(id),
                            None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                                "No '{}' record (needed for nested update `where` on exists) was found for a nested upsert on relation '{}'.",
                                child_model_name, relation_name
                            ))),
                        }?;

                        wq.add_filter(child_id.filter());
                    }

                    Ok(update_node)
                }),
            ),
        )?;

        // In case the connector doesn't support referential integrity, we add a subtree to the graph that emulates the ON_UPDATE referential action.
        // When that's the case, we create an intermediary node to which we connect all the nodes reponsible for emulating the referential action
        // Then, we connect the if node to that intermediary emulation node. This enables performing the emulation only in case the graph traverses
        // the update path (if the children already exists and goes to the THEN node).
        // It's only after we've executed the emulation that it'll traverse the update node, hence the ExecutionOrder between
        // the emulation node and the update node.
        let then_node = if let Some(emulation_node) = utils::insert_emulated_on_update_with_intermediary_node(
            graph,
            connector_ctx,
            &child_model,
            &read_children_node,
            &update_node,
        )? {
            graph.create_edge(&emulation_node, &update_node, QueryGraphDependency::ExecutionOrder)?;

            emulation_node
        } else {
            update_node
        };

        graph.create_edge(&if_node, &then_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

        // Specific handling based on relation type and inlining side.
        if parent_relation_field.relation().is_many_to_many() {
            // Many to many only needs a connect node.
            connect::connect_records_node(graph, &parent_node, &create_node, &parent_relation_field, 1)?;
        } else if parent_relation_field.is_inlined_on_enclosing_model() {
            let parent_model = parent_relation_field.model();
            let parent_model_name = parent_model.name.clone();
            let relation_name = parent_relation_field.relation().name.clone();
            let parent_model_id = parent_model.primary_identifier();
            let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);

            // Edge to retrieve the finder
            graph.create_edge(
                &parent_node,
                &update_node,
                QueryGraphDependency::ProjectedDataDependency(parent_model_id, Box::new(move |mut update_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record (needed to update inlined relation on '{}') was found for a nested upsert on relation '{}'.",
                            &parent_model_name, parent_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.add_filter(parent_id.filter());
                    }

                    Ok(update_node)
                })),
            )?;

            let parent_model_name = parent_relation_field.model().name.clone();
            let child_model_name = parent_relation_field.related_model().name.clone();
            let relation_name = parent_relation_field.relation().name.clone();

            // Edge to retrieve the child ID to inject
            graph.create_edge(
                &create_node,
                &update_node,
                QueryGraphDependency::ProjectedDataDependency(child_link.clone(), Box::new(move |mut update_node, mut child_links| {
                    let child_link = match child_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record (needed to update inlined relation on '{}') was found for a nested upsert on relation '{}'.",
                            child_model_name, parent_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_result_into_args(parent_link.assimilate(child_link)?);
                    }

                    Ok(update_node)
                })),
            )?;
        } else {
            let parent_model_name = parent_relation_field.model().name.clone();
            let child_model_name = parent_relation_field.related_model().name.clone();
            let relation_name = parent_relation_field.relation().name.clone();

            // Inlined on child
            // Edge to retrieve the child ID to inject (inject into the create)
            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ProjectedDataDependency(parent_link, Box::new(move |mut create_node, mut parent_links| {
                    let parent_link = match parent_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{}' record (needed to update inlined relation on '{}') was found for a nested upsert on relation '{}'.",
                            parent_model_name, child_model_name, relation_name
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = create_node {
                        wq.inject_result_into_args(child_link.assimilate(parent_link)?);
                    }

                    Ok(create_node)
                })),
            )?;
        }
    }

    Ok(())
}
