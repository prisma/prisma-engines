use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ConnectorContext, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{DatasourceFieldName, Filter, RecordFilter, WriteArgs, WriteOperation};
use datamodel::ReferentialAction;
use indexmap::IndexMap;
use prisma_models::{FieldSelection, ModelRef, PrismaValue, RelationFieldRef, SelectionResult};
use std::sync::Arc;

/// Coerces single values (`ParsedInputValue::Single` and `ParsedInputValue::Map`) into a vector.
/// Simply unpacks `ParsedInputValue::List`.
pub fn coerce_vec(val: ParsedInputValue) -> Vec<ParsedInputValue> {
    match val {
        ParsedInputValue::List(l) => l,
        m @ ParsedInputValue::Map(_) => vec![m],
        single => vec![single],
    }
}

pub fn node_is_create(graph: &QueryGraph, node: &NodeRef) -> bool {
    matches!(
        graph.node_content(node).unwrap(),
        Node::Query(Query::Write(WriteQuery::CreateRecord(_)))
    )
}

/// Produces a non-failing read query that fetches the requested selection of records for a given filterable.
pub fn read_ids_infallible<T>(model: ModelRef, selection: FieldSelection, filter: T) -> Query
where
    T: Into<Filter>,
{
    let selected_fields = get_selected_fields(&model, selection);
    let filter: Filter = filter.into();

    let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name: "read_ids_infallible".into(), // this name only eases debugging
        alias: None,
        model: model.clone(),
        args: (model, filter).into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
        aggregation_selections: vec![],
    });

    Query::Read(read_query)
}

fn get_selected_fields(model: &ModelRef, selection: FieldSelection) -> FieldSelection {
    // Always fetch the primary identifier as well.
    let primary_model_id = model.primary_identifier();

    if selection != primary_model_id {
        primary_model_id.merge(selection)
    } else {
        selection
    }
}

/// Adds a read query to the query graph that finds related records by parent ID.
/// Connects the parent node and the read node with an edge, which takes care of the
/// node transformation based on the parent ID.
///
/// Optionally, a filter can be passed that narrows down the child selection.
///
/// Returns a `NodeRef` to the newly created read node.
///
/// ```text
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///       Parent
/// └ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///          │
///          ▼
/// ┌─────────────────┐
/// │  Read Children  │
/// └─────────────────┘
///```
///
/// ## Example
/// Given two models, `Blog` and `Post`, where a blog has many posts, and a post has one block.
///
/// If the caller wants to query posts by blog ID:
/// - `parent_node` needs to return a blog ID during execution.
/// - `parent_relation_field` is the field on the `Blog` model, e.g. `posts`.
/// - `filter` narrows down posts, e.g. posts where their titles start with a given string.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field, filter))]
pub fn insert_find_children_by_parent_node<T>(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: T,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<Filter>,
{
    let parent_model_id = parent_relation_field.model().primary_identifier();
    let parent_linking_fields = parent_relation_field.linking_fields();
    let selection = parent_model_id.merge(parent_linking_fields);
    let child_model = parent_relation_field.related_model();

    let selected_fields = get_selected_fields(
        &parent_relation_field.related_model(),
        parent_relation_field.related_field().linking_fields(),
    );

    let read_children_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "find_children_by_parent".to_owned(),
        alias: None,
        parent_field: Arc::clone(parent_relation_field),
        parent_results: None,
        args: (child_model, filter).into(),
        selected_fields,
        aggregation_selections: vec![],
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent_node,
        &read_children_node,
        QueryGraphDependency::ProjectedDataDependency(
            selection,
            Box::new(|mut read_children_node, selections| {
                if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = read_children_node {
                    rq.parent_results = Some(selections);
                };

                Ok(read_children_node)
            }),
        ),
    )?;

    Ok(read_children_node)
}

/// Creates an update many records query node and adds it to the query graph.
/// Used to have a skeleton update node in the graph that can be further transformed during query execution based
/// on available information.
///
/// No edges are created.
pub fn update_records_node_placeholder<T>(graph: &mut QueryGraph, filter: T, model: ModelRef) -> NodeRef
where
    T: Into<Filter>,
{
    let args = WriteArgs::new();
    let filter = filter.into();
    let record_filter = filter.into();

    let ur = UpdateManyRecords {
        model,
        record_filter,
        args,
    };

    graph.create_node(Query::Write(WriteQuery::UpdateManyRecords(ur)))
}

