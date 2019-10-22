use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{filter::RecordFinder, Filter, QueryArguments};
use prisma_models::{ModelRef, PrismaArgs, PrismaValue, RelationFieldRef, SelectedFields};
use std::{convert::TryInto, sync::Arc};

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

/// Produces a non-failing ReadQuery for a given RecordFinder by using
/// a ManyRecordsQuery instead of a find one (i.e. returns empty list instead of "not found" error).
pub fn id_read_query_infallible(model: &ModelRef, record_finder: RecordFinder) -> Query {
    let selected_fields: SelectedFields = model.fields().id().into();
    let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name: "id_read_query_infallible".into(), // this name only eases debugging
        alias: None,
        model: Arc::clone(&model),
        args: record_finder.into(),
        selected_fields,
        nested: vec![],
        selection_order: vec![],
    });

    Query::Read(read_query)
}

pub fn ids_read_query_infallible(model: &ModelRef, finders: Vec<RecordFinder>) -> Query {
    let selected_fields: SelectedFields = model.fields().id().into();
    let as_filters: Vec<Filter> = finders.into_iter().map(|x| x.into()).collect();

    let read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
        name: "id_read_query_infallible".into(), // this name only eases debugging
        alias: None,
        model: Arc::clone(&model),
        args: Filter::or(as_filters).into(),
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
    let selected_fields = SelectedFields::new(
        vec![parent_relation_field.related_model().fields().id().into()],
        Some(Arc::clone(parent_relation_field)),
    );

    let read_parent_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "parent".to_owned(),
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

// Creates an "empty" query node. Sometimes required for
// Todo: Consider elevating the placeholder concept to the actual graph.
// - Prevents accidential reads, could just error if placeholder hasn't been replaced during building.
// - Definitely the cleaner solution.
// pub fn insert_query_node_placeholder(graph: &mut QueryGraph) -> NodeRef {
//     graph.create_node(Query::Read(ReadQuery::RecordQuery(RecordQuery::default())))
// }

/// Creates an update record query node and adds it to the query graph.
/// Used to have a skeleton update node in the graph that can be further transformed during query execution based
/// on available information.
pub fn update_record_node_placeholder(
    graph: &mut QueryGraph,
    record_finder: Option<RecordFinder>,
    model: ModelRef,
) -> NodeRef {
    let mut args = PrismaArgs::new();

    // args.insert(field.name(), value);
    args.update_datetimes(Arc::clone(&model), false);

    let ur = UpdateRecord {
        model,
        where_: record_finder,
        non_list_args: args,
        list_args: vec![],
    };

    graph.create_node(Query::Write(WriteQuery::UpdateRecord(ur)))
}

/// Inserts checks and disconnects for existing models for a 1:1 relation.
/// Expects that the parent node returns a valid ID for the model the `parent_relation_field` is located on.
///
/// Params:
/// `parent_node`: Node that provides the parent id for the find query and where the checks are appended to in the graph.
/// `parent_relation_field`: Field on the parent model to find children.
///
/// Todo: This function is virtually identical to the existing child check. Consolidate, if possible.
///
/// The elements added to the graph are all except `Append Node`:
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
///              ▼              │(Fail on p > 0 if parent
/// ┌────────────────────────┐  │     side required)
/// │ If p > 0 && p. inlined │  │
/// └────────────────────────┘  │
///         then │              │
///              ▼              │
/// ┌────────────────────────┐  │
/// │    Update ex. model    │◀─┘
/// └────────────────────────┘
/// ```
///
/// We only need to actually update ("disconnect") the existing model if
/// the relation is also inlined on that models side, so we put that check into the if flow.
pub fn insert_existing_1to1_related_model_checks(
    graph: &mut QueryGraph,
    parent_node: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    perform_relation_check: bool,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();
    let child_model_name = child_model.name.clone();
    let child_model_id_field = child_model.fields().id();
    let parent_side_required = parent_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let rf = Arc::clone(&parent_relation_field);

    let read_existing_children =
        insert_find_children_by_parent_node(graph, &parent_node, &parent_relation_field, None)?;

    // If the parent side is required, we also fail during runtime before disconnecting, as that would violate the parent relation side.
    let update_existing_child = update_record_node_placeholder(graph, None, child_model);
    let relation_field_name = parent_relation_field.related_field().name.clone();
    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_existing_children,
        &if_node,
        QueryGraphDependency::ParentIds(Box::new(move |node, child_ids| {
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
            println!("[1:1 Checks] Relation field: {}, child ids: {:?}", &relation_field_name, &child_ids);

            // If the parent requires the connection, we need to make sure that there isn't a parent already connected
            // to the existing child, as that would violate the other parent's relation side.
            if perform_relation_check && child_ids.len() > 0 && parent_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            // This has to succeed or the if-then node wouldn't trigger.
            let child_id = match child_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating previous parent."))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                println!("[1:1 Checks] Injecting field '{}' with value '{:?}', to update existing child node from read existing children check (model: {}) ", &relation_field_name, &child_id, child_model_name);

                let finder = RecordFinder {
                    field: child_model_id_field,
                    value: child_id,
                };

                wq.inject_record_finder(finder);
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(child_node)
        })))?;

    Ok(())
}
