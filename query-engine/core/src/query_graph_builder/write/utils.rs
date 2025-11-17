use crate::{
    Computation, DataExpectation, DataOperation, MissingRelatedRecord, ParsedInputValue, QueryGraphBuilderResult,
    RelationViolation, RowSink,
    inputs::{
        DeleteManyRecordsSelectorsInput, IfInput, LeftSideDiffInput, RelatedRecordsSelectorsInput, ReturnInput,
        RightSideDiffInput, UpdateManyRecordsSelectorsInput,
    },
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
};
use indexmap::IndexMap;
use psl::parser_database::ReferentialAction;
use query_structure::{
    DatasourceFieldName, FieldSelection, Filter, Model, PrismaValue, RecordFilter, RelationFieldRef, SelectionResult,
    WriteArgs, WriteOperation,
};
use schema::QuerySchema;

/// Coerces single values (`ParsedInputValue::Single` and `ParsedInputValue::Map`) into a vector.
/// Simply unpacks `ParsedInputValue::List`.
pub(crate) fn coerce_vec(val: ParsedInputValue<'_>) -> Vec<ParsedInputValue<'_>> {
    match val {
        ParsedInputValue::List(l) => l,
        m @ ParsedInputValue::Map(_) => vec![m],
        single => vec![single],
    }
}

pub(crate) fn node_is_create(graph: &QueryGraph, node: &NodeRef) -> bool {
    matches!(
        graph.node_content(node).unwrap(),
        Node::Query(Query::Write(WriteQuery::CreateRecord(_)))
    )
}

/// Produces a non-failing read query that fetches the requested selection of records for a given filterable.
pub(crate) fn read_ids_infallible<T>(model: Model, selection: FieldSelection, filter: T) -> Query
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
        options: QueryOptions::none(),
        relation_load_strategy: query_structure::RelationLoadStrategy::Query,
    });

    Query::Read(read_query)
}

/// Produces a non-failing read query that fetches the requested selection of a record for a given filterable.
pub(crate) fn read_id_infallible<T>(model: Model, selection: FieldSelection, filter: T) -> Query
where
    T: Into<Filter>,
{
    let selected_fields = get_selected_fields(&model, selection);
    let filter: Filter = filter.into();

    let read_query = ReadQuery::RecordQuery(RecordQuery {
        name: "read_ids_infallible".into(), // this name only eases debugging
        alias: None,
        model: model.clone(),
        filter: Some(filter),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
        options: QueryOptions::none(),
        relation_load_strategy: query_structure::RelationLoadStrategy::Query,
    });

    Query::Read(read_query)
}

fn get_selected_fields(model: &Model, selection: FieldSelection) -> FieldSelection {
    // Always fetch the primary identifier as well.
    let primary_model_id = model.shard_aware_primary_identifier();

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
pub(crate) fn insert_find_children_by_parent_node<T>(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: T,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<Filter>,
{
    let parent_model_id = parent_relation_field.model().shard_aware_primary_identifier();
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
        parent_field: parent_relation_field.clone(),
        parent_results: None,
        args: (child_model, filter).into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent_node,
        &read_children_node,
        QueryGraphDependency::ProjectedDataDependency(selection, RowSink::All(&RelatedRecordsSelectorsInput), None),
    )?;

    Ok(read_children_node)
}

/// Adds a node to read the old child, compare it to the new child and continues the graph execution only if there are diffences between the old & the new child.
/// This function is tailored for 1-1 nested connect.
pub fn insert_1to1_idempotent_connect_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    read_new_child_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let child_model = parent_relation_field.related_model();
    let child_model_identifier = child_model.shard_aware_primary_identifier();

    let diff_node = graph.create_node(Node::Computation(Computation::empty_diff_left_to_right(
        child_model_identifier.clone(),
    )));

    graph.create_edge(
        read_new_child_node,
        &diff_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            RowSink::All(&LeftSideDiffInput),
            Some(DataExpectation::non_empty_rows(
                MissingRelatedRecord::builder()
                    .model(&child_model.clone())
                    .relation(&parent_relation_field.relation())
                    .operation(DataOperation::NestedConnect)
                    .build(),
            )),
        ),
    )?;
    let read_old_child_node =
        insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    graph.create_edge(
        &read_old_child_node,
        &diff_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            RowSink::All(&RightSideDiffInput),
            None,
        ),
    )?;
    let if_node = graph.create_node(Flow::if_non_empty());

    graph.create_edge(
        &diff_node,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(child_model_identifier, RowSink::All(&IfInput), None),
    )?;
    let empty_node = graph.create_node(Node::Empty);

    graph.create_edge(&if_node, &empty_node, QueryGraphDependency::Then)?;

    Ok(empty_node)
}