/// Inserts checks and disconnects for existing models for a 1:1 relation.
/// Expects that the parent node returns a valid ID for the model the `parent_relation_field` is located on.
///
/// Params:
/// `parent_node`: Node that provides the parent id for the find query and where the checks are appended to in the graph.
/// `parent_relation_field`: Field on the parent model to find children.
///
/// ```text
/// ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///           Parent         │
/// └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///              :
///              ▼
/// ┌────────────────────────┐
/// │      Read related      │──┐
/// └────────────────────────┘  │
///              │              │
///              ▼              │
/// ┌────────────────────────┐  │
/// │ If p > 0 && c. inlined │  │
/// └────────────────────────┘  │
///         then │              │
///              ▼              │
/// ┌────────────────────────┐  │
/// │    Update ex. child    │◀─┘
/// └────────────────────────┘
/// ```
///
/// The edge between `Read Related` and `If` fails on node count > 0 if child side is required.
///
/// We only need to actually update ("disconnect") the existing model if
/// the relation is also inlined on that models side, so we put that check into the if flow.
///
/// Returns a `NodeRef` to the "Read Related" node in the graph illustrated above.
#[tracing::instrument(skip(graph, parent_node, parent_relation_field))]
pub fn insert_existing_1to1_related_model_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let child_model_identifier = parent_relation_field.related_model().primary_identifier();
    let child_linking_fields = parent_relation_field.related_field().linking_fields();

    let child_model = parent_relation_field.related_model();
    let child_side_required = parent_relation_field.related_field().is_required();
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let rf = Arc::clone(&parent_relation_field);

    // Note: Also creates the edge between `parent` and the new node.
    let read_existing_children =
        insert_find_children_by_parent_node(graph, &parent_node, &parent_relation_field, Filter::empty())?;

    let update_existing_child = update_records_node_placeholder(graph, Filter::empty(), child_model);
    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_existing_children,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |if_node, child_ids| {
                // If the other side ("child") requires the connection, we need to make sure that there isn't a child already connected
                // to the parent, as that would violate the other childs relation side.
                if !child_ids.is_empty() && child_side_required {
                    return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
                }

                if let Node::Flow(Flow::If(_)) = if_node {
                    // If the relation is inlined in the parent, we need to update the old parent and null out the relation (i.e. "disconnect").
                    Ok(Node::Flow(Flow::If(Box::new(move || {
                        !relation_inlined_parent && !child_ids.is_empty()
                    }))))
                } else {
                    unreachable!()
                }
            }),
        ),
    )?;

    let relation_name = parent_relation_field.relation().name.clone();

    graph.create_edge(&if_node, &update_existing_child, QueryGraphDependency::Then)?;
    graph.create_edge(
        &read_existing_children,
        &update_existing_child,
        QueryGraphDependency::ProjectedDataDependency(child_model_identifier, Box::new(move |mut update_existing_child, mut child_ids| {
            // This has to succeed or the if-then node wouldn't trigger.
            let child_id = match child_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No parent record (needed to update the previous parent) was found for a nested connect on relation '{}' .",
                    relation_name
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = update_existing_child {
                wq.inject_result_into_args(SelectionResult::from(&child_linking_fields));
            }

            if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut ur))) = update_existing_child {
                ur.record_filter = child_id.into();
            }

            Ok(update_existing_child)
         })))?;

    Ok(read_existing_children)
}

