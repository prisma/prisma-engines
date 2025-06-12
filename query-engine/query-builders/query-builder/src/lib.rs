use query_structure::{
    AggregationSelection, FieldSelection, Filter, Model, Placeholder, PrismaValue, QueryArguments, RecordFilter,
    RelationField, RelationLoadStrategy, ScalarCondition, ScalarField, SelectedField, SelectionResult,
    TaggedPrismaValue, WriteArgs,
};
use serde::Serialize;
use std::collections::BTreeMap;
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
        relation_load_strategy: RelationLoadStrategy,
    ) -> Result<DbQuery, Box<dyn std::error::Error + Send + Sync>>;

    /// Retrieve related records through an M2M relation.
    #[cfg(feature = "relation_joins")]
    fn build_get_related_records(
        &self,
        linkage: RelationLinkage,
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
    ) -> Result<CreateRecord, Box<dyn std::error::Error + Send + Sync>>;

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

    fn build_updates(
        &self,
        model: &Model,
        record_filter: RecordFilter,
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
        parent: PrismaValue,
        child: PrismaValue,
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

/// An insertion operation for a record in the database.
pub struct CreateRecord {
    /// The insert query to run in order to create the record.
    pub insert_query: DbQuery,
    /// The query to run prior to the insert in order to create default column values.
    /// This is used in some cases where the database does not support returning default values.
    pub select_defaults: Option<CreateRecordDefaultsQuery>,
    /// The field in the model of the record that corresponds to the last inserted ID, if
    /// required by the database.
    pub last_insert_id_field: Option<ScalarField>,
    /// The values to merge into the resulting record after insertion. These are inferred from the
    /// input arguments.
    pub merge_values: Vec<(SelectedField, PrismaValue)>,
}

/// A query that retrieves default values needed for an insert operation.
pub struct CreateRecordDefaultsQuery {
    /// The query that returns the default values.
    pub query: DbQuery,
    /// The fields that are selected in the query and their corresponding placeholders.
    /// These placeholders are referred to by the subsequent insert query.
    pub field_placeholders: Vec<(ScalarField, Placeholder)>,
}

#[derive(Debug)]
pub struct ConditionalLink {
    field: ScalarField,
    conditions: Vec<ScalarCondition>,
}

impl ConditionalLink {
    pub fn new(field: ScalarField, conditions: Vec<ScalarCondition>) -> Self {
        Self { field, conditions }
    }

    pub fn field(&self) -> &ScalarField {
        &self.field
    }

    pub fn into_field_and_conditions(self) -> (ScalarField, Vec<ScalarCondition>) {
        (self.field, self.conditions)
    }
}

#[derive(Debug)]
pub struct RelationLinkage {
    parent_field: RelationField,
    conditions: BTreeMap<ScalarField, Vec<ScalarCondition>>,
}

impl RelationLinkage {
    pub fn new(field: RelationField, links: Vec<ConditionalLink>) -> Self {
        Self {
            parent_field: field,
            conditions: links
                .into_iter()
                .map(ConditionalLink::into_field_and_conditions)
                .collect(),
        }
    }

    pub fn parent_field(&self) -> &RelationField {
        &self.parent_field
    }

    pub fn add_condition(&mut self, field: ScalarField, condition: ScalarCondition) {
        self.conditions.entry(field).or_default().push(condition);
    }

    pub fn into_parent_field_and_conditions(
        self,
    ) -> (
        RelationField,
        impl Iterator<Item = (ScalarField, Vec<ScalarCondition>)> + fmt::Debug,
    ) {
        (self.parent_field, self.conditions.into_iter())
    }
}

impl fmt::Display for RelationLinkage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}",
            self.parent_field.relation().name(),
            self.parent_field.model().name()
        )
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DbQuery {
    #[serde(rename_all = "camelCase")]
    RawSql {
        sql: String,
        #[serde(serialize_with = "serialize_params")]
        params: Vec<PrismaValue>,
    },
    #[serde(rename_all = "camelCase")]
    TemplateSql {
        fragments: Vec<Fragment>,
        #[serde(serialize_with = "serialize_params")]
        params: Vec<PrismaValue>,
        placeholder_format: PlaceholderFormat,
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DbQuery::RawSql { sql, .. } => {
                write!(formatter, "{}", sql)?;
            }
            DbQuery::TemplateSql { fragments, .. } => {
                let placeholder_format = PlaceholderFormat {
                    prefix: "$",
                    has_numbering: true,
                };
                let mut number = 1;
                for fragment in fragments {
                    match fragment {
                        Fragment::StringChunk { chunk } => {
                            write!(formatter, "{chunk}")?;
                        }
                        Fragment::Parameter => {
                            placeholder_format.write(formatter, &mut number)?;
                        }
                        Fragment::ParameterTuple => {
                            write!(formatter, "[")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, "]")?;
                        }
                        Fragment::ParameterTupleList { .. } => {
                            write!(formatter, "[(")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, ")]")?;
                        }
                    };
                }
            }
        }
        Ok(())
    }
}

fn serialize_params<S>(obj: &[PrismaValue], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_seq(obj.iter().map(TaggedPrismaValue::from))
}