/// Creates an update many records query node and adds it to the query graph.
/// Used to have a skeleton update node in the graph that can be further transformed during query execution based
/// on available information.
///
/// No edges are created.
pub fn update_records_node_placeholder<T>(graph: &mut QueryGraph, filter: T, model: Model) -> NodeRef
where
    T: Into<Filter>,
{
    update_records_node_placeholder_with_args(
        graph,
        filter,
        model,
        WriteArgs::new_empty(crate::executor::get_request_now()),
    )
}

pub fn update_records_node_placeholder_with_args<T>(
    graph: &mut QueryGraph,
    filter: T,
    model: Model,
    args: WriteArgs,
) -> NodeRef
where
    T: Into<Filter>,
{
    let filter = filter.into();
    let record_filter = filter.into();

    let ur = UpdateManyRecords {
        name: String::new(),
        model,
        record_filter,
        args,
        selected_fields: None,
        limit: None,
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
pub fn insert_existing_1to1_related_model_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();
    let child_linking_fields = parent_relation_field.related_field().linking_fields();

    let child_model = parent_relation_field.related_model();
    let child_side_required = parent_relation_field.related_field().is_required();
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let rf = parent_relation_field.clone();

    // Note: Also creates the edge between `parent` and the new node.
    let read_existing_children =
        insert_find_children_by_parent_node(graph, parent_node, parent_relation_field, Filter::empty())?;

    let write_args = WriteArgs::from_result(
        SelectionResult::from(&child_linking_fields),
        crate::executor::get_request_now(),
    );

    let update_existing_child =
        update_records_node_placeholder_with_args(graph, Filter::empty(), child_model, write_args);

    let if_node = graph.create_node(if relation_inlined_parent {
        Flow::if_false()
    } else {
        Flow::if_non_empty()
    });

    graph.create_edge(
        &read_existing_children,
        &if_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier.clone(),
            RowSink::All(&IfInput),
            // If the other side ("child") requires the connection, we need to make sure that there isn't a child already connected
            // to the parent, as that would violate the other childs relation side.
            if child_side_required {
                Some(DataExpectation::empty_rows(RelationViolation::from(rf)))
            } else {
                None
            },
        ),
    )?;

    graph.create_edge(&if_node, &update_existing_child, QueryGraphDependency::Then)?;
    graph.create_edge(
        &read_existing_children,
        &update_existing_child,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::ExactlyOne(&UpdateManyRecordsSelectorsInput),
            Some(DataExpectation::non_empty_rows(
                MissingRelatedRecord::builder()
                    .model(&parent_relation_field.model())
                    .relation(&parent_relation_field.relation())
                    .operation(DataOperation::NestedConnect)
                    .build(),
            )),
        ),
    )?;

    Ok(read_existing_children)
}