/// Inserts emulated referential actions for `onDelete` into the graph.
/// All relations that refer to the `model` row(s) being deleted are checked for their desired emulation and inserted accordingly.
/// Right now, supported modes are `Restrict` and `SetNull` (cascade will follow).
/// Those checks fail at runtime and are inserted between `parent_node` and `child_node`.
///
/// This function is usually part of a delete (`deleteOne` or `deleteMany`).
/// Expects `parent_node` to return one or more IDs (for records of `model`) to be checked.
///
/// Resulting graph (all emulations):
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
/// ┌ ─│  (ids to delete)    ─────────────────┬─────────────────────────────┬────────────────────────────────────────┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘                 │                             │                                        │
/// │             │                           │                             │                                        │
///               ▼                           ▼                             ▼                                        ▼
/// │  ┌────────────────────┐      ┌────────────────────┐        ┌────────────────────┐                   ┌────────────────────┐
///    │Find Connected Model│      │Find Connected Model│        │Find Connected Model│                   │Find Connected Model│
/// │  │    A (Restrict)    │      │    B (Restrict)    │     ┌──│    C (SetNull)     │                ┌──│    D (Cascade)     │
///    └────────────────────┘      └────────────────────┘     │  └────────────────────┘                │  └────────────────────┘
/// │             │                           │               │             │                          │             │
///        Fail if│> 0                 Fail if│> 0            │             ▼                          │             │
/// │             │                           │               │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │             ▼
///               ▼                           ▼               │  ┌────────────────────┐ │              │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │  ┌────────────────────┐      ┌────────────────────┐     ││ │  Insert onUpdate   │                │  ┌────────────────────┐ │
///    │       Empty        │      │       Empty        │     │  │ emulation subtree  │ │              ││ │  Insert onDelete   │
/// │  └────────────────────┘      └────────────────────┘     ││ │for relations using │                │  │ emulation subtree  │ │
///               │                           │               │  │the foreign key that│ │              ││ │ for all relations  │
/// │             │                           │               ││ │    was updated.    │                │  │   pointing to D.   │ │
///               │                           │               │  └────────────────────┘ │              ││ └────────────────────┘
/// │             │                           │               │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │                           │               │             │                          │             │
/// │             │                           │               │             │                          │             │
///               ▼                           │               │             ▼                          │             ▼
/// │  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                  │               │  ┌────────────────────┐                │  ┌────────────────────┐
///  ─▶        Delete       │◀────────────────┘               │  │ Update Cs (set FK  │                └─▶│     Delete Cs      │
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                                  └─▶│       null)        │                   └────────────────────┘
///               ▲                                              └────────────────────┘                              │
///               │                                                         │                                        │
///               └─────────────────────────────────────────────────────────┴────────────────────────────────────────┘
/// ```
#[tracing::instrument(skip(graph, model_to_delete, parent_node, child_node))]
pub fn insert_emulated_on_delete(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model_to_delete: &ModelRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    // If the connector uses the `ReferentialIntegrity::ForeignKeys` mode, we do not do any checks / emulation.
    if connector_ctx.referential_integrity.uses_foreign_keys() {
        return Ok(());
    }

    // If the connector uses the `ReferentialIntegrity::Prisma` mode, then the emulation will kick in.
    let internal_model = model_to_delete.internal_data_model();
    let relation_fields = internal_model.fields_pointing_to_model(model_to_delete, false);

    for rf in relation_fields {
        match rf.relation().on_delete() {
            ReferentialAction::NoAction => continue, // Explicitly do nothing.
            ReferentialAction::Restrict => emulate_restrict(graph, &rf, parent_node, child_node)?,
            ReferentialAction::SetNull => {
                emulate_on_delete_set_null(graph, connector_ctx, &rf, parent_node, child_node)?
            }
            ReferentialAction::Cascade => {
                emulate_on_delete_cascade(graph, &rf, connector_ctx, model_to_delete, parent_node, child_node)?
            }
            x => panic!("Unsupported referential action emulation: {}", x),
        }
    }

    Ok(())
}

/// Inserts restrict emulations into the graph between `parent_node` and `child_node`.
/// `relation_field` is the relation field pointing to the model to be deleted/updated.
///
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
/// ┌ ─│  (ids to del/upd)
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │             │
///               ▼
/// │  ┌────────────────────┐
///    │Find Connected Model│
/// │  │     (Restrict)     │
///    └────────────────────┘
/// │             │
///        Fail if│> 0
/// │             │
///               ▼
/// │  ┌────────────────────┐
///    │       Empty        │
/// │  └────────────────────┘
///               │
/// │             ▼
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// └ ▶   Delete / Update   │
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
pub fn emulate_restrict(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let noop_node = graph.create_node(Node::Empty);
    let relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().primary_identifier();
    let read_node = insert_find_children_by_parent_node(graph, parent_node, &relation_field, Filter::empty())?;

    graph.create_edge(
        &read_node,
        &noop_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            Box::new(move |noop_node, child_ids| {
                if !child_ids.is_empty() {
                    return Err(QueryGraphBuilderError::RelationViolation((relation_field).into()));
                }

                Ok(noop_node)
            }),
        ),
    )?;

    // Edge from empty node to the child (delete).
    graph.create_edge(&noop_node, child_node, QueryGraphDependency::ExecutionOrder)?;

    Ok(())
}

