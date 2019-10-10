use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{filter::RecordFinder, QueryArguments};
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

/// Swaps `parent` and `child` nodes.
///
/// ## When is a node swap necessary?
/// A common example: `parent` is a create and holds the inlined relation field, e.g. the foreign key in SQL terms.
/// This means we can't create the parent without knowing the actual ID of the child first.
/// Hence, we need to execute the child node first to get the child ID, then the
/// parent node can execute.
///
/// Important: How the swapped nodes are connected in the end is the callers to decide.
///
/// Performing a swap involves:
/// - Removing all edges from the parent to its parents
/// - Rewiring the previously removed edges to the child.
///
/// Notes:
/// - The parent keeps its child nodes.
/// - Any edge already existing between parent and child are not considered and NOT TOUCHED here.
///
/// ## Example
/// Take the following GraphQL query:
/// ```graphql
/// mutation {
///   createOneAlbum(
///     data: {
///       Title: "Master of Puppets"
///       Artist: { create: { Name: "Metallica" } }
///     }
///   ) {
///     id
///     Artist {
///       id
///     }
///   }
/// }
/// ```
///
/// The resulting query graph would look like this:
///```text
/// ┌─────────────┐
/// │Create Album │───────────┐
/// └─────────────┘           │
///        │                  │
///        │                  │
///        ▼                  ▼
/// ┌─────────────┐    ┌────────────┐
/// │Create Artist│    │ Read Album │
/// └─────────────┘    └────────────┘
///```
/// However, in a typical SQL database, the `Album` table holds the foreign key to Artist, as the relation is usually
/// "An Artist has many Albums, an Album has one Artist".
///
/// This would lead to the execution engine executing the `Create Album` query and failing, because we don't have the
/// foreign key for `Artist` - it doesn't exist yet. Hence, we swap the create queries and ensure that the execution
/// order is serialized in a way that necessary results for `Create Album` are available when it executes.
///
/// The result of the transformation looks like this in our example:
///```text
/// ┌─────────────┐
/// │Create Artist│
/// └─────────────┘
///        │
///        ▼
/// ┌─────────────┐
/// │Create Album │
/// └─────────────┘
///        │
///        ▼
/// ┌─────────────┐
/// │ Read Result │
/// └─────────────┘
///```
///
/// Please note that the decision of when to swap nodes is entirely on the callers side.
/// This function only swaps and doesn't check if the swap is necessary.
///
/// ## Return values
///
/// Returns (parent `NodeRef`, child `NodeRef`, relation field on parent `RelationFieldRef`) for convenience.
pub fn swap_nodes(
    // todo rename and redoc, this isn't doing a swap anymore
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    child_node: NodeRef,
) -> QueryGraphBuilderResult<(NodeRef, NodeRef)> {
    let parent_edges = graph.incoming_edges(&parent_node);
    for parent_edge in parent_edges {
        let parent_of_parent_node = graph.edge_source(&parent_edge);
        // let edge_content = graph.remove_edge(parent_edge).unwrap();

        // Todo: Warning, this assumes the edge contents can also be swapped.
        println!(
            "[Swap] Connecting parent of parent {} with child {}",
            parent_of_parent_node.id(),
            child_node.id()
        );
        graph.create_edge(
            &parent_of_parent_node,
            &child_node,
            QueryGraphDependency::ExecutionOrder,
        )?;
    }

    Ok((child_node, parent_node))
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
        name: "".into(),
        alias: None,
        model: Arc::clone(&model),
        args: record_finder.into(),
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
) -> QueryGraphBuilderResult<()> {
    let parent_model = parent_relation_field.model();
    let parent_model_id_field = parent_model.fields().id();
    let parent_model_name = parent_model.name.clone();
    let parent_side_required = parent_relation_field.is_required;
    let relation_inlined_parent = parent_relation_field.relation_is_inlined_in_parent();
    let rf = Arc::clone(&parent_relation_field);

    // Now check and disconnect the existing model, if necessary.
    let read_existing_parent_query_node =
        insert_find_children_by_parent_node(graph, &parent_node, &parent_relation_field, None)?;

    // If the parent side is required, we also fail during runtime before disconnecting, as that would violate the parent relation side.
    let update_existing_parent_node = update_record_node_placeholder(graph, None, parent_model);
    let relation_field_name = parent_relation_field.name.clone();
    let if_node = graph.create_node(Flow::default_if());

    graph.create_edge(
        &read_existing_parent_query_node,
        &if_node,
        QueryGraphDependency::ParentIds(Box::new(move |node, parent_ids| {
            if let Node::Flow(Flow::If(_)) = node {
                // If the relation is inlined in the parent, we need to update the old parent and null out the relation (i.e. "disconnect").
                Ok(Node::Flow(Flow::If(Box::new(move || {
                    relation_inlined_parent && !parent_ids.is_empty()
                }))))
            } else {
                unreachable!()
            }
        })),
    )?;

    graph.create_edge(&if_node, &update_existing_parent_node, QueryGraphDependency::Then)?;
    graph.create_edge(&read_existing_parent_query_node, &update_existing_parent_node, QueryGraphDependency::ParentIds(Box::new(move |mut child_node, mut parent_ids| {
            // If the parent requires the connection, we need to make sure that there isn't a parent already connected
            // to the existing child, as that would violate the other parent's relation side.
            if parent_ids.len() > 0 && parent_side_required {
                return Err(QueryGraphBuilderError::RelationViolation(rf.into()));
            }

            // This has to succeed or the if-then node wouldn't trigger.
            let parent_id = match parent_ids.pop() {
                Some(pid) => Ok(pid),
                None => Err(QueryGraphBuilderError::AssertionError(format!("[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-one relation, updating previous parent."))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = child_node {
                println!("[1:1 Checks] Injecting field '{}' with value '{:?}', to update existing parent node from read existing parent check (model: {}) ", &relation_field_name, &parent_id, parent_model_name);

                let finder = RecordFinder {
                    field: parent_model_id_field,
                    value: parent_id,
                };

                wq.inject_record_finder(finder);
                wq.inject_non_list_arg(relation_field_name, PrismaValue::Null);
            }

            Ok(child_node)
        })))?;

    Ok(())
}