/// Inserts emulated referential actions for `onDelete` into the graph.
/// All relations that refer to the `model` row(s) being deleted are checked for their desired emulation and inserted accordingly.
/// Those checks fail at runtime and are inserted as children to `node_providing_ids` node.
///
/// This function is usually part of a delete (`deleteOne` or `deleteMany`).
/// Expects `node_providing_ids` to return one or more IDs (for records of `model`) to be checked.
///
/// Returns a list of leaf nodes, each corresponding to a section of the tree related to the individual check.
///
/// Resulting graph (all emulations):
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///    |   Node providing   │
///    │   ids to delete     ─────────────────┬─────────────────────────────┬────────────────────────────────────────┐
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘                 │                             │                                        │
///               │                           │                             │                                        │
///               ▼                           ▼                             ▼                                        ▼
///    ┌────────────────────┐      ┌────────────────────┐        ┌────────────────────┐                   ┌────────────────────┐
///    │Find Connected Model│      │Find Connected Model│        │Find Connected Model│                   │Find Connected Model│
///    │    A (Restrict)    │      │    B (Restrict)    │     ┌──│    C (SetNull)     │                ┌──│    D (Cascade)     │
///    └────────────────────┘      └────────────────────┘     │  └────────────────────┘                │  └────────────────────┘
///               │                           │               │             │                          │             │
///        Fail if│> 0                 Fail if│> 0            │             ▼                          │             │
///               │                           │               │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │             ▼
///               ▼                           ▼               │  ┌────────────────────┐ │              │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///    ┌────────────────────┐      ┌────────────────────┐     ││ │  Insert onUpdate   │                │  ┌────────────────────┐ │
///    │       Empty        │      │       Empty        │     │  │ emulation subtree  │ │              ││ │  Insert onDelete   │
///    └────────────────────┘      └────────────────────┘     ││ │for relations using │                │  │ emulation subtree  │ │
///                                                           │  │the foreign key that│ │              ││ │ for all relations  │
///                                                           ││ │    was updated.    │                │  │   pointing to D.   │ │
///                                                           │  └────────────────────┘ │              ││ └────────────────────┘
///                                                           │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─               │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///                                                           │             │                          │             │
///                                                           │             │                          │             │
///                                                           │             ▼                          │             ▼
///                                                           │  ┌────────────────────┐                │  ┌────────────────────┐
///                                                           │  │ Update Cs (set FK  │                └─▶│     Delete Cs      │
///                                                           └─▶│       null)        │                   └────────────────────┘
///                                                              └────────────────────┘
/// ```
pub(crate) fn insert_emulated_on_delete(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model_to_delete: &Model,
    node_providing_ids: &NodeRef,
) -> QueryGraphBuilderResult<Vec<NodeRef>> {
    // If the connector uses the `RelationMode::ForeignKeys` or `RelationMode::PrismaSkipIntegrity` mode, we do not do any checks / emulation.
    if query_schema
        .relation_mode()
        .should_skip_emulated_referential_integrity()
    {
        return Ok(vec![]);
    }

    // If the connector uses the `RelationMode::Prisma` mode, then the emulation will kick in.
    let internal_model = &model_to_delete.dm;
    let relation_fields = internal_model.fields_pointing_to_model(model_to_delete);
    let mut leaf_nodes = vec![];
    for rf in relation_fields {
        match rf.relation().on_delete() {
            ReferentialAction::NoAction | ReferentialAction::Restrict => {
                let node = emulate_on_delete_restrict(graph, &rf, node_providing_ids)?;
                leaf_nodes.push(node);
            }
            ReferentialAction::SetNull => {
                let node = emulate_on_delete_set_null(graph, query_schema, &rf, node_providing_ids)?;
                if let Some(node) = node {
                    leaf_nodes.push(node);
                }
            }
            ReferentialAction::Cascade => {
                let node = emulate_on_delete_cascade(graph, &rf, query_schema, node_providing_ids)?;
                leaf_nodes.push(node);
            }
            x => panic!("Unsupported referential action emulation: {x}"),
        }
    }

    Ok(leaf_nodes)
}

/// Creates restrict emulations as child nodes to `node_providing_ids`.
/// `relation_field` is the relation field pointing to the model to be deleted/updated.
/// Returns leaf node in the created subtree.
///
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///    |   Node providing   │
///    │   ids to delete    |
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │
///               ▼
///    ┌────────────────────┐
///    │Find Connected Model│
///    │     (Restrict)     │
///    └────────────────────┘
///               │
///        Fail if│> 0
///               │
///               ▼
///    ┌────────────────────┐
///    │       Empty        │
///    └────────────────────┘
/// ```
pub fn emulate_on_delete_restrict(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef,
    node_providing_ids: &NodeRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let noop_node = graph.create_node(Node::Empty);
    let relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().shard_aware_primary_identifier();
    let read_node = insert_find_children_by_parent_node(graph, node_providing_ids, &relation_field, Filter::empty())?;

    graph.create_edge(
        &read_node,
        &noop_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::Discard,
            Some(DataExpectation::empty_rows(RelationViolation::from(relation_field))),
        ),
    )?;

    Ok(noop_node)
}