/// Inserts cascade emulations into the graph between `parent_node` and `child_node`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the deletion emulation to ensure that subsequent deletions are handled correctly as well.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
///    │  (ids to delete)    ─ ┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │            │
///               ▼
///    ┌────────────────────┐  │
///    │Find Connected Model│
/// ┌──│     (Cascade)      │  │
/// │  └────────────────────┘
/// │             │            │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │  ┌────────────────────┐ │
/// ││ │  Insert onDelete   │  │
/// │  │ emulation subtree  │ │
/// ││ │ for all relations  │  │
/// │  │  pointing to the   │ │
/// ││ │  Connected Model.  │  │
/// │  └────────────────────┘ │
/// │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │             │
/// │             ▼            │
/// │  ┌────────────────────┐
/// └─▶│  Delete children   │  │
///    └────────────────────┘
///               │            │
///               ▼
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─   │
///            Delete       │◀─
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
pub fn emulate_on_delete_cascade(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef, // This is the field _on the other model_ for cascade.
    connector_ctx: &ConnectorContext,
    _model: &ModelRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().primary_identifier();

    // Records that need to be deleted for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, parent_node, &parent_relation_field, Filter::empty())?;

    let delete_query = WriteQuery::DeleteManyRecords(DeleteManyRecords {
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
    });

    let delete_dependents_node = graph.create_node(Query::Write(delete_query));

    insert_emulated_on_delete(
        graph,
        connector_ctx,
        &dependent_model,
        &dependent_records_node,
        &delete_dependents_node,
    )?;

    graph.create_edge(
        &dependent_records_node,
        &delete_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |mut delete_dependents_node, dependent_ids| {
                if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut dmr))) = delete_dependents_node {
                    dmr.record_filter = dependent_ids.into();
                }

                Ok(delete_dependents_node)
            }),
        ),
    )?;

    graph.create_edge(
        &delete_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    Ok(())
}

/// Inserts set null emulations into the graph between `parent_node` and `child_node`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the deletion emulation to ensure that subsequent deletions are handled correctly as well.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
///    │  (ids to del/upd)   ─ ┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │            │
///               ▼
///    ┌────────────────────┐  │
///    │Find Connected Model│
/// ┌──│     (SetNull)      │  │
/// │  └────────────────────┘
/// │             │            │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │  ┌────────────────────┐ │
/// ││ │  Insert onUpdate   │  │
/// │  │ emulation subtree  │ │
/// ││ │for relations using │  │
/// │  │the foreign key that│ │
/// ││ │    was updated.    │  │
/// │  └────────────────────┘ │
/// │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │             │
/// │             ▼            │
/// │  ┌────────────────────┐
/// │  │Update children (set│  │
/// └─▶│      FK null)      │
///    └────────────────────┘  │
///               │
///               ▼            │
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///       Delete / Update   │◀ ┘
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
pub fn emulate_on_delete_set_null(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    relation_field: &RelationFieldRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().primary_identifier().clone();
    let child_fks = if relation_field.is_inlined_on_enclosing_model() {
        relation_field.scalar_fields()
    } else {
        relation_field.related_field().referenced_fields()
    };

    let child_update_args: IndexMap<_, _> = child_fks
        .into_iter()
        // Only the nullable fks should be updated to null
        .filter(|sf| !sf.is_required())
        .map(|child_fk| {
            (
                DatasourceFieldName::from(&child_fk),
                WriteOperation::scalar_set(PrismaValue::Null),
            )
        })
        .collect();

    if child_update_args.is_empty() {
        return Ok(());
    }

    // Records that need to be updated for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, parent_node, &parent_relation_field, Filter::empty())?;

    let set_null_query = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: child_update_args.into(),
    });

    let set_null_dependents_node = graph.create_node(Query::Write(set_null_query));

    graph.create_edge(
        &dependent_records_node,
        &set_null_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |mut set_null_dependents_node, dependent_ids| {
                if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut dmr))) = set_null_dependents_node
                {
                    dmr.record_filter = dependent_ids.into();
                }

                Ok(set_null_dependents_node)
            }),
        ),
    )?;

    graph.create_edge(
        &set_null_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    insert_emulated_on_delete(
        graph,
        connector_ctx,
        &dependent_model,
        &dependent_records_node,
        &set_null_dependents_node,
    )?;

    Ok(())
}

