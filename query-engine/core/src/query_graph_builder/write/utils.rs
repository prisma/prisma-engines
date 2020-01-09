use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{Filter, QueryArguments, ScalarCompare};
use itertools::Itertools;
use prisma_models::{Field, ModelRef, PrismaArgs, PrismaValue, RecordIdentifier, RelationFieldRef, SelectedFields};
use std::{convert::TryInto, sync::Arc};

pub trait IdFilter {
    fn filter(self) -> Filter;
}

impl IdFilter for RecordIdentifier {
    fn filter(self) -> Filter {
        let filters: Vec<Filter> = self
            .pairs
            .into_iter()
            .map(|(field, value)| match field {
                Field::Scalar(sf) => sf.equals(value),
                Field::Relation(_) => panic!("Relation fields in IDs are not supported"),
            })
            .collect();

        Filter::and(filters)
    }
}

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
    match graph.node_content(node).unwrap() {
        Node::Query(Query::Write(WriteQuery::CreateRecord(_))) => true,
        _ => false,
    }
}

/// Produces a non-failing read query that fetches IDs for a given filterable.
pub fn read_ids_infallible<T>(model: &ModelRef, filter: T) -> Query
where
    T: Into<Filter>,
{
    let selected_fields: SelectedFields = model.identifier().into();
    let filter: Filter = filter.into();

    let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name: "read_ids_infallible".into(), // this name only eases debugging
        alias: None,
        model: Arc::clone(&model),
        args: filter.into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
    });

    Query::Read(read_query)
}

/// Adds a read query to the query graph that finds related records by parent ID.
/// Connects the parent node and the read node with an edge, which takes care of the
/// node transformation based on the parent ID.
///
/// Optionally, a filter can be passed that narrows down the child selection.
///
/// Returns a `NodeRef` to the newly created read node.
///
/// The resulting graph is (dashed already exists, everything else is added):
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
pub fn insert_find_children_by_parent_node<T>(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    filter: T,
) -> QueryGraphBuilderResult<NodeRef>
where
    T: Into<QueryArguments>,
{
    let selected_fields: SelectedFields = parent_relation_field.related_model().identifier().into();

    let read_parent_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "find_children_by_parent".to_owned(),
        alias: None,
        parent_field: Arc::clone(parent_relation_field),
        parent_ids: None,
        args: filter.into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent_node,
        &read_parent_node,
        QueryGraphDependency::ParentIds(Box::new(|mut node, parent_ids| {
            if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = node {
                // We know that all PrismaValues in `parent_ids` are transformable into GraphqlIds.
                rq.parent_ids = Some(parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect());
            };

            Ok(node)
        })),
    )?;

    Ok(read_parent_node)
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
    let mut args = PrismaArgs::new();

    args.update_datetimes(Arc::clone(&model));

    let ur = UpdateManyRecords {
        model,
        filter: filter.into(),
        args: args,
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
/// The elements added to the graph are all except `Parent Node`:
/// ```text
/// ┌────────────────────────┐
/// │      Parent Node       │
/// └────────────────────────┘
///              │
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
pub fn insert_existing_1to1_related_model_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let child_model_id_field = child_model.identifier();
    let child_side_required = parent_relation_field.related_field().is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let rf = Arc::clone(&parent_relation_field);

    let read_existing_children =
        insert_find_children_by_parent_node(graph, &parent_node, &parent_relation_field, Filter::empty())?;

    let update_existing_child = update_records_node_placeholder(graph, Filter::empty(), child_model);
    let relation_field_name = parent_relation_field.related_field().name.clone();
    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_existing_children,
        &if_node,
        QueryGraphDependency::ParentIds(Box::new(move |node, child_ids| {
            // If the other side ("child") requires the connection, we need to make sure that there isn't a child already connected
            // to the parent, as that would violate the other childs relation side.
            if child_ids.len() > 0 && child_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            if let Node::Flow(Flow::If(_)) = node {
                // If the relation is inlined in the parent, we need to update the old parent and null out the relation (i.e. "disconnect").
                Ok(Node::Flow(Flow::If(Box::new(move || {
                    !relation_inlined_parent && !child_ids.is_empty()
                }))))
            } else {
                unreachable!()
            }
        })),
    )?;

    graph.create_edge(&if_node, &update_existing_child, QueryGraphDependency::Then)?;
    graph.create_edge(&read_existing_children, &update_existing_child, QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut child_ids| {
            // This has to succeed or the if-then node wouldn't trigger.
            let child_id = match child_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating previous parent."))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                wq.add_filter(child_id.filter());
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(child_node)
        })))?;

    Ok(())
}

