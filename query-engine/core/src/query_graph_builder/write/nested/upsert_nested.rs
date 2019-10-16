use super::*;
use crate::query_graph_builder::write::utils::coerce_vec;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue,
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

pub fn connect_nested_upsert(
    graph: &mut QueryGraph,
    parent_node: NodeRef,
    parent_relation_field: &RelationFieldRef,
    value: ParsedInputValue,
    child_model: &ModelRef,
) -> QueryGraphBuilderResult<()> {
    dbg!(&value);
    let model = parent_relation_field.related_model();
    let relation = parent_relation_field.relation();

    for value in coerce_vec(value) {
        let mut as_map: ParsedInputMap = value.try_into()?;
        let where_input = as_map.remove("where").expect("where argument is missing");
        let create_input = as_map.remove("create").expect("create argument is missing");
        let update_input = as_map.remove("update").expect("update argument is missing");

        let record_finder = extract_record_finder(where_input, &model)?;
        let child_read_query = utils::id_read_query_infallible(&model, record_finder.clone());
        let initial_read_node = graph.create_node(child_read_query);

        let create_node = create::create_record_node(graph, Arc::clone(&model), create_input.try_into()?)?;
        let update_node =
            update::update_record_node(graph, Some(record_finder), Arc::clone(&model), update_input.try_into()?)?;

        let if_node = graph.create_node(Flow::default_if());

        graph.create_edge(
            &initial_read_node,
            &if_node,
            QueryGraphDependency::ParentIds(Box::new(|node, parent_ids| {
                println!("IF gets called");
                if let Node::Flow(Flow::If(_)) = node {
                    // Todo: This looks super unnecessary
                    Ok(Node::Flow(Flow::If(Box::new(move || !parent_ids.is_empty()))))
                } else {
                    Ok(node)
                }
            })),
        )?;

        graph.create_edge(&if_node, &update_node, QueryGraphDependency::Then)?;

        graph.create_edge(&if_node, &create_node, QueryGraphDependency::Else)?;

        if !parent_relation_field.relation_is_inlined_in_parent() {
            let related_field_name = parent_relation_field.related_field().name.clone();
            graph.create_edge(
                &parent_node,
                &create_node,
                QueryGraphDependency::ParentIds(Box::new(|mut child_node, mut parent_ids| {
                    println!("THIS gets called");
                    let parent_id = match parent_ids.pop() {
                        Some(pid) => Ok(pid),
                        None => Err(QueryGraphBuilderError::AssertionError(format!(
                            "[Query Graph] Expected a valid parent ID to be present for a nested create in a nested upsert."
                        ))),
                    }?;

                    if let Node::Query(Query::Write(ref mut wq)) = child_node {
                        wq.inject_non_list_arg(related_field_name, parent_id);
                    }

                    Ok(child_node)
                })),
            )?;
        }

        graph.create_edge(&parent_node, &initial_read_node, QueryGraphDependency::ExecutionOrder)?;
    }

    Ok(())
}