/// Creates cascade emulations as child nodes to `node_providing_ids`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the deletion emulation to ensure that subsequent deletions are handled correctly as well.
/// Returns leaf node in the created subtree.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
///    |   Node providing   │
///    │   ids to delete    |
///     ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
///               │
///               ▼
///    ┌────────────────────┐
///    │Find Connected Model│
/// ┌──│     (Cascade)      │
/// │  └────────────────────┘
/// │             │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │  ┌────────────────────┐ │
/// ││ │  Insert onDelete   │
/// │  │ emulation subtree  │ │
/// ││ │ for all relations  │
/// │  │  pointing to the   │ │
/// ││ │  Connected Model.  │
/// │  └────────────────────┘ │
/// │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │             │
/// │             ▼
/// │  ┌────────────────────┐
/// └─▶│  Delete children   │
///    └────────────────────┘
/// ```
pub fn emulate_on_delete_cascade(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef, // This is the field _on the other model_ for cascade.
    query_schema: &QuerySchema,
    node_providing_ids: &NodeRef,
) -> QueryGraphBuilderResult<NodeRef> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();

    // Records that need to be deleted for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, node_providing_ids, &parent_relation_field, Filter::empty())?;

    let delete_query = WriteQuery::DeleteManyRecords(DeleteManyRecords {
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        limit: None,
    });

    let delete_dependents_node = graph.create_node(Query::Write(delete_query));

    let dependencies = insert_emulated_on_delete(graph, query_schema, &dependent_model, &dependent_records_node)?;
    create_execution_order_edges(graph, dependencies, delete_dependents_node)?;

    graph.create_edge(
        &dependent_records_node,
        &delete_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::All(&DeleteManyRecordsSelectorsInput),
            None,
        ),
    )?;

    Ok(delete_dependents_node)
}

/// Creates set null emulations as child nodes to `node_providing_ids`.
/// `relation_field` is the relation field pointing to the model to be deleted.
/// Recurses into the deletion emulation to ensure that subsequent deletions are handled correctly as well.
/// Returns leaf node in the created subtree. If no subtree was created, returns `None`.
///
/// ```text
///    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
///    |  Node providing   │
///    │   ids to delete   |
///     ─ ─ ─ ─ ─ ─ ─ ─ ─  ┘
///               │
///               ▼
///    ┌────────────────────┐
///    │Find Connected Model│
/// ┌──│     (SetNull)      │
/// │  └────────────────────┘
/// │             │
/// │             ▼
/// │┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │  ┌────────────────────┐ │
/// ││ │  Insert onUpdate   │
/// │  │ emulation subtree  │ │
/// ││ │for relations using │
/// │  │the foreign key that│ │
/// ││ │    was updated.    │
/// │  └────────────────────┘ │
/// │└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
/// │             │
/// │             ▼
/// │  ┌────────────────────┐
/// │  │Update children (set│
/// └─▶│      FK null)      │
///    └────────────────────┘
/// ```
pub fn emulate_on_delete_set_null(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    relation_field: &RelationFieldRef,
    node_providing_ids: &NodeRef,
) -> QueryGraphBuilderResult<Option<NodeRef>> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();
    let child_fks = relation_field.left_scalars();

    let child_update_args: IndexMap<_, _> = child_fks
        .iter()
        // Only the nullable fks should be updated to null
        .filter(|sf| !sf.is_required())
        .map(|child_fk| {
            (
                DatasourceFieldName::from(child_fk),
                WriteOperation::scalar_set(PrismaValue::Null),
            )
        })
        .collect();

    if child_update_args.is_empty() {
        return Ok(None);
    }

    // Records that need to be updated for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, node_providing_ids, &parent_relation_field, Filter::empty())?;

    let set_null_query = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        name: String::new(),
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: WriteArgs::new(child_update_args, crate::executor::get_request_now()),
        selected_fields: None,
        limit: None,
    });

    let set_null_dependents_node = graph.create_node(Query::Write(set_null_query));

    graph.create_edge(
        &dependent_records_node,
        &set_null_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::All(&UpdateManyRecordsSelectorsInput),
            None,
        ),
    )?;

    // Collect other relation fields that share at least one common foreign key with the relation field we're dealing with
    let overlapping_relation_fields = collect_overlapping_relation_fields(dependent_model, relation_field);

    // For every relation fields sharing one common foreign key on the updated model, apply onUpdate emulations.
    for rf in overlapping_relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction | ReferentialAction::Restrict => {
                emulate_on_update_restrict(graph, &rf, &dependent_records_node, &set_null_dependents_node)?
            }
            ReferentialAction::SetNull => emulate_on_update_set_null(
                graph,
                &rf,
                query_schema,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            ReferentialAction::Cascade => emulate_on_update_cascade(
                graph,
                &rf,
                query_schema,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            x => panic!("Unsupported referential action emulation: {x}"),
        }
    }

    Ok(Some(set_null_dependents_node))
}

