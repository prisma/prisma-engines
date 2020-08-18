mod dispatch;
pub use dispatch::*;

use crate::{Filter, QueryArguments, WriteArgs};
use async_trait::async_trait;
use dml::FieldArity;
use prisma_models::*;
use prisma_value::PrismaValue;

#[async_trait]
pub trait Connector {
    /// Returns a connection to a data source.
    async fn get_connection(&self) -> crate::Result<Box<dyn Connection>>;

    /// Returns name of the connector.
    fn name(&self) -> String;
}

#[async_trait]
pub trait Connection: ReadOperations + WriteOperations + Send + Sync {
    async fn start_transaction<'a>(&'a self) -> crate::Result<Box<dyn Transaction + 'a>>;
}

#[async_trait]
pub trait Transaction: ReadOperations + WriteOperations + Send + Sync {
    async fn commit(&self) -> crate::Result<()>;
    async fn rollback(&self) -> crate::Result<()>;
}

pub enum ConnectionLike<'conn, 'tx>
where
    'tx: 'conn,
{
    Connection(&'conn (dyn Connection + 'conn)),
    Transaction(&'conn (dyn Transaction + 'tx)),
}

/// A wrapper struct allowing to either filter for records or for the core to
/// communicate already known record selectors to connectors.
///
/// Connector implementations should use known selectors to skip unnecessary fetch operations
/// if the query core already determined the selectors in a previous step. Simply put,
/// `selectors` should always have precendence over `filter`.
#[derive(Debug, Clone)]
pub struct RecordFilter {
    pub filter: Filter,
    pub selectors: Option<Vec<RecordProjection>>,
}

impl RecordFilter {
    pub fn empty() -> Self {
        Self {
            filter: Filter::empty(),
            selectors: None,
        }
    }
}

impl From<Filter> for RecordFilter {
    fn from(filter: Filter) -> Self {
        Self {
            filter,
            selectors: None,
        }
    }
}

impl From<Vec<RecordProjection>> for RecordFilter {
    fn from(selectors: Vec<RecordProjection>) -> Self {
        Self {
            filter: Filter::empty(),
            selectors: Some(selectors),
        }
    }
}

impl From<RecordProjection> for RecordFilter {
    fn from(selector: RecordProjection) -> Self {
        Self {
            filter: Filter::empty(),
            selectors: Some(vec![selector]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Aggregator {
    /// Counts all records of the model that match the query.
    Count,

    /// Compute average for each field contained.
    Average(Vec<ScalarFieldRef>),

    /// Compute sum for each field contained.
    Sum(Vec<ScalarFieldRef>),

    /// Compute mininum for each field contained.
    Min(Vec<ScalarFieldRef>),

    /// Compute maximum for each field contained.
    Max(Vec<ScalarFieldRef>),
}

impl Aggregator {
    pub fn identifiers(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        match self {
            Aggregator::Count => vec![(TypeIdentifier::Int, FieldArity::Required)],
            Aggregator::Average(fields) => Self::map_field_types(&fields, Some(TypeIdentifier::Float)),
            Aggregator::Sum(fields) => Self::map_field_types(&fields, None),
            Aggregator::Min(fields) => Self::map_field_types(&fields, None),
            Aggregator::Max(fields) => Self::map_field_types(&fields, None),
        }
    }

    fn map_field_types(
        fields: &[ScalarFieldRef],
        fixed_type: Option<TypeIdentifier>,
    ) -> Vec<(TypeIdentifier, FieldArity)> {
        fields
            .into_iter()
            .map(|f| {
                (
                    fixed_type.clone().unwrap_or(f.type_identifier.clone()),
                    FieldArity::Required,
                )
            })
            .collect()
    }
}

/// Result of an aggregation operation on a model or field.
/// It is expected that the type of a `PrismaValue` matches the `TypeIdentifier`
/// of the accompanying `ScalarFieldRef` for `Sum`, `Min` and `Max`.
/// `Count` and `Average` are expected to be of `int` and `float` types, respectively.
#[derive(Debug, Clone)]
pub enum AggregationResult {
    Count(PrismaValue),
    Average(ScalarFieldRef, PrismaValue),
    Sum(ScalarFieldRef, PrismaValue),
    Min(ScalarFieldRef, PrismaValue),
    Max(ScalarFieldRef, PrismaValue),
}

#[async_trait]
pub trait ReadOperations {
    /// Gets a single record or `None` back from the database.
    ///
    /// - The `ModelRef` represents the datamodel and its relations.
    /// - The `Filter` defines what item we want back and is guaranteed to be
    ///   defined to filter at most one item by the core.
    /// - The `SelectedFields` defines the values to be returned.
    async fn get_single_record(
        &self,
        model: &ModelRef,
        filter: &Filter,
        selected_fields: &ModelProjection,
    ) -> crate::Result<Option<SingleRecord>>;

    /// Gets multiple records from the database.
    ///
    /// - The `ModelRef` represents the datamodel and its relations.
    /// - The `QueryArguments` defines various constraints (see docs for detailed explanation).
    /// - The `SelectedFields` defines the fields (e.g. columns or document fields)
    ///   to be returned as a projection of fields of the model it queries.
    async fn get_many_records(
        &self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &ModelProjection,
    ) -> crate::Result<ManyRecords>;

    /// Retrieves pairs of IDs that belong together from a intermediate join
    /// table.
    ///
    /// Given the field from parent, and the projections, return the given
    /// projections with the corresponding child projections fetched from the
    /// database. The IDs returned will be used to perform a in-memory join
    /// between two datasets.
    async fn get_related_m2m_record_ids(
        &self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> crate::Result<Vec<(RecordProjection, RecordProjection)>>;

    /// Aggregates records for a specific model based on the given aggregators.
    /// Whether or not the aggregations can be executed in a single query or
    /// requires multiple roundtrips to the underlying data source is at the
    /// discretion of the implementing connector.
    async fn aggregate_records(
        &self,
        model: &ModelRef,
        aggregators: Vec<Aggregator>,
        query_arguments: QueryArguments,
    ) -> crate::Result<Vec<AggregationResult>>;
}

#[async_trait]
pub trait WriteOperations {
    /// Insert a single record to the database.
    async fn create_record(&self, model: &ModelRef, args: WriteArgs) -> crate::Result<RecordProjection>;

    /// Update records in the `Model` with the given `WriteArgs` filtered by the
    /// `Filter`.
    async fn update_records(
        &self,
        model: &ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
    ) -> crate::Result<Vec<RecordProjection>>;

    /// Delete records in the `Model` with the given `Filter`.
    async fn delete_records(&self, model: &ModelRef, record_filter: RecordFilter) -> crate::Result<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    /// Connect the children to the parent.
    async fn connect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> crate::Result<()>;

    /// Disconnect the children from the parent.
    async fn disconnect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> crate::Result<()>;

    /// Execute the raw query in the database as-is. The `parameters` are
    /// parameterized values for databases that support prepared statements.
    ///
    /// Returns the number of rows affected.
    async fn execute_raw(&self, query: String, parameters: Vec<PrismaValue>) -> crate::Result<usize>;

    /// Execute the raw query in the database as-is. The `parameters` are
    /// parameterized values for databases that support prepared statements.
    ///
    /// Returns resulting rows as JSON.
    async fn query_raw(&self, query: String, parameters: Vec<PrismaValue>) -> crate::Result<serde_json::Value>;
}
