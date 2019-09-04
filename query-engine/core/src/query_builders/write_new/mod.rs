mod write_arguments;

pub use write_arguments::*;

use super::*;
use crate::{
    ArgumentListLookup, EdgeContent, EdgeOperation, Node, ParsedField, ParsedInputMap, ParsedInputValue, QueryGraph,
    ReadOneRecordBuilder,
};
use connector::{
    filter::{Filter, RecordFinder},
    CreateRecord, DeleteManyRecords, DeleteRecord, NestedWriteQueries, Query, ReadQuery, RootWriteQuery, UpdateRecord,
    WriteQuery,
};
use prisma_models::{ModelRef, RelationFieldRef};
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
    pub fn create_record(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let id_field = model.fields().id();

        let data_argument = field.arguments.lookup("data").unwrap();
        let data_map: ParsedInputMap = data_argument.value.try_into()?;
        let create_node = self.create_record_node(Arc::clone(&model), data_map)?;

        // Follow-up read query on the write
        let read_query = ReadOneRecordBuilder::new(field, model).build()?;
        let read_node = self.graph.create_node(Query::Read(read_query));

        self.graph.create_edge(
            &create_node,
            &read_node,
            EdgeContent::Read(EdgeOperation::DependentId(Box::new(|mut query, parent_id| {
                if let ReadQuery::RecordQuery(ref mut rq) = query {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id,
                    };

                    rq.record_finder = Some(finder);
                };

                query
            }))),
        );

        Ok(self)
    }

    /// Creates an update record query and adds it to the query graph, together with it's nested queries and companion read query.
    pub fn update_record(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
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

        self.graph.create_edge(
            &update_node,
            &read_node,
            EdgeContent::Read(EdgeOperation::DependentId(Box::new(|mut query, parent_id| {
                if let ReadQuery::RecordQuery(ref mut rq) = query {
                    let finder = RecordFinder {
                        field: id_field,
                        value: parent_id,
                    };

                    rq.record_finder = Some(finder);
                };

                query
            }))),
        );

        Ok(self)
    }

    /// Creates a delete record query and adds it to the query graph.
    pub fn delete_record(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let where_arg = field.arguments.lookup("where").unwrap();
        let record_finder = extract_record_finder(where_arg.value, &model)?;
        let delete = WriteQuery::Root(RootWriteQuery::DeleteRecord(DeleteRecord { where_: record_finder }));

        self.graph.create_node(Query::Write(delete));
        Ok(self)
    }

    /// Creates a delete many records query and adds it to the query graph.
    pub fn delete_many_records(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let filter = match field.arguments.lookup("where") {
            Some(where_arg) => extract_filter(where_arg.value.try_into()?, &model)?,
            None => Filter::empty(),
        };

        let delete_many = WriteQuery::Root(RootWriteQuery::DeleteManyRecords(DeleteManyRecords { model, filter }));

        self.graph.create_node(Query::Write(delete_many));
        Ok(self)
    }

    pub fn update_many_records(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        unimplemented!()
    }

    pub fn upsert_record(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        unimplemented!()
    }

    fn create_record_node(&self, model: ModelRef, data_map: ParsedInputMap) -> QueryBuilderResult<Node> {
        let create_args = WriteArguments::from(&model, data_map)?;
        let mut non_list_args = create_args.non_list;

        non_list_args.add_datetimes(Arc::clone(&model));

        let cr = CreateRecord {
            model,
            non_list_args,
            list_args: create_args.list,
            nested_writes: NestedWriteQueries::default(),
        };

        let node = self
            .graph
            .create_node(Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(Box::new(
                cr,
            )))));

        for (relation_field, data_map) in create_args.nested {
            self.connect_nested_query(&node, relation_field, data_map)?;
        }

        Ok(node)
    }

    fn update_record_node(
        &self,
        record_finder: Option<RecordFinder>,
        model: ModelRef,
        data_map: ParsedInputMap,
    ) -> QueryBuilderResult<Node> {
        let update_args = WriteArguments::from(&model, data_map)?;
        let list_causes_update = !update_args.list.is_empty();
        let mut non_list_args = update_args.non_list;

        non_list_args.update_datetimes(Arc::clone(&model), list_causes_update);

        let ur = UpdateRecord {
            where_: record_finder,
            non_list_args,
            list_args: update_args.list,
            nested_writes: NestedWriteQueries::default(),
        };

        let node = self
            .graph
            .create_node(Query::Write(WriteQuery::Root(RootWriteQuery::UpdateRecord(Box::new(
                ur,
            )))));

        for (relation_field, data_map) in update_args.nested {
            self.connect_nested_query(&node, relation_field, data_map)?;
        }

        Ok(node)
    }

    fn connect_nested_query(
        &self,
        parent: &Node,
        relation_field: RelationFieldRef,
        data_map: ParsedInputMap,
    ) -> QueryBuilderResult<()> {
        let model = relation_field.related_model();

        for (field_name, value) in data_map {
            match field_name.as_str() {
                "create" => self
                    .nested_create(value, &model)?
                    .into_iter()
                    .map(|node| {
                        let relation_field_name = relation_field.related_field().name.clone();

                        self.graph.create_edge(
                            parent,
                            &node,
                            EdgeContent::Write(
                                Arc::clone(&relation_field),
                                EdgeOperation::DependentId(Box::new(|mut query, parent_id| {
                                    query.inject_non_list_arg(relation_field_name, parent_id);
                                    query
                                })),
                            ),
                        );
                    })
                    .collect::<Vec<_>>(),

                "update" => self
                    .nested_update(value, &model, &relation_field)?
                    .into_iter()
                    .map(|node| {
                        let id_field = model.fields().id();
                        self.graph.create_edge(
                            parent,
                            &node,
                            EdgeContent::Write(
                                Arc::clone(&relation_field),
                                EdgeOperation::DependentId(Box::new(|mut query, parent_id| {
                                    if let WriteQuery::Root(RootWriteQuery::UpdateRecord(ref mut ur)) = query {
                                        ur.where_ = Some(RecordFinder {
                                            field: id_field,
                                            value: parent_id,
                                        });
                                    }

                                    query
                                })),
                            ),
                        );
                    })
                    .collect::<Vec<_>>(),

                _ => vec![],
            };
        }

        Ok(())
    }

    pub fn nested_create(&self, value: ParsedInputValue, model: &ModelRef) -> QueryBuilderResult<Vec<Node>> {
        Self::coerce_vec(value)
            .into_iter()
            .map(|value| self.create_record_node(Arc::clone(model), value.try_into()?))
            .collect::<QueryBuilderResult<Vec<_>>>()
    }

    pub fn nested_update(
        &self,
        value: ParsedInputValue,
        model: &ModelRef,
        relation_field: &RelationFieldRef,
    ) -> QueryBuilderResult<Vec<Node>> {
        Self::coerce_vec(value)
            .into_iter()
            .map(|value| {
                let (data_value, record_finder) = if relation_field.is_list {
                    let mut map: ParsedInputMap = value.try_into()?;
                    let where_arg = map.remove("where").unwrap();
                    let record_finder = utils::extract_record_finder(where_arg, &model)?;
                    let data_arg = map.remove("data").unwrap();

                    (data_arg, Some(record_finder))
                } else {
                    (value, None)
                };

                self.update_record_node(record_finder, Arc::clone(model), data_value.try_into()?)
            })
            .collect::<QueryBuilderResult<Vec<_>>>()
    }

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
