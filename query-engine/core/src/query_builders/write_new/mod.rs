mod write_arguments;

pub use write_arguments::*;

use super::*;
use crate::{
    query_ast::*,
    query_graph::{Flow, Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ParsedInputValue, ReadOneRecordBuilder,
};
use connector::{
    filter::{Filter, RecordFinder},
    QueryArguments,
};
use prisma_models::{ModelRef, RelationFieldRef, SelectedFields};
use std::{convert::TryInto, sync::Arc};

pub struct WriteQueryBuilder {
    graph: QueryGraph,
}

impl WriteQueryBuilder {
    pub fn new() -> Self {
        Self {
            graph: QueryGraph::new(),
        }
    }

    /// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
    pub fn create_record(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let id_field = model.fields().id();

        let data_argument = field.arguments.lookup("data").unwrap();
        let data_map: ParsedInputMap = data_argument.value.try_into()?;
        let create_node = self.create_record_node(Arc::clone(&model), data_map)?;

        // Follow-up read query on the write
        let read_query = ReadOneRecordBuilder::new(field, model).build()?;
        let read_node = self.graph.create_node(Query::Read(read_query));

        self.graph.add_result_node(&read_node);
        self.graph.create_edge(
            &create_node,
            &read_node,
            QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id.unwrap(),
                    };

                    rq.record_finder = Some(finder);
                };

                node
            })),
        );

        Ok(self)
    }

    /// Creates an update record query and adds it to the query graph, together with it's nested queries and companion read query.
    pub fn update_record(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let id_field = model.fields().id();

        // "where"
        let where_arg = field.arguments.lookup("where").unwrap();
        let record_finder = utils::extract_record_finder(where_arg.value, &model)?;

        // "data"
        let data_argument = field.arguments.lookup("data").unwrap();
        let data_map: ParsedInputMap = data_argument.value.try_into()?;

        let update_node = self.update_record_node(Some(record_finder), Arc::clone(&model), data_map)?;

        let read_query = ReadOneRecordBuilder::new(field, model).build()?;
        let read_node = self.graph.create_node(Query::Read(read_query));

        self.graph.add_result_node(&read_node);
        self.graph.create_edge(
            &update_node,
            &read_node,
            QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id.unwrap(),
                    };

                    rq.record_finder = Some(finder);
                };

                node
            })),
        );

        Ok(self)
    }

    /// Creates a delete record query and adds it to the query graph.
    pub fn delete_record(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let where_arg = field.arguments.lookup("where").unwrap();
        let record_finder = extract_record_finder(where_arg.value, &model)?;

        // Prefetch read query for the delete
        let mut read_query = ReadOneRecordBuilder::new(field, model).build()?;
        read_query.inject_record_finder(record_finder.clone());

        let read_node = self.graph.create_node(Query::Read(read_query));
        let delete_query = WriteQuery::DeleteRecord(DeleteRecord { where_: record_finder });
        let delete_node = self.graph.create_node(Query::Write(delete_query));

        self.graph.add_result_node(&read_node);
        self.graph
            .create_edge(&read_node, &delete_node, QueryGraphDependency::ExecutionOrder);

        Ok(self)
    }

    /// Creates a delete many records query and adds it to the query graph.
    pub fn delete_many_records(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let filter = match field.arguments.lookup("where") {
            Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
            None => Filter::empty(),
        };

        let delete_many = WriteQuery::DeleteManyRecords(DeleteManyRecords { model, filter });

        self.graph.create_node(Query::Write(delete_many));
        Ok(self)
    }

    /// Creates a create record query and adds it to the query graph.
    pub fn update_many_records(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let filter = match field.arguments.lookup("where") {
            Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
            None => Filter::empty(),
        };

        let data_argument = field.arguments.lookup("data").unwrap();
        let data_map: ParsedInputMap = data_argument.value.try_into()?;
        let update_args = WriteArguments::from(&model, data_map)?;

        let list_causes_update = !update_args.list.is_empty();
        let mut non_list_args = update_args.non_list;

        non_list_args.update_datetimes(Arc::clone(&model), list_causes_update);

        let update_many = WriteQuery::UpdateManyRecords(UpdateManyRecords {
            model,
            filter,
            non_list_args,
            list_args: update_args.list,
        });

        self.graph.create_node(Query::Write(update_many));
        Ok(self)
    }

    pub fn upsert_record(mut self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let where_arg = field.arguments.lookup("where").unwrap();
        let record_finder = utils::extract_record_finder(where_arg.value, &model)?;

        let create_argument = field.arguments.lookup("create").unwrap();
        let update_argument = field.arguments.lookup("update").unwrap();

        // Use ManyRecordsQuery to circumvent the connector blowing up on nonexistent record.
        let selected_fields: SelectedFields = model.fields().id().into();
        let initial_read_query = ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
            name: "".into(),
            alias: None,
            model: Arc::clone(&model),
            args: record_finder.clone().into(),
            selected_fields,
            nested: vec![],
            selection_order: vec![],
        });

        let initial_read_node = self.graph.create_node(Query::Read(initial_read_query));

        let create_node = self.create_record_node(Arc::clone(&model), create_argument.value.try_into()?)?;
        let update_node = self.update_record_node(
            Some(record_finder),
            Arc::clone(&model),
            update_argument.value.try_into()?,
        )?;

        let read_query = ReadOneRecordBuilder::new(field, Arc::clone(&model)).build()?;
        let read_node_create = self.graph.create_node(Query::Read(read_query.clone()));
        let read_node_update = self.graph.create_node(Query::Read(read_query));

        self.graph.add_result_node(&read_node_create);
        self.graph.add_result_node(&read_node_update);

        let if_node = self.graph.create_node(Flow::default_if());

        self.graph.create_edge(
            &initial_read_node,
            &if_node,
            QueryGraphDependency::ParentId(Box::new(|node, parent_id| {
                if let Node::Flow(Flow::If(_)) = node {
                    // Todo: This looks super unnecessary
                    Node::Flow(Flow::If(Box::new(move || parent_id.is_some())))
                } else {
                    node
                }
            })),
        );

        self.graph
            .create_edge(&if_node, &update_node, QueryGraphDependency::Then);

        self.graph
            .create_edge(&if_node, &create_node, QueryGraphDependency::Else);

        let id_field = model.fields().id();
        self.graph.create_edge(
            &update_node,
            &read_node_update,
            QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id.unwrap(),
                    };

                    rq.record_finder = Some(finder);
                };

                node
            })),
        );

        let id_field = model.fields().id();
        self.graph.create_edge(
            &create_node,
            &read_node_create,
            QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id.unwrap(),
                    };

                    rq.record_finder = Some(finder);
                };

                node
            })),
        );

        Ok(self)
    }

    fn create_record_node(&mut self, model: ModelRef, data_map: ParsedInputMap) -> QueryBuilderResult<NodeRef> {
        let create_args = WriteArguments::from(&model, data_map)?;
        let mut non_list_args = create_args.non_list;

        non_list_args.add_datetimes(Arc::clone(&model));

        let cr = CreateRecord {
            model,
            non_list_args,
            list_args: create_args.list,
        };

        let node = self.graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

        for (relation_field, data_map) in create_args.nested {
            self.connect_nested_query(&node, relation_field, data_map)?;
        }

        Ok(node)
    }

    fn update_record_node(
        &mut self,
        record_finder: Option<RecordFinder>,
        model: ModelRef,
        data_map: ParsedInputMap,
    ) -> QueryBuilderResult<NodeRef> {
        let update_args = WriteArguments::from(&model, data_map)?;
        let list_causes_update = !update_args.list.is_empty();
        let mut non_list_args = update_args.non_list;

        non_list_args.update_datetimes(Arc::clone(&model), list_causes_update);

        let ur = UpdateRecord {
            where_: record_finder,
            non_list_args,
            list_args: update_args.list,
        };

        let node = self.graph.create_node(Query::Write(WriteQuery::UpdateRecord(ur)));

        for (relation_field, data_map) in update_args.nested {
            self.connect_nested_query(&node, relation_field, data_map)?;
        }

        Ok(node)
    }

    fn connect_nested_query(
        &mut self,
        parent: &NodeRef,
        relation_field: RelationFieldRef,
        data_map: ParsedInputMap,
    ) -> QueryBuilderResult<()> {
        let model = relation_field.related_model();

        for (field_name, value) in data_map {
            match field_name.as_str() {
                "create" => self.connect_nested_create(parent, &relation_field, value, &model)?,
                "update" => self.connect_nested_update(parent, &relation_field, value, &model)?,

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
        &mut self,
        parent: &NodeRef,
        relation_field: &RelationFieldRef,
        value: ParsedInputValue,
        model: &ModelRef,
    ) -> QueryBuilderResult<()> {
        for value in Self::coerce_vec(value) {
            let nested_node = self.create_record_node(Arc::clone(model), value.try_into()?)?;
            let parent_query = self.graph.node_content(parent).unwrap();
            let relation_field_name = relation_field.related_field().name.clone();

            // Detect if a flip is necessary
            let (parent, child) = if let Node::Query(Query::Write(WriteQuery::CreateRecord(_))) = parent_query {
                if relation_field.relation_is_inlined_in_parent() {
                    // Actions required to do a flip:
                    // 1. Remove all edges from the parent to it's parents, and rewire them to the child.
                    // 2. Create an edge from child -> parent.
                    // Todo: Warning, this destroys the ordering of edges, which can lead to incorrect results being read.
                    //       Consider how the underlying graph can handle that.
                    let parent_edges = self.graph.incoming_edges(parent);
                    for parent_edge in parent_edges {
                        let parent_of_parent_node = self.graph.edge_source(&parent_edge);
                        let edge_content = self.graph.remove_edge(parent_edge).unwrap();

                        // Todo: Warning, this assumes the edge contents can also be "flipped".
                        self.graph
                            .create_edge(&parent_of_parent_node, &nested_node, edge_content);
                    }

                    (&nested_node, parent)
                } else {
                    (parent, &nested_node)
                }
            } else {
                (parent, &nested_node)
            };

            self.graph.create_edge(
                parent,
                child,
                QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
                    if let Node::Query(Query::Write(ref mut wq)) = node {
                        wq.inject_non_list_arg(relation_field_name, parent_id.unwrap());
                    }

                    node
                })),
            );

            // Detect if a connect is necessary?
            // ConnectRecords
        }

        Ok(())
    }

    pub fn connect_nested_update(
        &mut self,
        parent: &NodeRef,
        relation_field: &RelationFieldRef,
        value: ParsedInputValue,
        model: &ModelRef,
    ) -> QueryBuilderResult<()> {
        for value in Self::coerce_vec(value) {
            if relation_field.is_list {
                // We have a record specified as a record finder in "where"
                let mut map: ParsedInputMap = value.try_into()?;
                let where_arg = map.remove("where").unwrap();
                let record_finder = utils::extract_record_finder(where_arg, &model)?;
                let data_value = map.remove("data").unwrap();
                let update_node =
                    self.update_record_node(Some(record_finder), Arc::clone(model), data_value.try_into()?)?;

                self.graph
                    .create_edge(parent, &update_node, QueryGraphDependency::ExecutionOrder);
            } else {
                // We don't have a specific record (i.e. finder), we need to find it first.
                // Build a read query to load the necessary data first and connect it to the update.
                let read_parent_node =
                    self.graph
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

                let update_node = self.update_record_node(None, Arc::clone(model), value.try_into()?)?;
                let id_field = model.fields().id();

                self.graph.create_edge(
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

                self.graph.create_edge(
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

    // pub fn nested_delete(
    //     &mut self,
    //     value: ParsedInputValue,
    //     model: &ModelRef,
    //     relation_field: &RelationFieldRef,
    // ) -> QueryBuilderResult<Vec<Node>> {
    //     unimplemented!()
    // }

    fn coerce_vec(val: ParsedInputValue) -> Vec<ParsedInputValue> {
        match val {
            ParsedInputValue::List(l) => l,
            m @ ParsedInputValue::Map(_) => vec![m],
            single => vec![single],
        }
    }
}

impl Into<QueryGraph> for WriteQueryBuilder {
    fn into(self) -> QueryGraph {
        self.graph
    }
}
