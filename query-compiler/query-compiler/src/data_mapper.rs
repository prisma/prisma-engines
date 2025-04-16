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
        ReadQuery::RecordQuery(q) => get_result_node(&q.selected_fields, &q.selection_order, Some(&q.nested)),
        ReadQuery::ManyRecordsQuery(q) => get_result_node(&q.selected_fields, &q.selection_order, Some(&q.nested)),
        ReadQuery::RelatedRecordsQuery(q) => get_result_node(&q.selected_fields, &q.selection_order, Some(&q.nested)),
        ReadQuery::AggregateRecordsQuery(q) => get_result_node_for_aggregation(&q.selectors, &q.selection_order),
    }
}

fn map_write_query(query: &WriteQuery) -> Option<ResultNode> {
    match query {
        WriteQuery::CreateRecord(q) => get_result_node(&q.selected_fields, &q.selection_order, None),
        WriteQuery::CreateManyRecords(q) => get_result_node_for_create_many(&q.selected_fields),
        WriteQuery::UpdateRecord(u) => {
            match u {
                UpdateRecord::WithSelection(w) => get_result_node(&w.selected_fields, &w.selection_order, None),
                UpdateRecord::WithoutSelection(_) => None, // No result data
            }
        }
        WriteQuery::DeleteRecord(q) => get_result_node_for_delete(&q.selected_fields),
        WriteQuery::UpdateManyRecords(q) => get_result_node_for_update_many(&q.selected_fields),
        WriteQuery::DeleteManyRecords(_) => None, // No result data
        WriteQuery::ConnectRecords(_) => None,    // No result data
        WriteQuery::DisconnectRecords(_) => None, // No result data
        WriteQuery::ExecuteRaw(_) => None,        // No data mapping
        WriteQuery::QueryRaw(_) => None,          // No data mapping
        WriteQuery::Upsert(q) => get_result_node(&q.selected_fields, &q.selection_order, None),
    }
}

fn get_result_node(
    field_selection: &FieldSelection,
    selection_order: &Vec<String>,
    nested: Option<&Vec<ReadQuery>>,
) -> Option<ResultNode> {
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
                SelectedField::Relation(f) => {
                    let nested_selection = FieldSelection::new(f.selections.iter().map(|f| f.clone()).collect());
                    let nested_node = get_result_node(&nested_selection, &f.result_fields, None);
                    if let Some(nested_node) = nested_node {
                        node.add_field(f.field.name(), nested_node);
                    }
                }
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
    selectors: &Vec<AggregationSelection>,
    selection_order: &Vec<(String, Option<Vec<String>>)>,
) -> Option<ResultNode> {
/*    
    let mut node = ResultNode::new_object();
    let selector_map = selectors.iter().map(|s| (s.))

    for selector in selectors {
        
    }
    
    selectors.iter().map(|s| {
        match s {
            AggregationSelection::Field(_) => {}
            AggregationSelection::Count { .. } => {}
            AggregationSelection::Average(_) => {}
            AggregationSelection::Sum(_) => {}
            AggregationSelection::Min(_) => {}
            AggregationSelection::Max(_) => {}
        }
    })
    
    
    match selected_fields {
        None => None,
        Some(sf) => get_result_node(&sf.fields, &sf.order, Some(&sf.nested)),
    }
 */
    None
}

fn get_result_node_for_create_many(selected_fields: &Option<CreateManyRecordsFields>) -> Option<ResultNode> {
    match selected_fields {
        None => None,
        Some(sf) => get_result_node(&sf.fields, &sf.order, Some(&sf.nested)),
    }
}

fn get_result_node_for_delete(selected_fields: &Option<DeleteRecordFields>) -> Option<ResultNode> {
    match selected_fields {
        None => None,
        Some(sf) => get_result_node(&sf.fields, &sf.order, None),
    }
}

fn get_result_node_for_update_many(selected_fields: &Option<UpdateManyRecordsFields>) -> Option<ResultNode> {
    match selected_fields {
        None => None,
        Some(sf) => get_result_node(&sf.fields, &sf.order, Some(&sf.nested)),
    }
}
