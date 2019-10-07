use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{filter::RecordFinder, QueryArguments};
use prisma_models::{ModelRef, PrismaArgs, RelationFieldRef, SelectedFields};
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
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    child_node: NodeRef,
) -> (NodeRef, NodeRef) {
    let parent_edges = graph.incoming_edges(&parent_node);
    for parent_edge in parent_edges {
        let parent_of_parent_node = graph.edge_source(&parent_edge);
        let edge_content = graph.remove_edge(parent_edge).unwrap();

        // Todo: Warning, this assumes the edge contents can also be swapped.
        graph.create_edge(&parent_of_parent_node, &child_node, edge_content);
    }

    (child_node, parent_node)
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
pub fn insert_find_children_by_parent_node<T>(
    graph: &mut QueryGraph,
    parent_relation_field: &RelationFieldRef,
    parent: &NodeRef,
    filter: T,
) -> NodeRef
where
    T: Into<QueryArguments>,
{
    let read_parent_node = graph.create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
        name: "parent".to_owned(),
        alias: None,
        parent_field: Arc::clone(parent_relation_field),
        parent_ids: None,
        args: filter.into(),
        selected_fields: parent_relation_field.related_model().fields().id().into(), // Select related IDs
        nested: vec![],
        selection_order: vec![],
    })));

    graph.create_edge(
        parent,
        &read_parent_node,
        QueryGraphDependency::ParentIds(Box::new(|mut node, parent_ids| {
            if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = node {
                // We know that all PrismaValues in `parent_ids` are transformable into GraphqlIds.
                rq.parent_ids = Some(parent_ids.into_iter().map(|id| id.try_into().unwrap()).collect());
            };

            Ok(node)
        })),
    );

    read_parent_node
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
