mod connect_nested;
mod connect_or_create_nested;
mod create_nested;
mod delete_nested;
mod disconnect_nested;
mod set_nested;
mod update_nested;
mod upsert_nested;

use super::*;
use crate::{
    query_graph::{NodeRef, QueryGraph},
    ParsedInputMap,
};
use connect_nested::*;
use connect_or_create_nested::*;
use create_nested::*;
use delete_nested::*;
use disconnect_nested::*;
use prisma_models::RelationFieldRef;
use set_nested::*;
use update_nested::*;
use upsert_nested::*;

pub fn connect_nested_query(
    graph: &mut QueryGraph,
    parent: NodeRef,
    parent_relation_field: RelationFieldRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();

    for (field_name, value) in data_map {
        match field_name.as_str() {
            "create" => nested_create(graph, parent, &parent_relation_field, value, &child_model)?,
            "update" => nested_update(graph, &parent, &parent_relation_field, value, &child_model)?,
            "upsert" => nested_upsert(graph, parent, &parent_relation_field, value)?,
            "delete" => nested_delete(graph, &parent, &parent_relation_field, value, &child_model)?,
            "connect" => nested_connect(graph, parent, &parent_relation_field, value, &child_model)?,
            "disconnect" => nested_disconnect(graph, parent, &parent_relation_field, value, &child_model)?,
            "set" => nested_set(graph, &parent, &parent_relation_field, value, &child_model)?,
            "updateMany" => nested_update_many(graph, &parent, &parent_relation_field, value, &child_model)?,
            "deleteMany" => nested_delete_many(graph, &parent, &parent_relation_field, value, &child_model)?,
            "connectOrCreate" => nested_connect_or_create(graph, parent, &parent_relation_field, value, &child_model)?,
            _ => panic!("Unhandled nested operation: {}", field_name),
        };
    }

    Ok(())
}
