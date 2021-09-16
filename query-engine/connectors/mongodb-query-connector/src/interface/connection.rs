use super::catch;
use crate::{
    error::MongoError,
    root_queries::{aggregate, read, write},
    MongoDbTransaction,
};
use async_trait::async_trait;
use connector_interface::{
    Connection, ConnectionLike, ReadOperations, RelAggregationSelection, Transaction, WriteArgs, WriteOperations,
};
use mongodb::{ClientSession, Database};
use prisma_models::prelude::*;

pub struct MongoDbConnection {
    /// The session to use for operations.
    pub(crate) session: ClientSession,

    /// Handle to a mongo database.
    pub(crate) database: Database,
}

impl ConnectionLike for MongoDbConnection {}

#[async_trait]
impl Connection for MongoDbConnection {
    async fn start_transaction<'a>(
        &'a mut self,
    ) -> connector_interface::Result<Box<dyn connector_interface::Transaction + 'a>> {
        let tx = Box::new(MongoDbTransaction::new(self).await?);

        Ok(tx as Box<dyn Transaction>)
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl WriteOperations for MongoDbConnection {
    async fn create_record(
        &mut self,
        model: &ModelRef,
        args: WriteArgs,
    ) -> connector_interface::Result<RecordProjection> {
        catch(async move { write::create_record(&self.database, &mut self.session, model, args).await }).await
    }

    async fn create_records(
        &mut self,
        model: &ModelRef,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
    ) -> connector_interface::Result<usize> {
        catch(
            async move { write::create_records(&self.database, &mut self.session, model, args, skip_duplicates).await },
        )
        .await
    }

    async fn update_records(
        &mut self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
        args: WriteArgs,
    ) -> connector_interface::Result<Vec<RecordProjection>> {
        catch(async move { write::update_records(&self.database, &mut self.session, model, record_filter, args).await })
            .await
    }

    async fn delete_records(
        &mut self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
    ) -> connector_interface::Result<usize> {
        catch(async move { write::delete_records(&self.database, &mut self.session, model, record_filter).await }).await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector_interface::Result<()> {
        catch(async move { write::m2m_connect(&self.database, &mut self.session, field, parent_id, child_ids).await })
            .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector_interface::Result<()> {
        catch(
            async move { write::m2m_disconnect(&self.database, &mut self.session, field, parent_id, child_ids).await },
        )
        .await
    }

    async fn execute_raw(
        &mut self,
        _query: String,
        _parameters: Vec<prisma_value::PrismaValue>,
    ) -> connector_interface::Result<usize> {
        Err(MongoError::Unsupported("Raw queries".to_owned()).into_connector_error())
    }

    async fn query_raw(
        &mut self,
        _query: String,
        _parameters: Vec<prisma_value::PrismaValue>,
    ) -> connector_interface::Result<serde_json::Value> {
        Err(MongoError::Unsupported("Raw queries".to_owned()).into_connector_error())
    }
}

#[async_trait]
impl ReadOperations for MongoDbConnection {
    async fn get_single_record(
        &mut self,
        model: &ModelRef,
        filter: &connector_interface::Filter,
        selected_fields: &ModelProjection,
        aggr_selections: &[RelAggregationSelection],
    ) -> connector_interface::Result<Option<SingleRecord>> {
        catch(async move {
            read::get_single_record(
                &self.database,
                &mut self.session,
                model,
                filter,
                selected_fields,
                aggr_selections,
            )
            .await
        })
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &ModelRef,
        query_arguments: connector_interface::QueryArguments,
        selected_fields: &ModelProjection,
        aggregation_selections: &[RelAggregationSelection],
    ) -> connector_interface::Result<ManyRecords> {
        catch(async move {
            read::get_many_records(
                &self.database,
                &mut self.session,
                model,
                query_arguments,
                selected_fields,
                aggregation_selections,
            )
            .await
        })
        .await
    }

    async fn get_related_m2m_record_ids(
        &mut self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> connector_interface::Result<Vec<(RecordProjection, RecordProjection)>> {
        catch(async move {
            read::get_related_m2m_record_ids(&self.database, &mut self.session, from_field, from_record_ids).await
        })
        .await
    }

    async fn aggregate_records(
        &mut self,
        model: &ModelRef,
        query_arguments: connector_interface::QueryArguments,
        selections: Vec<connector_interface::AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<connector_interface::Filter>,
    ) -> connector_interface::Result<Vec<connector_interface::AggregationRow>> {
        catch(async move {
            aggregate::aggregate(
                &self.database,
                &mut self.session,
                model,
                query_arguments,
                selections,
                group_by,
                having,
            )
            .await
        })
        .await
    }
}
