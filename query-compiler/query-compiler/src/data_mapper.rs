use crate::result_node::ResultNode;
use query_core::{
    CreateManyRecordsFields, DeleteRecordFields, Node, Query, QueryGraph, ReadQuery, UpdateManyRecordsFields,
    UpdateRecord, WriteQuery,
};
use query_structure::{AggregationSelection, FieldSelection, SelectedField};
use std::collections::HashMap;

pub fn map_result_structure(graph: &QueryGraph) -> Option<ResultNode> {
    for idx in graph.result_nodes() {
        let maybe_node = graph.node_content(&idx);
        if let Some(node) = maybe_node {
            if let Node::Query(query) = node {
                return map_query(query);
            }
        }
    }
    None
}

fn map_query(query: &Query) -> Option<ResultNode> {
    match query {
        Query::Read(read_query) => map_read_query(read_query),
        Query::Write(write_query) => map_write_query(write_query),
    }
}

fn map_read_query(query: &ReadQuery) -> Option<ResultNode> {
    match query {
        ReadQuery::RecordQuery(q) => get_result_node_nested(&q.selected_fields, &q.selection_order, &q.nested),
        ReadQuery::ManyRecordsQuery(q) => get_result_node_nested(&q.selected_fields, &q.selection_order, &q.nested),
        ReadQuery::RelatedRecordsQuery(q) => get_result_node_nested(&q.selected_fields, &q.selection_order, &q.nested),
        ReadQuery::AggregateRecordsQuery(q) => get_result_node_for_aggregation(&q.selectors, &q.selection_order),
    }
}

fn map_write_query(query: &WriteQuery) -> Option<ResultNode> {
    match query {
        WriteQuery::CreateRecord(q) => get_result_node(&q.selected_fields, &q.selection_order),
        WriteQuery::CreateManyRecords(q) => get_result_node_for_create_many(&q.selected_fields),
        WriteQuery::UpdateRecord(u) => {
            match u {
                UpdateRecord::WithSelection(w) => get_result_node(&w.selected_fields, &w.selection_order),
                UpdateRecord::WithoutSelection(_) => None, // No result data
            }
        }
        WriteQuery::DeleteRecord(q) => get_result_node_for_delete(&q.selected_fields),
        WriteQuery::UpdateManyRecords(q) => get_result_node_for_update_many(&q.selected_fields),
        WriteQuery::DeleteManyRecords(_) => None, // No result data
        WriteQuery::ConnectRecords(_) => None,    // No result data
        WriteQuery::DisconnectRecords(_) => None, // No result data
        WriteQuery::ExecuteRaw(_) => None,        // Has no data mapping
        WriteQuery::QueryRaw(_) => None,          // Has no data mapping
        WriteQuery::Upsert(q) => get_result_node(&q.selected_fields, &q.selection_order),
    }
}

fn get_result_node_nested(
    field_selection: &FieldSelection,
    selection_order: &Vec<String>,
    _children: &Vec<ReadQuery>,
) -> Option<ResultNode> {
    get_result_node(field_selection, selection_order)
}

fn get_result_node(field_selection: &FieldSelection, selection_order: &Vec<String>) -> Option<ResultNode> {
    let field_map = field_selection
        .selections()
        .map(|fs| (fs.prisma_name(), fs))
        .collect::<HashMap<_, _>>();

    let mut node = ResultNode::new_object();
    for prisma_name in selection_order {
        match field_map.get(prisma_name.as_str()) {
            None => {}
            Some(sf) => match sf {
                SelectedField::Scalar(f) => {
                    let prisma_type = f.type_identifier_with_arity().0.to_prisma_type();
                    node.add_field(
                        prisma_name,
                        ResultNode::new_value(sf.db_name().into_owned(), prisma_type),
                    );
                }
                SelectedField::Composite(_) => todo!("MongoDB specific"),
                SelectedField::Relation(_) => todo!("Needs recursion"),
                SelectedField::Virtual(f) => {
                    let prisma_type = f.type_identifier_with_arity().0.to_prisma_type();
                    let serialized_name = f.serialized_name();
                    let child = node
                        .get_entry(serialized_name.0)
                        .or_insert_with(|| ResultNode::new_object());
                    child.add_field(serialized_name.1, ResultNode::new_value(f.db_alias(), prisma_type));
                }
            },
        }
    }

    Some(node)
}

fn get_result_node_for_aggregation(
    p0: &Vec<AggregationSelection>,
    p1: &Vec<(String, Option<Vec<String>>)>,
) -> Option<ResultNode> {
    None
}

fn get_result_node_for_create_many(p0: &Option<CreateManyRecordsFields>) -> Option<ResultNode> {
    None
}

fn get_result_node_for_delete(p0: &Option<DeleteRecordFields>) -> Option<ResultNode> {
    None
}

fn get_result_node_for_update_many(p0: &Option<UpdateManyRecordsFields>) -> Option<ResultNode> {
    None
}
