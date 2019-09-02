mod write_arguments;
mod nested;

pub use write_arguments::*;
pub use nested::*;

use super::*;
use crate::{ArgumentListLookup, ParsedField, ParsedInputMap, QueryGraph, Node};
use connector::{CreateRecord, NestedWriteQueries, Query, RootWriteQuery, WriteQuery};
use prisma_models::{ModelRef, RelationFieldRef};
use std::{convert::TryInto, sync::Arc};

pub fn create_record<'a>(
    model: ModelRef,
    mut field: ParsedField,
    graph: &mut QueryGraph,
    parent: Option<Node<'a>>,
) -> QueryBuilderResult<CreateRecord> {
    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;

    let create_args = WriteArguments::from(&model, data_map)?;
    let mut non_list_args = create_args.non_list;
    non_list_args.add_datetimes(Arc::clone(&model));

    let cr = CreateRecord {
        model,
        non_list_args,
        list_args: create_args.list,
        nested_writes: NestedWriteQueries::default(),
    };

    let node = graph.create_node(Query::Write(WriteQuery::Root(RootWriteQuery::CreateRecord(Box::new(cr)))));


    unimplemented!()
}
