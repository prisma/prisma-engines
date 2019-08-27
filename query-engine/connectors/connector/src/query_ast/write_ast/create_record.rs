use super::*;
use prisma_models::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub model: ModelRef,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
    pub nested_writes: NestedWriteQueries,
}

#[derive(Debug, Clone)]
pub struct NestedCreateRecord {
    pub relation_field: Arc<RelationField>,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
    pub top_is_create: bool,
    pub nested_writes: NestedWriteQueries,
}

impl Into<RootWriteQuery> for NestedCreateRecord {
    fn into(self) -> RootWriteQuery {
        RootWriteQuery::CreateRecord(Box::new(CreateRecord {
            model: self.relation_field.model(),
            non_list_args: self.non_list_args,
            list_args: self.list_args,
            nested_writes: NestedWriteQueries::default(),
        }))
    }
}