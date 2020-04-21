mod dispatch;

pub use dispatch::*;

use crate::{Filter, QueryArguments, WriteArgs};
use prisma_models::*;
use prisma_value::PrismaValue;

pub trait Connector {
    fn get_connection<'a>(&'a self) -> crate::IO<Box<dyn Connection + 'a>>;
}

pub trait Connection: ReadOperations + WriteOperations + Send + Sync {
    fn start_transaction<'a>(&'a self) -> crate::IO<Box<dyn Transaction + 'a>>;
}

pub trait Transaction<'a>: ReadOperations + WriteOperations + Send + Sync {
    fn commit<'b>(&'b self) -> crate::IO<'b, ()>;
    fn rollback<'b>(&'b self) -> crate::IO<'b, ()>;
}

pub enum ConnectionLike<'conn, 'tx>
where
    'tx: 'conn,
{
    Connection(&'conn (dyn Connection + 'conn)),
    Transaction(&'conn (dyn Transaction<'tx> + 'tx)),
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

pub trait ReadOperations {
    /// Gets a single record or `None` back from the database.
    ///
    /// - The `ModelRef` represents the datamodel and its relations.
    /// - The `Filter` defines what item we want back and is guaranteed to be
    ///   defined to filter at most one item by the core.
    /// - The `SelectedFields` defines the values to be returned.
    fn get_single_record<'a>(
        &'a self,
        model: &'a ModelRef,
        filter: &'a Filter,
        selected_fields: &'a ModelProjection,
    ) -> crate::IO<'a, Option<SingleRecord>>;

    /// Gets multiple records from the database.
    ///
    /// - The `ModelRef` represents the datamodel and its relations.
    /// - The `QueryArguments` defines the filter and ordering of the returned
    ///   data, other parameters are currently not necessary due to windowing
    ///   handled in the core.
    /// - The `SelectedFields` defines the values to be returned.
    fn get_many_records<'a>(
        &'a self,
        model: &'a ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'a ModelProjection,
    ) -> crate::IO<'a, ManyRecords>;

    /// Retrieves pairs of IDs that belong together from a intermediate join
    /// table.
    ///
    /// Given the field from parent, and the projections, return the given
    /// projections with the corresponding child projections fetched from the
    /// database. The IDs returned will be used to perform a in-memory join
    /// between two datasets.
    fn get_related_m2m_record_ids<'a>(
        &'a self,
        from_field: &'a RelationFieldRef,
        from_record_ids: &'a [RecordProjection],
    ) -> crate::IO<'a, Vec<(RecordProjection, RecordProjection)>>;

    // return the number of items from the `Model`, filtered by the given `QueryArguments`.
    fn count_by_model<'a>(&'a self, model: &'a ModelRef, query_arguments: QueryArguments) -> crate::IO<'a, usize>;
}

pub trait WriteOperations {
    /// Insert a single record to the database.
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> crate::IO<RecordProjection>;

    /// Update records in the `Model` with the given `WriteArgs` filtered by the
    /// `Filter`.
    fn update_records<'a>(
        &'a self,
        model: &'a ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
    ) -> crate::IO<Vec<RecordProjection>>;

    /// Delete records in the `Model` with the given `Filter`.
    fn delete_records<'a>(&'a self, model: &'a ModelRef, record_filter: RecordFilter) -> crate::IO<usize>;

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    /// Connect the children to the parent.
    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordProjection,
        child_ids: &'a [RecordProjection],
    ) -> crate::IO<()>;

    /// Disconnect the children from the parent.
    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a RecordProjection,
        child_ids: &'a [RecordProjection],
    ) -> crate::IO<()>;

    /// Execute the raw query in the database as-is. The `parameters` are
    /// parameterized values for databases that support prepared statements.
    fn execute_raw<'a>(&'a self, query: String, parameters: Vec<PrismaValue>) -> crate::IO<serde_json::Value>;
}
