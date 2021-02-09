use std::unimplemented;

use async_trait::async_trait;
use connector_interface::{Connection, ReadOperations, WriteOperations};
use mongodb::Database;
use prisma_models::prelude::*;

pub struct MongoDbConnection {
    /// Handle to a mongo database.
    pub(crate) database: Database,
}

#[async_trait]
impl Connection for MongoDbConnection {
    async fn start_transaction<'a>(
        &'a self,
    ) -> connector_interface::Result<Box<dyn connector_interface::Transaction + 'a>> {
        unimplemented!("Unsupported MongoDB feature: Transactions.");
    }
}

#[async_trait]
impl WriteOperations for MongoDbConnection {
    async fn create_record(
        &self,
        model: &ModelRef,
        args: connector_interface::WriteArgs,
    ) -> connector_interface::Result<RecordProjection> {
        todo!()
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
impl ReadOperations for MongoDbConnection {
    async fn get_single_record(
        &self,
        model: &ModelRef,
        filter: &connector_interface::Filter,
        selected_fields: &ModelProjection,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        todo!()
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
