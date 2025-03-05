use query_structure::{
    AggregationSelection, FieldSelection, Filter, Model, PrismaValue, QueryArguments, RecordFilter, RelationField,
    ScalarCondition, ScalarField, SelectionResult, WriteArgs,
};
use serde::Serialize;
use std::fmt::Formatter;
use std::{collections::HashMap, fmt};

mod query_arguments_ext;

pub use query_arguments_ext::QueryArgumentsExt;
use query_template::{Fragment, PlaceholderFormat};

pub trait QueryBuilder {
    fn build_get_records(
        &self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    /// Retrieve related records through an M2M relation.
    #[cfg(feature = "relation_joins")]
    fn build_get_related_records(
        &self,
        link: RelationLink,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_aggregate(
        &self,
        model: &Model,
        args: QueryArguments,
        selections: &[AggregationSelection],
        group_by: Vec<ScalarField>,
        having: Option<Filter>,
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

    fn build_upsert(
        &self,
        model: &Model,
        filter: Filter,
        create_args: WriteArgs,
        update_args: WriteArgs,
        selected_fields: &FieldSelection,
        unique_constraints: &[ScalarField],
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_m2m_connect(
        &self,
        field: RelationField,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    fn build_m2m_disconnect(
        &self,
        field: RelationField,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

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

#[derive(Debug)]
pub struct RelationLink {
    field: RelationField,
    condition: Option<ScalarCondition>,
}

impl RelationLink {
    pub fn new(field: RelationField, condition: Option<ScalarCondition>) -> Self {
        Self { field, condition }
    }

    pub fn field(&self) -> &RelationField {
        &self.field
    }

    pub fn into_field_and_condition(self) -> (RelationField, Option<ScalarCondition>) {
        (self.field, self.condition)
    }
}

impl fmt::Display for RelationLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.field.relation().name(), self.field.model().name())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DbQuery {
    RawSql {
        sql: String,
        params: Vec<PrismaValue>,
    },
    TemplateSql {
        fragments: Vec<Fragment>,
        placeholder: PlaceholderFormat,
        params: Vec<PrismaValue>,
    },
}

impl DbQuery {
    pub fn params(&self) -> &Vec<PrismaValue> {
        match self {
            DbQuery::RawSql { params, .. } => params,
            DbQuery::TemplateSql { params, .. } => params,
        }
    }
}

impl fmt::Display for DbQuery {
    /// Should only be used for debugging, unit testing and playground CLI output.
    /// The placeholder syntax does not attempt to match any actual SQL flavour.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn format_fragments(f: &mut Formatter<'_>, fragments: &Vec<Fragment>) -> fmt::Result {
            let mut number = 1;
            for fragment in fragments {
                match fragment {
                    Fragment::StringChunk(s) => {
                        write!(f, "{}", s)?;
                    }
                    Fragment::Parameter => {
                        write!(f, "${number}")?;
                        number += 1;
                    }
                    Fragment::ParameterTuple => {
                        write!(f, "[${number}]")?;
                        number += 1;
                    }
                };
            }
            Ok(())
        }

        match self {
            DbQuery::RawSql { sql, .. } => write!(f, "{}", sql),
            DbQuery::TemplateSql { fragments, .. } => format_fragments(f, fragments),
        }
    }
}
