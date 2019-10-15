use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use connector::{Filter, ScalarCompare};
use prisma_models::{ModelRef, PrismaValue, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

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

pub fn connect_nested_delete_many(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    for value in utils::coerce_vec(value) {
        let as_map: ParsedInputMap = value.try_into()?;
        let filter = extract_filter(as_map, child_model)?;

        let find_child_records_node =
            utils::insert_find_children_by_parent_node(graph, parent, parent_relation_field, filter.clone())?;

        let update_many = WriteQuery::DeleteManyRecords(DeleteManyRecords {
            model: Arc::clone(&child_model),
            filter,
        });

        let delete_many_node = graph.create_node(Query::Write(update_many));
        let id_field = child_model.fields().id();

        graph.create_edge(
            &find_child_records_node,
            &delete_many_node,
            QueryGraphDependency::ParentIds(Box::new(move |mut node, mut parent_ids| {
                if let Node::Query(Query::Write(WriteQuery::DeleteManyRecords(ref mut ur))) = node {
                    // TODO: we should not clone here
                    let ids_filter = id_field.is_in(Some(parent_ids.clone()));
                    let new_filter = Filter::and(vec![ur.filter.clone(), ids_filter]);

                    ur.filter = new_filter;
                }

                Ok(node)
            })),
        )?;
    }
    Ok(())
}
