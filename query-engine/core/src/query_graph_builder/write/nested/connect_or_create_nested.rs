use super::*;
use crate::query_graph_builder::write::utils::coerce_vec;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    InputAssertions, ParsedInputMap, ParsedInputValue,
};
use connector::{Filter, IdFilter};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

/// Handles nested connect cases.
///
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_connect_or_create(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();
    let values = utils::coerce_vec(value);

    if relation.is_many_to_many() {
        handle_many_to_many(graph, parent_node, parent_relation_field, values, child_model)
    } else if relation.is_one_to_many() {
        handle_one_to_many(graph, parent_node, parent_relation_field, values, child_model)
    } else {
        handle_one_to_one(graph, parent_node, parent_relation_field, values, child_model)
    }
}

/// Handles a nested connect-or-create many-to-many relation case.
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
/// └─▶│     Connect     │   │  Create Child   │  │
///    └─────────────────┘   └─────────────────┘  │
///                                   │           │
///                                   │           │
///                                   │           │
///                                   ▼           │
///                          ┌─────────────────┐  │
///                          │     Connect     │◀─┘
///                          └─────────────────┘
/// ```
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    todo!()
}

fn handle_one_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    todo!()
}

fn handle_one_to_one(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    values: Vec<ParsedInputValue>,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    todo!()
}
