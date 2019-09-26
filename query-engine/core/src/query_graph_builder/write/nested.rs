use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputMap, ParsedInputValue,
};
use connector::filter::RecordFinder;
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

pub fn connect_nested_query(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: RelationFieldRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<()> {
    let model = relation_field.related_model();

    for (field_name, value) in data_map {
        match field_name.as_str() {
            "create" => connect_nested_create(graph, parent, &relation_field, value, &model)?,
            "update" => connect_nested_update(graph, parent, &relation_field, value, &model)?,
            "connect" => connect_nested_connect(graph, parent, &relation_field, value, &model)?,
            "delete" => connect_nested_delete(graph, parent, &relation_field, value, &model)?,
            _ => (),
        };
    }

    Ok(())
}

pub fn connect_nested_create(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let child = create::create_record_node(graph, Arc::clone(model), value.try_into()?)?;
        let relation_field_name = relation_field.name.clone();
        let (parent, child) = utils::flip_nodes(graph, parent, &child, relation_field);

        graph.create_edge(
            parent,
            child,
            QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                // The following injection is necessary for cases where the relation is inlined.
                // The injection won't do anything in other cases.
                // The other case where the relation is not inlined is handled further down.
                if let Node::Query(Query::Write(ref mut wq)) = node {
                    wq.inject_non_list_arg(relation_field_name, parent_id.unwrap());
                }

                node
            })),
        );

        // Detect if a connect is necessary between the nodes.
        // A connect is necessary if the nested create is done on a relation that
        // is a list (x-to-many) and where the relation is not inlined (aka manifested as an
        // actual join table, for example).
        if relation_field.is_list && !relation_field.relation().is_inline_relation() {
            connect::connect_records_node(graph, parent, child, relation_field);
        }
    }

    Ok(())
}

pub fn connect_nested_update(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        if relation_field.is_list {
            // We have a record specified as a record finder in "where"
            let mut map: ParsedInputMap = value.try_into()?;
            let where_arg = map.remove("where").unwrap();
            let record_finder = extract_record_finder(where_arg, &model)?;
            let data_value = map.remove("data").unwrap();
            let update_node =
                update::update_record_node(graph, Some(record_finder), Arc::clone(model), data_value.try_into()?)?;

            graph.create_edge(parent, &update_node, QueryGraphDependency::ExecutionOrder);
        } else {
            let find_child_records_node = utils::find_ids_by_parent(graph, relation_field, parent);
            let update_node = update::update_record_node(graph, None, Arc::clone(model), value.try_into()?)?;
            let id_field = model.fields().id();

            graph.create_edge(
                &find_child_records_node,
                &update_node,
                QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                    if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = node {
                        ur.where_ = Some(RecordFinder {
                            field: id_field,
                            value: parent_id.unwrap(),
                        });
                    }

                    node
                })),
            );
        }
    }

    Ok(())
}

pub fn connect_nested_connect(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        // First, we need to build a read query on the record to be conneted.
        let record_finder = extract_record_finder(value, &model)?;
        let child_read_query = utils::id_read_query_infallible(&model, record_finder);
        let child_node = graph.create_node(child_read_query);

        // Flip the read node and parent node if necessary.
        let (parent, child) = utils::flip_nodes(graph, parent, &child_node, relation_field);
        let relation_field_name = relation_field.name.clone();

        // Edge from parent to child (e.g. Create to ReadQuery).
        graph.create_edge(
            parent,
            child,
            QueryGraphDependency::ParentId(Box::new(|mut child_node, parent_id| {
                // If the child is a write query, inject the parent id.
                // This covers cases of inlined relations.
                if let Node::Query(Query::Write(ref mut wq)) = child_node {
                    wq.inject_non_list_arg(relation_field_name, parent_id.unwrap())
                }

                child_node
            })),
        );

        // Detect if a connect is actually necessary between the nodes.
        // A connect is necessary if the nested connect is done on a relation that
        // is a list (x-to-many) and where the relation is not inlined (aka manifested as an
        // actual join table, for example).
        if relation_field.is_list && !relation_field.relation().is_inline_relation() {
            connect::connect_records_node(graph, parent, child, relation_field);
        }
    }

    Ok(())
}

/// Adds a delete (single) record node to the graph and connects it to the parent.
/// Auxiliary nodes may be added to support the deletion process, e.g. extra read nodes.
///
/// If the relation is a list:
/// - Delete specific record from the list, a record finder must be present in the data.
///
/// If the relation is not a list:
/// - Just delete the one node that can be present, if desired (as it is a non-list, aka 1-to-1 relation).
/// - The relation HAS to be inlined, because it is 1-to-1.
/// - If the relation is inlined in the parent, we need to generate a read query to grab the id of the record we want to delete.
/// - If the relation is inlined but not in the parent, we can directly generate a delete on the record with the parent ID.
pub fn connect_nested_delete(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        if relation_field.is_list {
            // Todo:
            // - we need to make sure the records are actually connected...
            // - What about the checks currently performed in `DeleteActions`?
            let record_finder = extract_record_finder(value, &model)?;
            let delete_node = delete::delete_record_node(graph, Some(record_finder), Arc::clone(&model));

            // graph.create_edge(parent, to: &NodeRef, content: QueryGraphDependency);
            unimplemented!()
        } else {
            // if relation_field.relation_is_inlined_in_parent() {
            //     let delete_node = delete::delete_record_node(graph, None, Arc::clone(&model));
            //     let find_child_records_node = utils::find_ids_by_parent(graph, relation_field, parent);

            //     None
            // } else {
            //     None
            // };

            let val: PrismaValue = value.try_into()?;
            match val {
                PrismaValue::Boolean(b) if b => unimplemented!(),
                // vec.push(NestedDeleteRecord {
                //     relation_field: Arc::clone(&relation_field),
                //     where_: None,
                // }),
                _ => (),
            };
        }
    }

    unimplemented!()
}
