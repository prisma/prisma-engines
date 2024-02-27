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
use query_structure::RelationFieldRef;
use schema::{constants::operations, QuerySchema};
use set_nested::*;
use update_nested::*;
use upsert_nested::*;

#[rustfmt::skip]
pub fn connect_nested_query(
    graph: &mut QueryGraph,
    query_schema: &QuerySchema,
    parent: NodeRef,
    parent_relation_field: RelationFieldRef,
    data_map: ParsedInputMap<'_>,
    ctx: Option<&CompileContext>,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();

    // TODO laplab: assert that `parent_results` is `None` in all other cases?
    for (field_name, value) in data_map {
        match field_name.as_ref() {
            operations::CREATE => nested_create(graph, query_schema,parent, &parent_relation_field, value, &child_model)?,
            operations::CREATE_MANY => nested_create_many(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::UPDATE => nested_update(graph, query_schema, &parent, &parent_relation_field, value, &child_model, ctx)?,
            operations::UPSERT => nested_upsert(graph, query_schema, parent, &parent_relation_field, value)?,
            operations::DELETE => nested_delete(graph, query_schema, &parent, &parent_relation_field, value, &child_model)?,
            operations::CONNECT => nested_connect(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::DISCONNECT => nested_disconnect(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::SET => nested_set(graph, &parent, &parent_relation_field, value, &child_model)?,
            operations::UPDATE_MANY => nested_update_many(graph, query_schema, &parent, &parent_relation_field, value, &child_model)?,
            operations::DELETE_MANY => nested_delete_many(graph, query_schema, &parent, &parent_relation_field, value, &child_model)?,
            operations::CONNECT_OR_CREATE => nested_connect_or_create(graph, query_schema, parent, &parent_relation_field, value, &child_model)?,
            _ => panic!("Unhandled nested operation: {field_name}"),
        };
    }

    Ok(())
}

// TODO laplab: comment
pub fn assumes_parent_exists(data_map: &ParsedInputMap<'_>) -> bool {
    for (field_name, _value) in data_map.iter() {
        let assumes = match field_name.as_ref() {
            operations::CREATE
            | operations::CREATE_MANY
            | operations::UPSERT
            | operations::CONNECT
            | operations::DISCONNECT
            | operations::SET
            | operations::CONNECT_OR_CREATE
            | operations::UPDATE_MANY
            | operations::DELETE_MANY => true,
            operations::UPDATE | operations::DELETE => false,
            _ => panic!("Unhandled nested operation: {field_name}"),
        };
        if assumes {
            return true;
        }
    }
    false
}
