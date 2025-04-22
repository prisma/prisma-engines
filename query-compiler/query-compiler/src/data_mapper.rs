use crate::result_node::ResultNode;
use query_core::{
    CreateManyRecordsFields, DeleteRecordFields, Node, Query, QueryGraph, ReadQuery, UpdateManyRecordsFields,
    UpdateRecord, WriteQuery,
};
use query_structure::{AggregationSelection, FieldSelection, SelectedField, TypeIdentifier};
use std::collections::HashMap;

pub fn map_result_structure(graph: &QueryGraph) -> Option<ResultNode> {
    for idx in graph.result_nodes().chain(graph.root_nodes()) {
        let node = graph.node_content(&idx);
        if let Some(Node::Query(query)) = node {
            return map_query(query);
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
        WriteQuery::CreateManyRecords(q) => get_result_node_for_create_many(q.selected_fields.as_ref()),
        WriteQuery::UpdateRecord(u) => {
            match u {
                UpdateRecord::WithSelection(w) => get_result_node(&w.selected_fields, &w.selection_order, None),
                UpdateRecord::WithoutSelection(_) => None, // No result data
            }
        }
        WriteQuery::DeleteRecord(q) => get_result_node_for_delete(q.selected_fields.as_ref()),
        WriteQuery::UpdateManyRecords(q) => get_result_node_for_update_many(q.selected_fields.as_ref()),
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
    nested_queries: Option<&Vec<ReadQuery>>,
) -> Option<ResultNode> {
    let field_map = field_selection
        .selections()
        .map(|fs| (fs.prisma_name(), fs))
        .collect::<HashMap<_, _>>();

    let mut node = ResultNode::new_object();
    for prisma_name in selection_order {
        match field_map.get(prisma_name.as_str()) {
            Some(sf @ SelectedField::Scalar(f)) => {
                let prisma_type = f.type_identifier_with_arity().0.to_prisma_type();
                node.add_field(
                    prisma_name,
                    ResultNode::new_value(sf.db_name().into_owned(), prisma_type),
                );
            }
            Some(SelectedField::Composite(_)) => todo!("MongoDB specific"),
            Some(SelectedField::Relation(f)) => {
                let nested_selection = FieldSelection::new(f.selections.to_vec());
                let nested_node = get_result_node(&nested_selection, &f.result_fields, None);
                if let Some(nested_node) = nested_node {
                    node.add_field(f.field.name(), nested_node);
                }
            }
            Some(SelectedField::Virtual(f)) => {
                let prisma_type = f.type_identifier_with_arity().0.to_prisma_type();
                let serialized_name = f.serialized_name();
                let child = node.entry(serialized_name.0).or_insert_with(ResultNode::new_object);
                child.add_field(serialized_name.1, ResultNode::new_value(f.db_alias(), prisma_type));
            }
            None => {}
        }
    }

    if let Some(nested_queries) = nested_queries {
        for nested_query in nested_queries {
            let nested_node = map_read_query(nested_query);
            if let Some(nested_node) = nested_node {
                let nested_query_name = nested_query.get_alias_or_name();
                node.add_field(nested_query_name, nested_node);
            }
        }
    }

    Some(node)
}

fn get_result_node_for_aggregation(
    selectors: &Vec<AggregationSelection>,
    selection_order: &Vec<(String, Option<Vec<String>>)>,
) -> Option<ResultNode> {
    let mut node = ResultNode::new_object();

    let mut selector_type_map = HashMap::<String, TypeIdentifier>::new();
    for selector in selectors {
        for identifier in selector.identifiers() {
            selector_type_map.insert(identifier.0, identifier.1);
        }
    }

    for (nested_name, field_names) in selection_order {
        if let Some(field_names) = field_names {
            let mut agg_node = ResultNode::new_object();

            for field_name in field_names {
                let result_type = if field_name == "_all" {
                    selector_type_map.get("all")
                } else {
                    selector_type_map.get(field_name)
                };

                if let Some(result_type) = result_type {
                    agg_node.add_field(
                        field_name,
                        ResultNode::Value {
                            db_name: field_name.into(),
                            result_type: result_type.to_prisma_type(),
                        },
                    );
                } else {
                    panic!("Unknown type for aggregate field: {field_name}");
                }
            }

            node.add_field(nested_name, agg_node);
        }
    }

    Some(node)
}

fn get_result_node_for_create_many(selected_fields: Option<&CreateManyRecordsFields>) -> Option<ResultNode> {
    get_result_node(
        &selected_fields?.fields,
        &selected_fields?.order,
        Some(&selected_fields?.nested),
    )
}

fn get_result_node_for_delete(selected_fields: Option<&DeleteRecordFields>) -> Option<ResultNode> {
    get_result_node(&selected_fields?.fields, &selected_fields?.order, None)
}

fn get_result_node_for_update_many(selected_fields: Option<&UpdateManyRecordsFields>) -> Option<ResultNode> {
    get_result_node(
        &selected_fields?.fields,
        &selected_fields?.order,
        Some(&selected_fields?.nested),
    )
}
