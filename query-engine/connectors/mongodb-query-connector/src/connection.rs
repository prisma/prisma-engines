use connector_interface::filter::Filter;
use connector_interface::{self as connector, QueryArguments, ReadOperations, WriteArgs, WriteOperations};
use connector_interface::{RecordFilter, Transaction};

use prisma_models::prelude::*;
use async_trait::async_trait;

use std::sync::Arc;

/// A connection to a MongoDB database.
#[derive(Debug)]
pub struct Connection {
    client: mongodb::Client,
}

impl Connection {
    /// Create a new instance of `Connection`.
    pub(crate) fn new(client: mongodb::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl connector_interface::Connection for Connection {
    async fn start_transaction(&self) -> connector::Result<Box<dyn Transaction + '_>> {
        panic!("Mongodb transactions are not yet supported");
    }
}

#[async_trait]
impl ReadOperations for Connection {
    async fn get_single_record(
        &self,
        _model: &Arc<Model>,
        _filter: &Filter,
        _selected_fields: &ModelProjection,
    ) -> connector::Result<Option<SingleRecord>> {
            todo!();
    }

    async fn get_many_records(
        &self,
        _model: &ModelRef,
        _query_arguments: QueryArguments,
        _selected_fields: &ModelProjection,
    ) -> connector::Result<ManyRecords> {
            todo!();
    }

    async fn get_related_m2m_record_ids(
        &self,
        _from_field: &RelationFieldRef,
        _from_record_ids: &[RecordProjection],
    ) -> connector::Result<Vec<(RecordProjection, RecordProjection)>> {
            todo!();
    }

    async fn count_by_model(&self, _model: &ModelRef, _query_arguments: QueryArguments) -> connector::Result<usize> {
            todo!();
    }
}

#[async_trait]
impl WriteOperations for Connection {
    async fn create_record(&self, _model: &ModelRef, _args: WriteArgs) -> connector::Result<RecordProjection> {
        panic!("Write operations should be implemented on Transactions only");
    }

    async fn update_records(
        &self,
        _model: &ModelRef,
        _record_filter: RecordFilter,
        _args: WriteArgs,
    ) -> connector::Result<Vec<RecordProjection>> {
        panic!("Write operations should be implemented on Transactions only");
    }

    async fn delete_records(&self, _model: &ModelRef, _record_filter: RecordFilter) -> connector::Result<usize> {
        panic!("Write operations should be implemented on Transactions only");
    }

    async fn connect(
        &self,
        _field: &RelationFieldRef,
        _parent_id: &RecordProjection,
        _child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    async fn disconnect(
        &self,
        _field: &RelationFieldRef,
        _parent_id: &RecordProjection,
        _child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        panic!("Write operations should be implemented on Transactions only");
    }

    async fn execute_raw(&self, _query: String, _parameters: Vec<PrismaValue>) -> connector::Result<serde_json::Value> {
        panic!("Write operations should be implemented on Transactions only");
    }
}
