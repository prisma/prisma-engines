use std::collections::HashMap;

use query_structure::{
    FieldSelection, Filter, Model, PrismaValue, QueryArguments, RecordFilter, RelationField, ScalarCondition, WriteArgs,
};
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

    /// Retrieve related records through an M2M relation.
    fn build_get_related_records(
        &self,
        field: RelationField,
        link_conditions: Vec<ScalarCondition>,
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

    fn build_update(
        &self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_updates_from_filter(
        &self,
        model: &Model,
        filter: Filter,
        args: WriteArgs,
        selected_fields: Option<&FieldSelection>,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>>;

    fn build_delete(
        &self,
        model: &Model,
        filter: RecordFilter,
        selected_fields: Option<&FieldSelection>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_deletes(
        &self,
        model: &Model,
        filter: RecordFilter,
        limit: Option<usize>,
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>>;

    fn build_raw(
        &self,
        model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        query_type: Option<String>,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;
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