/// Inserts set null emulations into the graph between `parent_node` and `child_node`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the update emulation to ensure that subsequent updates are handled correctly as well.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
///    │  (ids to del/upd)   ─ ┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │            │
///               ▼
///    ┌────────────────────┐  │
///    │Find Connected Model│
/// ┌──│     (SetNull)      │  │
/// │  └────────────────────┘
/// │             │            │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │  ┌────────────────────┐  │
/// ││ │  Insert onUpdate   │  │
/// │  │ emulation subtree  │  │
/// ││ │for relations using │  │
/// │  │the foreign key that│  │
/// ││ │    was updated.    │  │
/// │  └────────────────────┘  │
/// │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │             │
/// │             ▼            │
/// │  ┌────────────────────┐
/// │  │Update children (set│  │
/// └─▶│      FK null)      │
///    └────────────────────┘  │
///               │
///               ▼            │
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///           Update        │◀ ┘
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
pub fn emulate_on_update_set_null(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef,
    connector_ctx: &ConnectorContext,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().primary_identifier().clone();

    // Only the nullable fks should be updated to null
    let (parent_pks, child_fks) = if relation_field.is_inlined_on_enclosing_model() {
        (relation_field.referenced_fields(), relation_field.scalar_fields())
    } else {
        (
            relation_field.related_field().scalar_fields(),
            relation_field.related_field().referenced_fields(),
        )
    };

    // Unwraps are safe as in this stage, no node content can be replaced.
    let parent_update_args = extract_update_args(graph.node_content(child_node).unwrap());
    let parent_updates_pk = parent_pks
        .into_iter()
        .any(|parent_pk| parent_update_args.get_field_value(parent_pk.db_name()).is_some());

    if !parent_updates_pk {
        return Ok(());
    }

    let child_update_args: IndexMap<_, _> = child_fks
        .iter()
        .filter(|child_fk| !child_fk.is_required())
        .map(|child_fk| {
            (
                DatasourceFieldName::from(child_fk),
                WriteOperation::scalar_set(PrismaValue::Null),
            )
        })
        .collect();

    // Records that need to be updated for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, parent_node, &parent_relation_field, Filter::empty())?;

    let set_null_query = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: child_update_args.into(),
    });

    let set_null_dependents_node = graph.create_node(Query::Write(set_null_query));

    graph.create_edge(
        &dependent_records_node,
        &set_null_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |mut set_null_dependents_node, dependent_ids| {
                if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut dmr))) = set_null_dependents_node
                {
                    dmr.record_filter = dependent_ids.into();
                }

                Ok(set_null_dependents_node)
            }),
        ),
    )?;

    graph.create_edge(
        &set_null_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    // Collect other relation fields that share at least one common foreign key with the relation key we're dealing with
    let dependent_relation_fields: Vec<_> = dependent_model
        .fields()
        .relation()
        .into_iter()
        .filter(|rf| rf != relation_field)
        .filter(|rf| {
            let fks = if rf.is_inlined_on_enclosing_model() {
                rf.scalar_fields()
            } else {
                rf.related_field().referenced_fields()
            };

            fks.iter().any(|fk| child_fks.contains(fk))
        })
        .map(|rf| match rf.is_inlined_on_enclosing_model() {
            true => rf,
            false => rf.related_field(),
        })
        .collect();

    // If there are any relation fields sharing one common foreign key, recurse
    for rf in dependent_relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction => continue,
            ReferentialAction::Restrict => {
                emulate_restrict(graph, &rf, &dependent_records_node, &set_null_dependents_node)?
            }
            ReferentialAction::SetNull => emulate_on_update_set_null(
                graph,
                &rf,
                connector_ctx,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            ReferentialAction::Cascade => emulate_on_update_cascade(
                graph,
                &rf,
                connector_ctx,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            x => panic!("Unsupported referential action emulation: {}", x),
        }
    }

    Ok(())
}

