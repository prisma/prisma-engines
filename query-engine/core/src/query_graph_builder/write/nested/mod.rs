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
use schema::ConnectorContext;
use schema_builder::constants::operations;
use set_nested::*;
use update_nested::*;
use upsert_nested::*;

#[rustfmt::skip]
pub fn connect_nested_query(
    graph: &mut QueryGraph,
    connector_ctx: &ConnectorContext,
    parent: NodeRef,
    parent_relation_field: RelationFieldRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();

    for (field_name, value) in data_map {
        match field_name.as_str() {
            operations::CREATE => nested_create(graph, connector_ctx,parent, &parent_relation_field, value, &child_model)?,
            operations::CREATE_MANY => nested_create_many(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::UPDATE => nested_update(graph, connector_ctx, &parent, &parent_relation_field, value, &child_model)?,
            operations::UPSERT => nested_upsert(graph, connector_ctx, parent, &parent_relation_field, value)?,
            operations::DELETE => nested_delete(graph, connector_ctx, &parent, &parent_relation_field, value, &child_model)?,
            operations::CONNECT => nested_connect(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::DISCONNECT => nested_disconnect(graph, parent, &parent_relation_field, value, &child_model)?,
            operations::SET => nested_set(graph, &parent, &parent_relation_field, value, &child_model)?,
            operations::UPDATE_MANY => nested_update_many(graph, connector_ctx, &parent, &parent_relation_field, value, &child_model)?,
            operations::DELETE_MANY => nested_delete_many(graph, connector_ctx, &parent, &parent_relation_field, value, &child_model)?,
            operations::CONNECT_OR_CREATE => nested_connect_or_create(graph, connector_ctx, parent, &parent_relation_field, value, &child_model)?,
            _ => panic!("Unhandled nested operation: {}", field_name),
        };
    }

    Ok(())
}
