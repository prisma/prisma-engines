use crate::{expression::EnumsMap, result_node::ResultNode};
use indexmap::IndexSet;
use itertools::Itertools;
use query_core::{
    CreateManyRecordsFields, DeleteRecordFields, Node, Query, QueryGraph, ReadQuery, UpdateManyRecordsFields,
    UpdateRecord, WriteQuery, schema::constants::aggregations,
};
use query_structure::{AggregationSelection, FieldSelection, SelectedField};
use std::collections::HashMap;

pub fn map_result_structure(graph: &QueryGraph, enums: &mut EnumsMap) -> Option<ResultNode> {
    graph
        .result_nodes()
        .chain(graph.leaf_nodes())
        .find_map(|idx| {
            if let Node::Query(query) = graph.node_content(&idx)? {
                map_query(query, enums)
            } else {
                None
            }
        })
        .or_else(|| {
            graph
                .result_nodes()
                .chain(graph.leaf_nodes())
                .all(|idx| match graph.node_content(&idx) {
                    Some(Node::Query(Query::Write(WriteQuery::QueryRaw(_) | WriteQuery::ExecuteRaw(_)))) => false,
                    Some(Node::Query(Query::Write(write))) => write.returns().is_none(),
                    _ => false,
                })
                .then_some(ResultNode::AffectedRows)
        })
}

fn map_query(query: &Query, enums: &mut EnumsMap) -> Option<ResultNode> {
    match query {
        Query::Read(read_query) => map_read_query(read_query, enums),
        Query::Write(write_query) => map_write_query(write_query, enums),
    }
}

fn map_read_query(query: &ReadQuery, enums: &mut EnumsMap) -> Option<ResultNode> {
    match query {
        ReadQuery::RecordQuery(q) => get_result_node(
            &q.selected_fields,
            &q.selection_order,
            &q.nested,
            q.relation_load_strategy.is_join(),
            enums,
        ),
        ReadQuery::ManyRecordsQuery(q) => get_result_node(
            &q.selected_fields,
            &q.selection_order,
            &q.nested,
            q.relation_load_strategy.is_join(),
            enums,
        ),
        ReadQuery::RelatedRecordsQuery(q) => {
            get_result_node(&q.selected_fields, &q.selection_order, &q.nested, false, enums)
        }
        ReadQuery::AggregateRecordsQuery(q) => get_result_node_for_aggregation(&q.selectors, &q.selection_order, enums),
    }
}

fn map_write_query(query: &WriteQuery, enums: &mut EnumsMap) -> Option<ResultNode> {
    match query {
        WriteQuery::CreateRecord(q) => get_result_node(&q.selected_fields, &q.selection_order, &[], false, enums),
        WriteQuery::CreateManyRecords(q) => get_result_node_for_create_many(q.selected_fields.as_ref(), enums),
        WriteQuery::UpdateRecord(u) => {
            match u {
                UpdateRecord::WithSelection(w) => {
                    get_result_node(&w.selected_fields, &w.selection_order, &[], false, enums)
                }
                UpdateRecord::WithoutSelection(_) => None, // No result data
            }
        }
        WriteQuery::DeleteRecord(q) => get_result_node_for_delete(q.selected_fields.as_ref(), enums),
        WriteQuery::UpdateManyRecords(q) => get_result_node_for_update_many(q.selected_fields.as_ref(), enums),
        WriteQuery::DeleteManyRecords(_) => None, // No result data
        WriteQuery::ConnectRecords(_) => None,    // No result data
        WriteQuery::DisconnectRecords(_) => None, // No result data
        WriteQuery::ExecuteRaw(_) => None,        // No data mapping
        WriteQuery::QueryRaw(_) => None,          // No data mapping
        WriteQuery::Upsert(q) => get_result_node(&q.selected_fields, &q.selection_order, &[], false, enums),
    }
}

