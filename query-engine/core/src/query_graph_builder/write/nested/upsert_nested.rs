use super::*;
use crate::query_graph_builder::write::utils::coerce_vec;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue,
};
use connector::{Filter, IdFilter};
use prisma_models::RelationFieldRef;
use std::{convert::TryInto, sync::Arc};

/// Handles a nested upsert.
/// The constructed query graph can have different shapes based on the relation
/// of parent and child and where relations are inlined:
///
/// Many-to-many relation:
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///          Parent       ────────────────────────┐
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │              │
///             │                                 │
///             │                  │              │
///             │                                 │
///             ▼                  ▼              │
///    ┌─────────────────┐  ┌ ─ ─ ─ ─ ─ ─         │
/// ┌──│   Read Child    │      Result   │        │
/// │  └─────────────────┘  └ ─ ─ ─ ─ ─ ─         │
/// │           │                                 │
/// │           │                                 │
/// │           │                                 │
/// │           ▼                                 │
/// │  ┌─────────────────┐                        │
/// │  │   If (exists)   │────────────┐           │
/// │  └─────────────────┘            │           │
/// │           │                     │           │
/// │           │                     │           │
/// │           │                     │           │
/// │           ▼                     ▼           │
/// │  ┌─────────────────┐   ┌─────────────────┐  │
/// └─▶│  Update Child   │   │  Create Child   │  │
///    └─────────────────┘   └─────────────────┘  │
///                                   │           │
///                                   │           │
///                                   │           │
///                                   ▼           │
///                          ┌─────────────────┐  │
///                          │     Connect     │◀─┘
///                          └─────────────────┘
/// ```
///
/// One-to-x relation:
/// ```text
///    Inlined in parent:                                     Inlined in child:
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐                                    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///          Parent       ────────────────────────┐                 Parent       ────────────────────────┐
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │              │           └ ─ ─ ─ ─ ─ ─ ─ ─ ┘         │              │
///             │                                 │                    │                                 │
///             │                  │              │                    │                  │              │
///             │                                 │                    │                                 │
///             ▼                  ▼              │                    ▼                  ▼              │
///    ┌─────────────────┐  ┌ ─ ─ ─ ─ ─ ─         │           ┌─────────────────┐  ┌ ─ ─ ─ ─ ─ ─         │
/// ┌──│   Read Child    │      Result   │        │        ┌──│   Read Child    │      Result   │        │
/// │  └─────────────────┘  └ ─ ─ ─ ─ ─ ─         │        │  └─────────────────┘  └ ─ ─ ─ ─ ─ ─         │
/// │           │                                 │        │           │                                 │
/// │           │                                 │        │           │                                 │
/// │           │                                 │        │           │                                 │
/// │           ▼                                 │        │           ▼                                 │
/// │  ┌─────────────────┐                        │        │  ┌─────────────────┐                        │
/// │  │   If (exists)   │────────────┐           │        │  │   If (exists)   │────────────┐           │
/// │  └─────────────────┘            │           │        │  └─────────────────┘            │           │
/// │           │                     │           │        │           │                     │           │
/// │           │                     │           │        │           │                     │           │
/// │           │                     │           │        │           │                     │           │
/// │           ▼                     ▼           │        │           ▼                     ▼           │
/// │  ┌─────────────────┐   ┌─────────────────┐  │        │  ┌─────────────────┐   ┌─────────────────┐  │
/// └─▶│  Update Child   │   │  Create Child   │  │        └─▶│  Update Child   │   │  Create Child   │◀─┘
///    └─────────────────┘   └─────────────────┘  │           └─────────────────┘   └─────────────────┘
///                                   │           │
///                                   │           │
///                                   │           │
///                                   ▼           │
///                          ┌─────────────────┐  │
///                          │  Update Parent  │◀─┘
///                          └─────────────────┘
/// ```
/// Todo split this mess up and clean up the code.
pub fn nested_upsert(
    graph: &mut QueryGraph,
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
        let create_input = as_map.remove("create").expect("create argument is missing");
        let update_input = as_map.remove("update").expect("update argument is missing");

        // Read child(ren) node
        let filter: Filter = if parent_relation_field.is_list {
            let where_input: ParsedInputMap = as_map.remove("where").expect("where argument is missing").try_into()?;
            extract_unique_filter(where_input, &child_model)?
        } else {
            Filter::empty()
        };

        let read_children_node =
            utils::insert_find_children_by_parent_node(graph, &parent_node, parent_relation_field, filter)?;

        let if_node = graph.create_node(Flow::default_if());
        let create_node = create::create_record_node(graph, Arc::clone(&child_model), create_input.try_into()?)?;
        let update_node = update::update_record_node(
            graph,
            Filter::empty(),
            Arc::clone(&child_model),
            update_input.try_into()?,
        )?;

        graph.create_edge(
            &read_children_node,
            &if_node,
            QueryGraphDependency::ParentProjection(
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

        graph.create_edge(
            &read_children_node,
            &update_node,
            QueryGraphDependency::ParentProjection(child_model_identifier.clone(), Box::new(move |mut update_node, mut child_ids| {
                if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut wq))) = update_node {
                    let child_id = match child_ids.pop() {
                        Some(id) => Ok(id),
                        None => Err(QueryGraphBuilderError::AssertionError("[Query Graph] Expected a valid parent ID to be present for a nested update in a nested upsert.".to_string())),
                    }?;

                    wq.add_filter(child_id.filter());
                }

                Ok(update_node)
            })),
        )?;

        graph.create_edge(&if_node, &update_node, QueryGraphDependency::Then)?;
        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

        // Specific handling based on relation type and inlining side.
        if parent_relation_field.relation().is_many_to_many() {
            // Many to many only needs a connect node.
            connect::connect_records_node(graph, &parent_node, &create_node, &parent_relation_field, 1)?;
        } else if parent_relation_field.is_inlined_on_enclosing_model() {
            let parent_model = parent_relation_field.model();
            let parent_model_id = parent_model.primary_identifier();
            let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);

            // Edge to retrieve the finder
            graph.create_edge(
                &parent_node,
                &update_node,
                QueryGraphDependency::ParentProjection(parent_model_id, Box::new(move |mut update_node, mut parent_ids| {
                    let parent_id = match parent_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::AssertionError("[Query Graph] Expected a valid parent ID to be present to retrieve the finder for a parent update in a nested upsert.".to_string())),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.add_filter(parent_id.filter());
                    }

                    Ok(update_node)
                })),
            )?;

            // Edge to retrieve the child ID to inject
            graph.create_edge(
                &create_node,
                &update_node,
                QueryGraphDependency::ParentProjection(child_link.clone(), Box::new(move |mut update_node, mut child_links| {
                    let child_link = match child_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::AssertionError("[Query Graph] Expected a valid parent ID to be present to retrieve the ID to inject for a parent update in a nested upsert.".to_string())),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = update_node {
                        wq.inject_projection_into_args(parent_link.assimilate(child_link)?);
                    }

                    Ok(update_node)
                })),
            )?;
        } else {
            // Inlined on child
            // Edge to retrieve the child ID to inject (inject into the create)
            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ParentProjection(parent_link, Box::new(move |mut create_node, mut parent_links| {
                    let parent_link = match parent_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::AssertionError("[Query Graph] Expected a valid parent ID to be present to retrieve the ID to inject into the child create in a nested upsert.".to_string())),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = create_node {
                        wq.inject_projection_into_args(child_link.assimilate(parent_link)?);
                    }

                    Ok(create_node)
                })),
            )?;
        }
    }

    Ok(())
}