/// Inserts emulated referential actions for `onUpdate` into the graph.
/// All relations that refer to the `model` row(s) being deleted are checked for their desired emulation and inserted accordingly.
/// Right now, supported modes are `Restrict` and `SetNull` (cascade will follow).
/// Those checks fail at runtime and are inserted between `parent_node` and `child_node`.
///
/// This function is usually part of a delete (`deleteOne` or `deleteMany`).
/// Expects `parent_node` to return one or more IDs (for records of `model`) to be checked.
///
/// Resulting graph (all emulations):
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
///    │  (ids to update)
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │
///               ▼
///    ┌────────────────────┐
/// ┌ ─│     Join node      │─────────────────┬─────────────────────────────┬────────────────────────────────────────┐
///    └────────────────────┘                 │                             │                                        │
/// │             │                           │                             │                                        │
///               │                           │                             │                                        │
/// │             ▼                           ▼                             ▼                                        ▼
///    ┌────────────────────┐      ┌────────────────────┐        ┌────────────────────┐                   ┌────────────────────┐
/// │  │Find Connected Model│      │Find Connected Model│        │Find Connected Model│                   │Find Connected Model│
///    │    A (Restrict)    │      │    B (Restrict)    │     ┌──│    C (SetNull)     │                ┌──│    D (Cascade)     │
/// │  └────────────────────┘      └────────────────────┘     │  └────────────────────┘                │  └────────────────────┘
///               │                           │               │             │                          │             │
/// │      Fail if│> 0                 Fail if│> 0            │             ▼                          │             │
///               │                           │               │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │             ▼
/// │             ▼                           ▼               │  ┌────────────────────┐ │              │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///    ┌────────────────────┐      ┌────────────────────┐     ││ │  Insert onUpdate   │                │  ┌────────────────────┐ │
/// │  │       Empty        │      │       Empty        │     │  │ emulation subtree  │ │              ││ │  Insert onUpdate   │
///    └────────────────────┘      └────────────────────┘     ││ │for relations using │                │  │ emulation subtree  │ │
/// │             │                           │               │  │the foreign key that│ │              ││ │ for all relations  │
///               │                           │               ││ │    was updated.    │                │  │   pointing to D.   │ │
/// │             │                           │               │  └────────────────────┘ │              ││ └────────────────────┘
///               │                           │               │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
/// │             │                           │               │             │                          │             │
///               │                           │               │             │                          │             │
/// │             ▼                           │               │             ▼                          │             ▼
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                  │               │  ┌────────────────────┐                │  ┌────────────────────┐
/// └ ▶        Update       │◀────────────────┘               │  │ Update Cs (set FK  │                └─▶│     Update Cs      │
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─                                  └─▶│       null)        │                   └────────────────────┘
///               ▲                                              └────────────────────┘                              │
///               │                                                         │                                        │
///               └─────────────────────────────────────────────────────────┴────────────────────────────────────────┘
/// ```
#[tracing::instrument(skip(graph, model_to_update, parent_node, child_node))]
pub fn insert_emulated_on_update_with_intermediary_node(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model_to_update: &ModelRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<Option<NodeRef>> {
    // If the connector uses the `ReferentialIntegrity::ForeignKeys` mode, we do not do any checks / emulation.
    if connector_ctx.referential_integrity.uses_foreign_keys() {
        return Ok(None);
    }

    // If the connector uses the `ReferentialIntegrity::Prisma` mode, then the emulation will kick in.
    let internal_model = model_to_update.internal_data_model();
    let relation_fields = internal_model.fields_pointing_to_model(model_to_update, false);

    let join_node = graph.create_node(Flow::Return(None));

    graph.create_edge(
        &parent_node,
        &join_node,
        QueryGraphDependency::ProjectedDataDependency(
            model_to_update.primary_identifier(),
            Box::new(move |return_node, parent_ids| {
                if let Node::Flow(Flow::Return(_)) = return_node {
                    Ok(Node::Flow(Flow::Return(Some(parent_ids))))
                } else {
                    Ok(return_node)
                }
            }),
        ),
    )?;

    for rf in relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction => continue, // Explicitly do nothing.
            ReferentialAction::Restrict => emulate_restrict(graph, &rf, &join_node, child_node)?,
            ReferentialAction::SetNull => {
                emulate_on_update_set_null(graph, &rf, connector_ctx, &join_node, child_node)?
            }
            ReferentialAction::Cascade => emulate_on_update_cascade(graph, &rf, connector_ctx, &join_node, child_node)?,
            x => panic!("Unsupported referential action emulation: {}", x),
        }
    }

    Ok(Some(join_node))
}