/// Inserts checks into the graph that check all required, non-list relations pointing to
/// the given `model`. Those checks fail at runtime (edges to the `Empty` node) if one or more
/// records are found. Checks are inserted between `parent_node` and `child_node`.
///
/// This function is usually part of a delete (`deleteOne` or `deleteMany`).
/// Expects `parent_node` to return one or more IDs (for records of `model`) to be checked.
///
/// ## Example for a standard delete scenario
/// - We have 2 relations, from `A` and `B` to `model`.
/// - This function inserts the nodes and edges in between `Find Record IDs` (`parent_node`) and
///   `Delete` (`child_node`) into the graph (but not the edge from `Find` to `Delete`, assumed already existing here).
///
/// ```text
///    ┌────────────────────┐
///    │ Find Record IDs to │
/// ┌──│       Delete       │
/// │  └────────────────────┘
/// │             │
/// │             ▼
/// │  ┌────────────────────┐
/// ├─▶│Find Connected Model│
/// │  │         A          │──┐
/// │  └────────────────────┘  │
/// │             │            │
/// │             ▼            │
/// │  ┌────────────────────┐  │
/// ├─▶│Find Connected Model│  │ Fail if > 0
/// │  │         B          │  │
/// │  └────────────────────┘  │
/// │             │Fail if > 0 │
/// │             ▼            │
/// │  ┌────────────────────┐  │
/// ├─▶│       Empty        │◀─┘
/// │  └────────────────────┘
/// │             │
/// │             ▼
/// │  ┌────────────────────┐
/// └─▶│       Delete       │
///    └────────────────────┘
/// ```
pub fn insert_deletion_checks(
    graph: &mut QueryGraph,
    model: &ModelRef,
    parent_node: &NodeRef,
    child_node: &NodeRef,
) -> QueryGraphBuilderResult<()> {
    let internal_model = model.internal_data_model();
    let relation_fields = internal_model.fields_requiring_model(model);
    let mut check_nodes = vec![];

    if relation_fields.len() > 0 {
        let noop_node = graph.create_node(Node::Empty);

        // We know that the relation can't be a list and must be required on the related model for `model` (see fields_requiring_model).
        // For all requiring models (RM), we use the field on `model` to query for existing RM records and error out if at least one exists.
        for rf in relation_fields {
            let relation_field = rf.related_field();
            let read_node = insert_find_children_by_parent_node(graph, parent_node, &relation_field, Filter::empty())?;

            graph.create_edge(
                &read_node,
                &noop_node,
                QueryGraphDependency::ParentIds(Box::new(move |node, parent_ids| {
                    if !parent_ids.is_empty() {
                        return Err(QueryGraphBuilderError::RelationViolation((relation_field).into()));
                    }

                    Ok(node)
                })),
            )?;

            check_nodes.push(read_node);
        }

        // Connects all `Find Connected Model` nodes with execution order dependency from the example in the docs.
        check_nodes.into_iter().fold1(|prev, next| {
            graph
                .create_edge(&prev, &next, QueryGraphDependency::ExecutionOrder)
                .unwrap();

            next
        });

        // Edge from empty node to the child (delete).
        graph.create_edge(&noop_node, child_node, QueryGraphDependency::ExecutionOrder)?;
    }

    Ok(())
}