/// Creates a `QueryGraphDependency::ExecutionOrder` edge between each node in the `from` list and `to` node.
pub fn create_execution_order_edges(
    graph: &mut QueryGraph,
    from: Vec<NodeRef>,
    to: NodeRef,
) -> QueryGraphBuilderResult<()> {
    for node in from {
        graph.create_edge(&node, &to, QueryGraphDependency::ExecutionOrder)?;
    }
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
    query_schema: &QuerySchema,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();

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
        name: String::new(),
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: WriteArgs::new(child_update_args, crate::executor::get_request_now()),
        selected_fields: None,
        limit: None,
    });

    let set_null_dependents_node = graph.create_node(Query::Write(set_null_query));

    graph.create_edge(
        &dependent_records_node,
        &set_null_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::All(&UpdateManyRecordsSelectorsInput),
            None,
        ),
    )?;

    graph.create_edge(
        &set_null_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    // Collect other relation fields that share at least one common foreign key with the relation field we're dealing with
    let overlapping_relation_fields = collect_overlapping_relation_fields(dependent_model, relation_field);

    // For every relation fields sharing one common foreign key, recurse
    for rf in overlapping_relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction | ReferentialAction::Restrict => {
                emulate_on_update_restrict(graph, &rf, &dependent_records_node, &set_null_dependents_node)?
            }
            ReferentialAction::SetNull => emulate_on_update_set_null(
                graph,
                &rf,
                query_schema,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            ReferentialAction::Cascade => emulate_on_update_cascade(
                graph,
                &rf,
                query_schema,
                &dependent_records_node,
                &set_null_dependents_node,
            )?,
            x => panic!("Unsupported referential action emulation: {x}"),
        }
    }

    Ok(())
}

pub fn emulate_on_update_restrict(
    graph: &mut QueryGraph,
    relation_field: &RelationFieldRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let noop_node = graph.create_node(Node::Empty);
    let relation_field = relation_field.related_field();
    let child_model_identifier = relation_field.related_model().shard_aware_primary_identifier();
    let read_node = insert_find_children_by_parent_node(graph, parent_node, &relation_field, Filter::empty())?;

    let linking_fields = relation_field.linking_fields();

    // Unwraps are safe as in this stage, no node content can be replaced.
    let parent_update_args = extract_update_args(graph.node_content(child_node).unwrap());

    let linking_fields_updated = linking_fields
        .into_iter()
        .any(|parent_pk| parent_update_args.get_field_value(&parent_pk.db_name()).is_some());

    graph.create_edge(
        &read_node,
        &noop_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::Discard,
            // If any linking fields are to be updated and there are already connected children, then fail
            if linking_fields_updated {
                Some(DataExpectation::empty_rows(RelationViolation::from(relation_field)))
            } else {
                None
            },
        ),
    )?;

    // Edge from empty node to the child (delete).
    graph.create_edge(&noop_node, child_node, QueryGraphDependency::ExecutionOrder)?;

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
pub fn insert_emulated_on_update_with_intermediary_node(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model_to_update: &Model,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<Option<NodeRef>> {
    // If the connector uses the `RelationMode::ForeignKeys` mode or the `RelationMode::PrismaSkipIntegrity`, we do not do any checks / emulation.
    if query_schema
        .relation_mode()
        .should_skip_emulated_referential_integrity()
    {
        return Ok(None);
    }

    // If the connector uses the `RelationMode::Prisma` mode, then the emulation will kick in.
    let internal_model = &model_to_update.dm;
    let relation_fields = internal_model.fields_pointing_to_model(model_to_update);

    let join_node = graph.create_node(Flow::Return(Vec::new()));

    graph.create_edge(
        parent_node,
        &join_node,
        QueryGraphDependency::ProjectedDataDependency(
            model_to_update.shard_aware_primary_identifier(),
            RowSink::All(&ReturnInput),
            None,
        ),
    )?;

    for rf in relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction | ReferentialAction::Restrict => {
                emulate_on_update_restrict(graph, &rf, &join_node, child_node)?
            }
            ReferentialAction::SetNull => emulate_on_update_set_null(graph, &rf, query_schema, &join_node, child_node)?,
            ReferentialAction::Cascade => emulate_on_update_cascade(graph, &rf, query_schema, &join_node, child_node)?,
            x => panic!("Unsupported referential action emulation: {x}"),
        }
    }

    Ok(Some(join_node))
}