#[tracing::instrument(skip(graph, model_to_update, parent_node, child_node))]
pub fn insert_emulated_on_update(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    model_to_update: &ModelRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    // If the connector uses the `ReferentialIntegrity::ForeignKeys` mode, we do not do any checks / emulation.
    if connector_ctx.referential_integrity.uses_foreign_keys() {
        return Ok(());
    }

    // If the connector uses the `ReferentialIntegrity::Prisma` mode, then the emulation will kick in.
    let internal_model = model_to_update.internal_data_model();
    let relation_fields = internal_model.fields_pointing_to_model(model_to_update, false);

    for rf in relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction => continue, // Explicitly do nothing.
            ReferentialAction::Restrict => emulate_restrict(graph, &rf, &parent_node, child_node)?,
            ReferentialAction::SetNull => {
                emulate_on_update_set_null(graph, &rf, connector_ctx, &parent_node, child_node)?
            }
            ReferentialAction::Cascade => {
                emulate_on_update_cascade(graph, &rf, connector_ctx, &parent_node, child_node)?
            }
            x => panic!("Unsupported referential action emulation: {}", x),
        }
    }

    Ok(())
}

fn extract_update_args(parent_node: &Node) -> &WriteArgs {
    if let Node::Query(Query::Write(q)) = parent_node {
        match q {
            WriteQuery::UpdateRecord(one) => &one.args,
            WriteQuery::UpdateManyRecords(many) => &many.args,
            _ => panic!("Parent operation for update emulation is not an update."),
        }
    } else {
        panic!("Parent operation for update emulation is not a query.")
    }
}

/// Inserts cascade emulations into the graph between `parent_node` and `child_node`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the update emulation to ensure that subsequent updates are handled correctly as well.
///
/// ```text
//    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Parent       │
///    │  (ids to update)    ─ ┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │            │
///               ▼
///    ┌────────────────────┐  │
///    │Find Connected Model│
/// ┌──│     (Cascade)      │  │
/// │  └────────────────────┘
/// │             │            │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
/// │  ┌────────────────────┐ │
/// ││ │  Insert onUpdate   │  │
/// │  │ emulation subtree  │ │
/// ││ │ for all relations  │  │
/// │  │  pointing to the   │ │
/// ││ │  Connected Model.  │  │
/// │  └────────────────────┘ │
/// │└ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ │
/// │             │
/// │             │            │
/// │             ▼
/// │  ┌────────────────────┐  │
/// └─▶│  Update children   │
///    └────────────────────┘  │
///               │
///               ▼            │
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///            Update       │◀ ┘
///    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// ```
pub fn emulate_on_update_cascade(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef, // This is the field _on the other model_ for cascade.
    connector_ctx: &ConnectorContext,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().primary_identifier();
    let (parent_pks, child_fks) = if relation_field.is_inlined_on_enclosing_model() {
        (relation_field.referenced_fields(), relation_field.scalar_fields())
    } else {
        (
            relation_field.related_field().scalar_fields(),
            relation_field.related_field().referenced_fields(),
        )
    };

    // Records that need to be updated for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, parent_node, &parent_relation_field, Filter::empty())?;

    // Unwraps are safe as in this stage, no node content can be replaced.
    let parent_update_args = extract_update_args(graph.node_content(child_node).unwrap());
    let child_update_args: Vec<_> = parent_pks
        .into_iter()
        .zip(child_fks)
        .filter_map(|(parent_pk, child_fk)| {
            parent_update_args
                .get_field_value(parent_pk.db_name())
                .map(|value| (DatasourceFieldName::from(&child_fk), value.clone()))
        })
        .collect();

    let update_query = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: child_update_args.into(),
    });

    let update_dependents_node = graph.create_node(Query::Write(update_query));

    insert_emulated_on_update(
        graph,
        connector_ctx,
        &dependent_model,
        &dependent_records_node,
        &update_dependents_node,
    )?;

    graph.create_edge(
        &dependent_records_node,
        &update_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            Box::new(move |mut update_dependents_node, dependent_ids| {
                if let Node::Query(Query::Write(WriteQuery::UpdateManyRecords(ref mut dmr))) = update_dependents_node {
                    dmr.record_filter = dependent_ids.into();
                }

                Ok(update_dependents_node)
            }),
        ),
    )?;

    graph.create_edge(
        &update_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    Ok(())
}
