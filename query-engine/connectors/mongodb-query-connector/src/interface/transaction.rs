use super::*;
use crate::{
    error::MongoError,
    queries::{read, write},
};
use connector_interface::{ReadOperations, Transaction, WriteOperations};
use futures::Future;
use mongodb::Database;

/// Not really a transaction right now, just something to
/// satisfy the core interface until we figure something out.
pub struct MongoDbTransaction {
    /// Handle to a mongo database.
    pub(crate) database: Database,
}

impl MongoDbTransaction {
    pub(crate) fn new(database: Database) -> Self {
        Self { database }
    }

    async fn catch<O>(
        &self,
        fut: impl Future<Output = Result<O, MongoError>>,
    ) -> Result<O, connector_interface::error::ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error()),
        }
    }
}

#[async_trait]
impl Transaction for MongoDbTransaction {
    async fn commit(&self) -> connector_interface::Result<()> {
        // Totally commited.
        Ok(())
    }

    async fn rollback(&self) -> connector_interface::Result<()> {
        // Totally rolled back.
        Ok(())
    }
}

#[async_trait]
impl WriteOperations for MongoDbTransaction {
    async fn create_record(
        &self,
        model: &ModelRef,
        args: connector_interface::WriteArgs,
    ) -> connector_interface::Result<RecordProjection> {
        self.catch(async move { write::create_record(&self.database, model, args).await })
            .await
    }

    async fn create_records(
        &self,
        model: &ModelRef,
        args: Vec<connector_interface::WriteArgs>,
        skip_duplicates: bool,
    ) -> connector_interface::Result<usize> {
        todo!()
    }

    async fn update_records(
        &self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
        args: connector_interface::WriteArgs,
    ) -> connector_interface::Result<Vec<RecordProjection>> {
        todo!()
    }

    async fn delete_records(
        &self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
    ) -> connector_interface::Result<usize> {
        todo!()
    }

    async fn connect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector_interface::Result<()> {
        todo!()
    }

    async fn disconnect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector_interface::Result<()> {
        todo!()
    }

    async fn execute_raw(
        &self,
        query: String,
        parameters: Vec<prisma_value::PrismaValue>,
    ) -> connector_interface::Result<usize> {
        todo!()
    }

    async fn query_raw(
        &self,
        query: String,
        parameters: Vec<prisma_value::PrismaValue>,
    ) -> connector_interface::Result<serde_json::Value> {
        todo!()
    }
}

#[async_trait]
impl ReadOperations for MongoDbTransaction {
    async fn get_single_record(
        &self,
        model: &ModelRef,
        filter: &connector_interface::Filter,
        selected_fields: &ModelProjection,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        self.catch(async move { read::get_single_record(&self.database, model, filter, selected_fields).await })
            .await
    }

    async fn get_many_records(
        &self,
        model: &ModelRef,
        query_arguments: connector_interface::QueryArguments,
        selected_fields: &ModelProjection,
    ) -> connector_interface::Result<ManyRecords> {
        todo!()
    }

    async fn get_related_m2m_record_ids(
        &self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> connector_interface::Result<Vec<(RecordProjection, RecordProjection)>> {
        todo!()
    }

    async fn aggregate_records(
        &self,
        model: &ModelRef,
        query_arguments: connector_interface::QueryArguments,
        selections: Vec<connector_interface::AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<connector_interface::Filter>,
    ) -> connector_interface::Result<Vec<connector_interface::AggregationRow>> {
        todo!()
    }
}