pub fn insert_emulated_on_update(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    model_to_update: &Model,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    // If the connector uses the `RelationMode::ForeignKeys` mode or the `RelationMode::PrismaSkipIntegrity` mode, we do not do any checks / emulation.
    if query_schema
        .relation_mode()
        .should_skip_emulated_referential_integrity()
    {
        return Ok(());
    }

    // If the connector uses the `RelationMode::Prisma` mode, then the emulation will kick in.
    let internal_model = &model_to_update.dm;
    let relation_fields = internal_model.fields_pointing_to_model(model_to_update);

    for rf in relation_fields {
        match rf.relation().on_update() {
            ReferentialAction::NoAction | ReferentialAction::Restrict => {
                emulate_on_update_restrict(graph, &rf, parent_node, child_node)?
            }
            ReferentialAction::SetNull => {
                emulate_on_update_set_null(graph, &rf, query_schema, parent_node, child_node)?
            }
            ReferentialAction::Cascade => emulate_on_update_cascade(graph, &rf, query_schema, parent_node, child_node)?,
            x => panic!("Unsupported referential action emulation: {x}"),
        }
    }

    Ok(())
}

fn extract_update_args(parent_node: &Node) -> &WriteArgs {
    if let Node::Query(Query::Write(q)) = parent_node {
        match q {
            WriteQuery::UpdateRecord(one) => one.args(),
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
    query_schema: &QuerySchema,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let dependent_model = relation_field.model();
    let parent_relation_field = relation_field.related_field();
    let child_model_identifier = parent_relation_field.related_model().shard_aware_primary_identifier();
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

    // Computes update arguments for child based on parent update arguments
    let child_update_args: Vec<_> = parent_pks
        .into_iter()
        .zip(child_fks)
        .filter_map(|(parent_pk, child_fk)| {
            parent_update_args
                .get_field_value(parent_pk.db_name())
                .map(|value| (DatasourceFieldName::from(&child_fk), value.clone()))
        })
        .collect();

    // If nothing was found to be updated for the child, stop here
    if child_update_args.is_empty() {
        return Ok(());
    }

    // Records that need to be updated for the cascade.
    let dependent_records_node =
        insert_find_children_by_parent_node(graph, parent_node, &parent_relation_field, Filter::empty())?;

    let update_query = WriteQuery::UpdateManyRecords(UpdateManyRecords {
        name: String::new(),
        model: dependent_model.clone(),
        record_filter: RecordFilter::empty(),
        args: WriteArgs::new(
            child_update_args.into_iter().collect(),
            crate::executor::get_request_now(),
        ),
        selected_fields: None,
        limit: None,
    });

    let update_dependents_node = graph.create_node(Query::Write(update_query));

    insert_emulated_on_update(
        graph,
        query_schema,
        &dependent_model,
        &dependent_records_node,
        &update_dependents_node,
    )?;

    graph.create_edge(
        &dependent_records_node,
        &update_dependents_node,
        QueryGraphDependency::ProjectedDataDependency(
            child_model_identifier,
            RowSink::All(&UpdateManyRecordsSelectorsInput),
            None,
        ),
    )?;

    graph.create_edge(
        &update_dependents_node,
        child_node,
        QueryGraphDependency::ExecutionOrder,
    )?;

    Ok(())
}

/// Collect relation fields that share at least one common foreign key with `relation_field`.
pub(crate) fn collect_overlapping_relation_fields(
    model: Model,
    relation_field: &RelationFieldRef,
) -> Vec<RelationFieldRef> {
    let child_fks = relation_field.left_scalars();

    let dependent_relation_fields: Vec<_> = model
        .fields()
        .relation()
        .filter(|rf| rf != relation_field)
        .filter(|rf| {
            let fks = rf.left_scalars();

            fks.iter().any(|fk| child_fks.contains(fk))
        })
        .map(|rf| match rf.is_inlined_on_enclosing_model() {
            true => rf,
            false => rf.related_field(),
        })
        .collect();

    dependent_relation_fields
}
