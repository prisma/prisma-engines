use query_structure::{
    AggregationSelection, FieldSelection, Filter, Model, Placeholder, PrismaValue, QueryArguments, RecordFilter,
    RelationField, RelationLoadStrategy, ScalarCondition, ScalarField, SelectedField, SelectionResult, WriteArgs,
};
use serde::{Serialize, Serializer, ser::SerializeSeq, ser::SerializeStruct, ser::SerializeTuple};
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
    ) -> Result<Vec<DbQuery>, Box<dyn std::error::Error + Send + Sync>>;

    /// Retrieve related records through an M2M relation.
    #[cfg(feature = "relation_joins")]
    fn build_get_related_records(
        &self,
        linkage: RelationLinkage,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
    ) -> Result<GetRelatedRecordsQuery, Box<dyn std::error::Error + Send + Sync>>;

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
        parent_field: RelationField,
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

/// A query that retrieves related records through an M2M relation.
#[cfg(feature = "relation_joins")]
pub struct GetRelatedRecordsQuery {
    /// The query that retrieves the related records.
    pub query: DbQuery,
    /// The alias used for the linking field in the query.
    pub linking_field_alias: String,
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

#[derive(Debug)]
pub enum DbQuery {
    RawSql {
        sql: String,
        args: Vec<PrismaValue>,
        arg_types: Vec<ArgType>,
    },
    TemplateSql {
        fragments: Vec<Fragment>,
        args: Vec<PrismaValue>,
        arg_types: Vec<DynamicArgType>,
        placeholder_format: PlaceholderFormat,
        chunkable: Chunkable,
    },
}

impl Serialize for DbQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::RawSql { sql, args, arg_types } => {
                let mut state = serializer.serialize_struct("DbQuery", 4)?;
                state.serialize_field("type", "rawSql")?;
                state.serialize_field("sql", sql)?;
                state.serialize_field("args", args)?;
                state.serialize_field("argTypes", arg_types)?;
                state.end()
            }
            Self::TemplateSql {
                fragments,
                args,
                arg_types,
                placeholder_format,
                chunkable,
            } => {
                let mut tuple = serializer.serialize_tuple(5)?;
                tuple.serialize_element(&SerializedFragments(fragments))?;
                tuple.serialize_element(&SerializedPlaceholderFormat(placeholder_format))?;
                tuple.serialize_element(args)?;
                tuple.serialize_element(arg_types)?;
                tuple.serialize_element(chunkable)?;
                tuple.end()
            }
        }
    }
}

impl DbQuery {
    pub fn params(&self) -> &[PrismaValue] {
        match self {
            DbQuery::RawSql { args: params, .. } => params,
            DbQuery::TemplateSql { args: params, .. } => params,
        }
    }
}

impl fmt::Display for DbQuery {
    /// Should only be used for debugging, unit testing and playground CLI output.
    /// The placeholder syntax does not attempt to match any actual SQL flavour.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DbQuery::RawSql { sql, .. } => {
                write!(formatter, "{sql}")?;
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
                        Fragment::ParameterTuple {
                            item_prefix,
                            item_separator,
                            item_suffix,
                        } => {
                            write!(formatter, "[{item_prefix}")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, "{item_suffix}{item_separator}*]")?;
                        }
                        Fragment::ParameterTupleList {
                            item_prefix,
                            item_separator,
                            item_suffix,
                            group_separator,
                        } => {
                            write!(formatter, "[{item_prefix}")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, "{item_suffix}{item_separator}*]{group_separator}*")?;
                        }
                    };
                }
            }
        }
        Ok(())
    }
}

struct SerializedFragments<'a>(&'a Vec<Fragment>);

impl Serialize for SerializedFragments<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for fragment in self.0 {
            match fragment {
                Fragment::StringChunk { chunk } => seq.serialize_element(chunk)?,
                Fragment::Parameter => seq.serialize_element(&ParameterFragment)?,
                Fragment::ParameterTuple {
                    item_prefix,
                    item_separator,
                    item_suffix,
                } => seq.serialize_element(&("T", item_prefix, item_separator, item_suffix))?,
                Fragment::ParameterTupleList {
                    item_prefix,
                    item_separator,
                    item_suffix,
                    group_separator,
                } => seq.serialize_element(&("L", item_prefix, item_separator, item_suffix, group_separator))?,
            }
        }
        seq.end()
    }
}

struct ParameterFragment;

impl Serialize for ParameterFragment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

struct SerializedPlaceholderFormat<'a>(&'a PlaceholderFormat);

impl Serialize for SerializedPlaceholderFormat<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.0.prefix)?;
        tuple.serialize_element(&self.0.has_numbering)?;
        tuple.end()
    }
}

#[derive(Debug)]
pub enum DynamicArgType {
    Tuple { elements: Vec<ArgType> },
    Single { r#type: ArgType },
}

impl Serialize for DynamicArgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Tuple { elements } => {
                let mut state = serializer.serialize_struct("DynamicArgType", 2)?;
                state.serialize_field("arity", "tuple")?;
                state.serialize_field("elements", elements)?;
                state.end()
            }
            Self::Single { r#type } => r#type.serialize(serializer),
        }
    }
}

#[derive(Debug)]
pub struct ArgType {
    pub arity: Arity,
    pub scalar_type: ArgScalarType,
    pub db_type: Option<String>,
}

impl Serialize for ArgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.arity == Arity::Scalar && self.db_type.is_none() {
            return serializer.serialize_str(self.scalar_type.as_str());
        }

        let mut state = serializer.serialize_struct("ArgType", 3)?;
        state.serialize_field("arity", &self.arity)?;
        state.serialize_field("scalarType", self.scalar_type.as_str())?;
        if let Some(db_type) = &self.db_type {
            state.serialize_field("dbType", db_type)?;
        }
        state.end()
    }
}

impl ArgType {
    pub fn new(arity: Arity, scalar_type: ArgScalarType, db_type: Option<String>) -> Self {
        Self {
            arity,
            scalar_type,
            db_type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Arity {
    Scalar,
    List,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArgScalarType {
    String,
    Int,
    #[serde(rename = "bigint")]
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum,
    Uuid,
    Json,
    #[serde(rename = "datetime")]
    DateTime,
    Bytes,
    Unknown,
}

impl ArgScalarType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::BigInt => "bigint",
            Self::Float => "float",
            Self::Decimal => "decimal",
            Self::Boolean => "boolean",
            Self::Enum => "enum",
            Self::Uuid => "uuid",
            Self::Json => "json",
            Self::DateTime => "datetime",
            Self::Bytes => "bytes",
            Self::Unknown => "unknown",
        }
    }
}

/// Indicates whether the parameters of this query can be chunked into smaller queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(into = "bool")]
pub enum Chunkable {
    Yes,
    No,
}

impl From<Chunkable> for bool {
    fn from(chunkable: Chunkable) -> Self {
        matches!(chunkable, Chunkable::Yes)
    }
}

impl From<&QueryArguments> for Chunkable {
    fn from(args: &QueryArguments) -> Self {
        if !args.order_by.is_empty()
            || args.cursor.is_some()
            || args.has_unbatchable_filters()
            || args.has_unbatchable_ordering()
        {
            Chunkable::No
        } else {
            Chunkable::Yes
        }
    }
}

impl From<&Filter> for Chunkable {
    fn from(filter: &Filter) -> Self {
        if filter.can_batch() {
            Chunkable::Yes
        } else {
            Chunkable::No
        }
    }
}
