use query_structure::{FieldSelection, Model, PrismaValue, QueryArguments, WriteArgs};
use serde::Serialize;
mod query_arguments_ext;

pub use query_arguments_ext::QueryArgumentsExt;

pub trait QueryBuilder {
    fn build_get_records(
        &self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_create_record(
        &self,
        model: &Model,
        args: WriteArgs,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_inserts(
        &self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>>;
}

impl QueryBuilder for Box<dyn QueryBuilder> {
    fn build_get_records(
        &self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        self.as_ref().build_get_records(model, query_arguments, selected_fields)
    }

    fn build_create_record(
        &self,
        model: &Model,
        args: WriteArgs,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>> {
        self.as_ref().build_create_record(model, args, selected_fields)
    }

    fn build_inserts(
        &self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>> {
        self.as_ref()
            .build_inserts(model, args, skip_duplicates, selected_fields)
    }
}

#[derive(Debug, Serialize)]
pub struct DbQuery {
    pub query: String,
    pub params: Vec<PrismaValue>,
}

impl DbQuery {
    pub fn new(query: String, params: Vec<PrismaValue>) -> Self {
        Self { query, params }
    }
}
