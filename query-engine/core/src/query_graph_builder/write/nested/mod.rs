mod connect_nested;
mod create_nested;
mod delete_nested;
mod update_nested;

use super::*;
use crate::{
    query_graph::{NodeRef, QueryGraph},
    ParsedInputMap,
};
use connect_nested::*;
use connector::filter::RecordFinder;
use create_nested::*;
use delete_nested::*;
use prisma_models::RelationFieldRef;
use update_nested::*;

pub fn connect_nested_query(
    graph: &mut QueryGraph,
    parent: &NodeRef,
    parent_relation_field: RelationFieldRef,
    data_map: ParsedInputMap,
) -> QueryGraphBuilderResult<()> {
    let child_model = parent_relation_field.related_model();

    for (field_name, value) in data_map {
        match field_name.as_str() {
            "create" => connect_nested_create(graph, parent, &parent_relation_field, value, &child_model)?,
            "update" => connect_nested_update(graph, parent, &parent_relation_field, value, &child_model)?,
            "connect" => connect_nested_connect(graph, parent, &parent_relation_field, value, &child_model)?,
            "delete" => connect_nested_delete(graph, parent, &parent_relation_field, value, &child_model)?,
            _ => (),
        };
    }

    Ok(())
}