fn get_result_node(
    field_selection: &FieldSelection,
    selection_order: &[String],
    nested_queries: &[ReadQuery],
    uses_relation_joins: bool,
    enums: &mut EnumsMap,
) -> Option<ResultNode> {
    let field_map = field_selection
        .selections()
        .map(|fs| (fs.prisma_name_grouping_virtuals(), fs))
        .collect::<HashMap<_, _>>();
    let grouped_virtuals = field_selection
        .virtuals()
        .into_group_map_by(|vs| vs.serialized_group_name());
    let nested_map = nested_queries
        .iter()
        .map(|q| (q.get_alias_or_name(), q))
        .collect::<HashMap<_, _>>();

    let mut node = ResultNode::new_object();
    for prisma_name in selection_order {
        match field_map.get(prisma_name.as_str()) {
            Some(sf @ SelectedField::Scalar(f)) => {
                enums.add(&f.r#type());
                node.add_field(
                    prisma_name,
                    ResultNode::new_value(sf.db_name().into_owned(), f.corresponding_prisma_type()),
                );
            }
            Some(SelectedField::Composite(_)) => todo!("MongoDB specific"),
            Some(SelectedField::Relation(f)) => {
                let nested_selection = FieldSelection::new(f.selections.to_vec());
                let nested_node = get_result_node(&nested_selection, &f.result_fields, &[], uses_relation_joins, enums);
                if let Some(nested_node) = nested_node {
                    node.add_field(f.field.name(), nested_node);
                }
            }
            Some(SelectedField::Virtual(f)) => {
                for vs in grouped_virtuals
                    .get(f.serialized_group_name())
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    let (group_name, field_name) = vs.serialized_name();
                    let db_name = if uses_relation_joins {
                        vs.serialized_field_name().to_owned()
                    } else {
                        vs.db_alias()
                    };

                    node.entry(group_name)
                        .or_insert_with(if uses_relation_joins {
                            ResultNode::new_object
                        } else {
                            ResultNode::new_flattened_object
                        })
                        .add_field(field_name, ResultNode::new_value(db_name, vs.r#type().to_prisma_type()));
                }
            }
            None => {
                if let Some(q) = nested_map.get(prisma_name.as_str()) {
                    let nested_node = map_read_query(q, enums);
                    if let Some(nested_node) = nested_node {
                        node.add_field(q.get_alias_or_name(), nested_node);
                    }
                }
            }
        }
    }

    Some(node)
}

fn get_result_node_for_aggregation(
    selectors: &[AggregationSelection],
    selection_order: &[(String, Option<Vec<String>>)],
    enums: &mut EnumsMap,
) -> Option<ResultNode> {
    let mut ordered_set = IndexSet::new();

    for (key, nested) in selection_order {
        if let Some(nested) = nested {
            for nested_key in nested {
                ordered_set.insert((Some(key.as_str()), nested_key.as_str()));
            }
        } else {
            ordered_set.insert((None, key.as_str()));
        }
    }

    let mut node = ResultNode::new_object();

    for (underscore_name, name, db_name, typ) in selectors
        .iter()
        .flat_map(|sel| {
            sel.identifiers().map(move |ident| {
                let (name, db_name) =
                    if matches!(&sel, AggregationSelection::Count { all: Some(_), .. }) && ident.name == "all" {
                        ("_all", "_all")
                    } else {
                        (ident.name, ident.db_name)
                    };
                (aggregate_underscore_name(sel), name, db_name, ident.typ)
            })
        })
        .sorted_by_key(|(underscore_name, name, _, _)| ordered_set.get_index_of(&(*underscore_name, *name)))
    {
        enums.add(&typ);
        let value = ResultNode::new_value(db_name.into(), typ.to_prisma_type());
        if let Some(undescore_name) = underscore_name {
            node.entry(undescore_name)
                .or_insert_with(ResultNode::new_object)
                .add_field(name, value);
        } else {
            node.add_field(name, value);
        }
    }

    Some(node)
}

fn aggregate_underscore_name(sel: &AggregationSelection) -> Option<&'static str> {
    match sel {
        AggregationSelection::Field(_) => None,
        AggregationSelection::Count { .. } => Some(aggregations::UNDERSCORE_COUNT),
        AggregationSelection::Average(_) => Some(aggregations::UNDERSCORE_AVG),
        AggregationSelection::Sum(_) => Some(aggregations::UNDERSCORE_SUM),
        AggregationSelection::Min(_) => Some(aggregations::UNDERSCORE_MIN),
        AggregationSelection::Max(_) => Some(aggregations::UNDERSCORE_MAX),
    }
}

fn get_result_node_for_create_many(
    selected_fields: Option<&CreateManyRecordsFields>,
    enums: &mut EnumsMap,
) -> Option<ResultNode> {
    get_result_node(
        &selected_fields?.fields,
        &selected_fields?.order,
        &selected_fields?.nested,
        false,
        enums,
    )
}

fn get_result_node_for_delete(
    selected_fields: Option<&DeleteRecordFields>,
    enums: &mut EnumsMap,
) -> Option<ResultNode> {
    get_result_node(&selected_fields?.fields, &selected_fields?.order, &[], false, enums)
}

fn get_result_node_for_update_many(
    selected_fields: Option<&UpdateManyRecordsFields>,
    enums: &mut EnumsMap,
) -> Option<ResultNode> {
    get_result_node(
        &selected_fields?.fields,
        &selected_fields?.order,
        &selected_fields?.nested,
        false,
        enums,
    )
}
