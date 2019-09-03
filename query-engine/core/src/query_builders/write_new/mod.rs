mod nested;
mod write_arguments;

pub use nested::*;
pub use write_arguments::*;

use super::*;
use crate::{ArgumentListLookup, EdgeContent, Node, ParsedField, ParsedInputMap, QueryGraph, ReadOneRecordBuilder};
use connector::{CreateRecord, NestedWriteQueries, Query, RootWriteQuery, WriteQuery};
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

    /// Creates a root record
    pub fn create_record_root<'a>(self, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<Self> {
        let data_argument = field.arguments.lookup("data").unwrap();
        let data_map: ParsedInputMap = data_argument.value.try_into()?;
        let create_node = self.create_record_node(Arc::clone(&model), data_map, None)?;
        let read_query = ReadOneRecordBuilder::new(field, model).build()?;
        let read_node = self.graph.create_node(Query::Read(read_query));

        self.graph.create_edge(&create_node, &read_node, EdgeContent::Read);
        Ok(self)
    }

    fn create_record_node<'a>(
        &'a self,
        model: ModelRef,
        data_map: ParsedInputMap,
        parent: Option<(Node<'a>, RelationFieldRef)>,
    ) -> QueryBuilderResult<Node<'a>> {
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

        if let Some((parent, relation_field)) = parent {
            self.graph
                .create_edge(&parent, &node, EdgeContent::Write(relation_field));
        };

        Ok(node)
    }

    fn connect_to_parent<'a>(node: Node<'a>, parent: Option<Node<'a>>, relation: RelationFieldRef) -> () {}
}

impl Into<QueryGraph> for WriteQueryBuilder {
    fn into(self) -> QueryGraph {
        self.graph
    }
}
