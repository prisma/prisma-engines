use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    write::write_args_parser::WriteArgsParser,
    ParsedInputList, ParsedInputValue,
};
use psl::datamodel_connector::ConnectorCapability;
use query_structure::{Filter, IntoFilter, Model, RelationFieldRef};
use schema::constants::args;
use std::convert::TryInto;

/// Handles nested create one cases.
/// The resulting graph can take multiple forms, based on the relation type to the parent model.
/// Information on the graph shapes can be found on the individual handlers.
pub fn nested_create(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    let relation = parent_relation_field.relation();

    let data_maps = utils::coerce_vec(value)
        .into_iter()
        .map(|value| {
            let mut parser = WriteArgsParser::from(child_model, value.try_into()?)?;
            parser.args.add_datetimes(child_model);
            Ok((parser.args, parser.nested))
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;
    let child_records_count = data_maps.len();

    // In some limited cases, we create related records in bulk. The conditions are:
    // 1. Connector must support creating records in bulk
    // 2. The number of child records should be greater than one. Technically, there is nothing
    //    preventing us from using bulk creation for that case, it just does not make a lot of sense
    // 3. None of the children have any nested create operations. The main reason for this
    //    limitation is that we do not have any ordering guarantees on records returned from the
    //    database after their creation. To put it simply, `INSERT ... RETURNING *` can and will
    //    return records in random order, at least on Postgres. This means that if have 10 children
    //    records of model X, each of which has 10 children records of model Y, we won't be able to
    //    associate created records from model X with their children from model Y.
    // 4. Relation is not 1-1. Again, no technical limitations here, but we know that there can only
    //    ever be a single related record, so we do not support it in bulk operations due to (2).
    // 5. If relation is 1-many, it must be inlined in children. Otherwise, we once again have only
    //    one possible related record, see (2).
    // 6. If relation is many-many, connector needs to support `RETURNING` or something similar,
    //    because we need to know the ids of created children records.
    let has_create_many = query_schema.has_capability(ConnectorCapability::CreateMany);
    let has_returning = query_schema.has_capability(ConnectorCapability::InsertReturning);
    let is_one_to_many_in_child = relation.is_one_to_many() && parent_relation_field.relation_is_inlined_in_child();
    let is_many_to_many = relation.is_many_to_many() && has_returning;
    let has_nested = data_maps.iter().any(|(_args, nested)| !nested.is_empty());
    let should_use_bulk_create =
        has_create_many && child_records_count > 1 && !has_nested && (is_one_to_many_in_child || is_many_to_many);

    if should_use_bulk_create {
        // Create all child records in a single query.
        let selected_fields = if relation.is_many_to_many() {
            let selected_fields = child_model.primary_identifier();
            let selection_order = selected_fields.db_names().collect();
            Some(CreateManyRecordsFields {
                fields: selected_fields,
                order: selection_order,
                nested: Vec::new(),
            })
        } else {
            None
        };
        let query = CreateManyRecords {
            name: String::new(), // This node will not be serialized so we don't need a name.
            model: child_model.clone(),
            args: data_maps.into_iter().map(|(args, _nested)| args).collect(),
            skip_duplicates: false,
            selected_fields,
            split_by_shape: !query_schema.has_capability(ConnectorCapability::SupportsDefaultInInsert),
        };
        let create_many_node = graph.create_node(Query::Write(WriteQuery::CreateManyRecords(query)));

        if relation.is_one_to_many() {
            handle_one_to_many_bulk(graph, parent_node, parent_relation_field, create_many_node)?;
        } else {
            handle_many_to_many_bulk(
                graph,
                parent_node,
                parent_relation_field,
                create_many_node,
                child_records_count,
            )?;
        }
    } else {
        // Create each child record separately.
        let creates = data_maps
            .into_iter()
            .map(|(args, nested)| {
                create::create_record_node_from_args(graph, query_schema, child_model.clone(), args, nested)
            })
            .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

        if relation.is_many_to_many() {
            handle_many_to_many(graph, parent_node, parent_relation_field, creates)?;
        } else if relation.is_one_to_many() {
            handle_one_to_many(graph, parent_node, parent_relation_field, creates)?;
        } else {
            handle_one_to_one(graph, parent_node, parent_relation_field, creates)?;
        }
    }

    Ok(())
}

/// Handles one-to-many nested bulk create.
///
/// This function only considers the case where relation is inlined in child.
/// `parent_node` produces single ID of "one" side of the relation.
/// `child_node` creates records for the "many" side of the relation, using ID from `parent_node`.
///
/// Resulting graph consists of just `parent_node` and `child_node` connected with an edge.
fn handle_one_to_many_bulk(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    child_node: NodeRef,
) -> QueryGraphBuilderResult<()> {
    let parent_link = parent_relation_field.linking_fields();
    let child_link = parent_relation_field.related_field().linking_fields();

    let relation_name = parent_relation_field.relation().name().to_owned();
    let parent_model_name = parent_relation_field.model().name().to_owned();
    let child_model_name = parent_relation_field.related_model().name().to_owned();

    graph.create_edge(
        &parent_node,
        &child_node,
        QueryGraphDependency::ProjectedDataDependency(parent_link, Box::new(move |mut create_node, mut parent_links| {
            let parent_link = match parent_links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{parent_model_name}' record (needed to inline the relation on '{child_model_name}' record) was found for a nested create on one-to-many relation '{relation_name}'."
                ))),
            }?;

            if let Node::Query(Query::Write(ref mut wq)) = create_node {
                wq.inject_result_into_args(child_link.assimilate(parent_link)?);
            }

            Ok(create_node)
        })))?;

    Ok(())
}

