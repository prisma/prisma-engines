use super::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ParsedInputValue, ParsedInputMap,
};
use connector::{
    QueryArguments,
    filter::{RecordFinder},
};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

pub fn connect_nested_query(
        graph: &mut QueryGraph,
        parent: &NodeRef,
        relation_field: RelationFieldRef,
        data_map: ParsedInputMap,
    ) -> QueryBuilderResult<()> {
        let model = relation_field.related_model();

        for (field_name, value) in data_map {
            match field_name.as_str() {
                "create" => connect_nested_create(graph, parent, &relation_field, value, &model)?,
                "update" => connect_nested_update(graph, parent, &relation_field, value, &model)?,
                "connect" => connect_nested_connect(graph, parent, &relation_field, value, &model)?,
                // "delete" => self
                //     .nested_delete(value, &model, &relation_field)?
                //     .into_iter()
                //     .map(|node| {
                //         //
                //         unimplemented!()
                //     })
                //     .collect::<Vec<_>>(),
                _ => (),
            };
        }

        Ok(())
    }

    pub fn connect_nested_create(
        graph: &mut QueryGraph,
        parent: &NodeRef,
        relation_field: &RelationFieldRef,
        value: ParsedInputValue,
        model: &ModelRef,
    ) -> QueryBuilderResult<()> {
        for value in utils::coerce_vec(value) {
            let child = create::create_record_node(graph, Arc::clone(model), value.try_into()?)?;
            let relation_field_name = relation_field.name.clone();
            let (parent, child) = utils::flip_nodes(graph, parent, &child, relation_field);

            graph.create_edge(
                parent,
                child,
                QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                    // The following injection is necessary for cases where the relation is inlined.
                    // The injection won't do anything in other cases.
                    // The other case where the relation is not inlined is handled further down.
                    if let Node::Query(Query::Write(ref mut wq)) = node {
                        wq.inject_non_list_arg(relation_field_name, parent_id.unwrap());
                    }

                    node
                })),
            );

            // Detect if a connect is necessary between the nodes.
            // A connect is necessary if the nested create is done on a relation that
            // is a list (x-to-many) and where the relation is not inlined (aka manifested as an
            // actual join table, for example).
            if relation_field.is_list && !relation_field.relation().is_inline_relation() {
                connect::connect_records_node(graph, parent, child, relation_field);
            }
        }

        Ok(())
    }

    pub fn connect_nested_update(
        graph: &mut QueryGraph,
        parent: &NodeRef,
        relation_field: &RelationFieldRef,
        value: ParsedInputValue,
        model: &ModelRef,
    ) -> QueryBuilderResult<()> {
        for value in utils::coerce_vec(value) {
            if relation_field.is_list {
                // We have a record specified as a record finder in "where"
                let mut map: ParsedInputMap = value.try_into()?;
                let where_arg = map.remove("where").unwrap();
                let record_finder = extract_record_finder(where_arg, &model)?;
                let data_value = map.remove("data").unwrap();
                let update_node =
                    update::update_record_node(graph, Some(record_finder), Arc::clone(model), data_value.try_into()?)?;

                graph
                    .create_edge(parent, &update_node, QueryGraphDependency::ExecutionOrder);
            } else {
                // We don't have a specific record (i.e. finder), we need to find it first.
                // Build a read query to load the necessary data first and connect it to the update.
                let read_parent_node =
                    graph
                        .create_node(Query::Read(ReadQuery::RelatedRecordsQuery(RelatedRecordsQuery {
                            name: "parent".to_owned(),
                            alias: None,
                            parent_field: Arc::clone(relation_field),
                            parent_ids: None,
                            args: QueryArguments::default(),
                            selected_fields: relation_field.related_field().model().fields().id().into(),
                            nested: vec![],
                            selection_order: vec![],
                        })));

                let update_node = update::update_record_node(graph, None, Arc::clone(model), value.try_into()?)?;
                let id_field = model.fields().id();

                graph.create_edge(
                    &read_parent_node,
                    &update_node,
                    QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                        if let Node::Query(Query::Write(WriteQuery::UpdateRecord(ref mut ur))) = node {
                            ur.where_ = Some(RecordFinder {
                                field: id_field,
                                value: parent_id.unwrap(),
                            });
                        }

                        node
                    })),
                );

                graph.create_edge(
                    parent,
                    &read_parent_node,
                    QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                        if let Node::Query(Query::Read(ReadQuery::RelatedRecordsQuery(ref mut rq))) = node {
                            rq.parent_ids = Some(vec![parent_id.unwrap().try_into().unwrap()]);
                        };

                        node
                    })),
                );
            }
        }

        Ok(())
    }

    pub fn connect_nested_connect(
        graph: &mut QueryGraph,
        parent: &NodeRef,
        relation_field: &RelationFieldRef,
        value: ParsedInputValue,
        model: &ModelRef,
    ) -> QueryBuilderResult<()> {
        for value in utils::coerce_vec(value) {
            // First, we need to build a read query on the record to be conneted.
            let record_finder = extract_record_finder(value, &model)?;
            let child_read_query = utils::id_read_query_infallible(&model, record_finder);
            let child_node = graph.create_node(child_read_query);

            // Flip the read node and parent node if necessary.
            let (parent, child) = utils::flip_nodes(graph, parent, &child_node, relation_field);
            let relation_field_name = relation_field.name.clone();

            // Edge from parent to child (e.g. Create to ReadQuery).
            graph.create_edge(
                parent,
                child,
                QueryGraphDependency::ParentId(Box::new(|mut child_node, parent_id| {
                    // If the child is a write query, inject the parent id.
                    // This covers cases of inlined relations.
                    if let Node::Query(Query::Write(ref mut wq)) = child_node {
                        wq.inject_non_list_arg(relation_field_name, parent_id.unwrap())
                    }

                    child_node
                })),
            );

            // Detect if a connect is actually necessary between the nodes.
            // A connect is necessary if the nested connect is done on a relation that
            // is a list (x-to-many) and where the relation is not inlined (aka manifested as an
            // actual join table, for example).
            if relation_field.is_list && !relation_field.relation().is_inline_relation() {
                connect::connect_records_node(graph, parent, child, relation_field);
            }
        }

        Ok(())
    }