/// Handles many-to-many nested bulk create.
///
/// `parent_node` produces single ID of one side of the many-to-many relation.
/// `child_node` produces multiple IDs of another side of many-to-many relation.
///
/// Please refer to the `connect::connect_records_node` documentation for the resulting graph shape.
fn handle_many_to_many_bulk(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    child_node: NodeRef,
    expected_connects: usize,
) -> QueryGraphBuilderResult<()> {
    graph.create_edge(&parent_node, &child_node, QueryGraphDependency::ExecutionOrder)?;
    connect::connect_records_node(
        graph,
        &parent_node,
        &child_node,
        parent_relation_field,
        expected_connects,
    )?;
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
fn handle_many_to_many(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    create_nodes: Vec<NodeRef>,
) -> QueryGraphBuilderResult<()> {
    // Todo optimize with createMany
    for create_node in create_nodes {
        graph.create_edge(&parent_node, &create_node, QueryGraphDependency::ExecutionOrder)?;
        connect::connect_records_node(graph, &parent_node, &create_node, parent_relation_field, 1)?;
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

        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_relation_field.model().name().to_owned();
        let child_model_name = parent_relation_field.related_model().name().to_owned();

        // We extract the child linking fields in the edge, because after the swap, the child is the new parent.
        graph.create_edge(
            &parent_node,
            &child_node,
            QueryGraphDependency::ProjectedDataDependency(child_link, Box::new(move |mut parent_node, mut child_links| {
                let child_link = match child_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{child_model_name}' record (needed to inline the relation on '{parent_model_name}' record) was found for a nested create on one-to-many relation '{relation_name}'."
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

            let relation_name = parent_relation_field.relation().name().to_owned();
            let parent_model_name = parent_relation_field.model().name().to_owned();
            let child_model_name = parent_relation_field.related_model().name().to_owned();

            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ProjectedDataDependency(parent_link, Box::new(move |mut create_node, mut parent_links| {
                    let parent_link = match parent_links.pop() {
                        Some(link) => Ok(link),
                        None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                            "No '{parent_model_name}' record (needed to inline the relation on '{child_model_name}' record) was found for a nested create on one-to-many relation '{relation_name}'."
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

    let relation_name = parent_relation_field.relation().name();
    let parent_model_name = extractor_model.name().to_owned();
    let child_model_name = assimilator_model.name().to_owned();

    graph.create_edge(
        &parent_node,
        &create_node,
        QueryGraphDependency::ProjectedDataDependency(extractor, Box::new(move |mut child_node, mut links| {
            let link = match links.pop() {
                Some(link) => Ok(link),
                None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                    "No '{parent_model_name}' record (needed to inline the relation with create on '{child_model_name}' record) was found for a nested create on one-to-one relation '{relation_name}'."
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
        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_model.name().to_owned();
        let child_model_name = parent_relation_field.related_model().name().to_owned();
        let update_node = utils::update_records_node_placeholder(graph, Filter::empty(), parent_model);
        let parent_link = parent_relation_field.linking_fields();

        graph.create_edge(
            &create_node,
            &update_node,
            QueryGraphDependency::ProjectedDataDependency(child_link, Box::new(move |mut update_node, mut child_links| {
                let child_link = match child_links.pop() {
                    Some(link) => Ok(link),
                    None => Err(QueryGraphBuilderError::RecordNotFound(format!(
                        "No '{child_model_name}' record (needed to inline the relation with an update on '{parent_model_name}' record) was found for a nested create on one-to-one relation '{relation_name}'."
                    ))),
                }?;

                if let Node::Query(Query::Write(ref mut wq)) = update_node {
                    wq.inject_result_into_args(parent_link.assimilate(child_link)?);
                }

                Ok(update_node)
            })),
         )?;

        let parent_model_identifier = parent_relation_field.model().primary_identifier();
        let relation_name = parent_relation_field.relation().name();
        let parent_model_name = parent_relation_field.model().name().to_owned();

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

pub fn nested_create_many(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
    child_model: &Model,
) -> QueryGraphBuilderResult<()> {
    // Nested input is an object of { data: [...], skipDuplicates: bool }
    let mut obj: ParsedInputMap<'_> = value.try_into()?;

    let data_list: ParsedInputList<'_> = utils::coerce_vec(obj.swap_remove(args::DATA).unwrap());
    let skip_duplicates: bool = match obj.swap_remove(args::SKIP_DUPLICATES) {
        Some(val) => val.try_into()?,
        None => false,
    };

    let args = data_list
        .into_iter()
        .map(|data_value| {
            let data_map = data_value.try_into()?;
            let mut args = WriteArgsParser::from(child_model, data_map)?.args;

            args.add_datetimes(child_model);
            Ok(args)
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    let query = CreateManyRecords {
        name: String::new(), // This node will not be serialized so we don't need a name.
        model: child_model.clone(),
        args,
        skip_duplicates,
        selected_fields: None,
        split_by_shape: !query_schema.has_capability(ConnectorCapability::SupportsDefaultInInsert),
    };

    let create_node = graph.create_node(Query::Write(WriteQuery::CreateManyRecords(query)));

    // Currently, `createMany` is only supported for 1-many relations. This is checked during parsing.
    handle_one_to_many_bulk(graph, parent_node, parent_relation_field, create_node)
}
