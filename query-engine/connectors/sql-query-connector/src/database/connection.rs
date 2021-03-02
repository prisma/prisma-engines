use super::transaction::SqlConnectorTransaction;
use crate::{database::operations::*, sql_info::SqlInfo, QueryExt, SqlError};
use async_trait::async_trait;
use connector::RelAggregationSelection;
use connector_interface::{
    self as connector, filter::Filter, AggregationRow, AggregationSelection, Connection, QueryArguments,
    ReadOperations, RecordFilter, Transaction, WriteArgs, WriteOperations,
};
use prisma_models::prelude::*;
use prisma_value::PrismaValue;
use quaint::{connector::TransactionCapable, prelude::ConnectionInfo};
use std::future::Future;

pub struct SqlConnection<C> {
    inner: C,
    connection_info: ConnectionInfo,
}

impl<C> SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    pub fn new(inner: C, connection_info: &ConnectionInfo) -> Self {
        let connection_info = connection_info.clone();
        Self { inner, connection_info }
    }

    async fn catch<O>(
        &self,
        fut: impl Future<Output = Result<O, SqlError>>,
    ) -> Result<O, connector_interface::error::ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error(&self.connection_info)),
        }
    }
}

#[async_trait]
impl<C> Connection for SqlConnection<C>
where
    C: QueryExt + TransactionCapable + Send + Sync + 'static,
{
    #[tracing::instrument(skip(self))]
    async fn start_transaction<'a>(&'a self) -> connector::Result<Box<dyn Transaction + 'a>> {
        let fut_tx = self.inner.start_transaction();
        let connection_info = &self.connection_info;

        self.catch(async move {
            let tx: quaint::connector::Transaction = fut_tx.await.map_err(SqlError::from)?;
            Ok(Box::new(SqlConnectorTransaction::new(tx, &connection_info)) as Box<dyn Transaction>)
        })
        .await
    }
}

#[async_trait]
impl<C> ReadOperations for SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    async fn get_single_record(
        &self,
        model: &ModelRef,
        filter: &Filter,
        selected_fields: &ModelProjection,
    ) -> connector::Result<Option<SingleRecord>> {
        self.catch(async move { read::get_single_record(&self.inner, model, filter, selected_fields).await })
            .await
    }

    async fn get_many_records(
        &self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &ModelProjection,
        aggr_selections: &[RelAggregationSelection],
    ) -> connector::Result<ManyRecords> {
        self.catch(async move {
            read::get_many_records(&self.inner, model, query_arguments, selected_fields, aggr_selections).await
        })
        .await
    }

    async fn get_related_m2m_record_ids(
        &self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> connector::Result<Vec<(RecordProjection, RecordProjection)>> {
        self.catch(async move { read::get_related_m2m_record_ids(&self.inner, from_field, from_record_ids).await })
            .await
    }

    async fn aggregate_records(
        &self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selections: Vec<AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<Filter>,
    ) -> connector::Result<Vec<AggregationRow>> {
        self.catch(
            async move { read::aggregate(&self.inner, model, query_arguments, selections, group_by, having).await },
        )
        .await
    }
}

#[async_trait]
impl<C> WriteOperations for SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    async fn create_record(&self, model: &ModelRef, args: WriteArgs) -> connector::Result<RecordProjection> {
        self.catch(async move { write::create_record(&self.inner, model, args).await })
            .await
    }

    async fn create_records(
        &self,
        model: &ModelRef,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
    ) -> connector::Result<usize> {
        self.catch(async move {
            write::create_records(
                &self.inner,
                SqlInfo::from(&self.connection_info),
                model,
                args,
                skip_duplicates,
            )
            .await
        })
        .await
    }

    async fn update_records(
        &self,
        model: &ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
    ) -> connector::Result<Vec<RecordProjection>> {
        self.catch(async move { write::update_records(&self.inner, model, record_filter, args).await })
            .await
    }

    async fn delete_records(&self, model: &ModelRef, record_filter: RecordFilter) -> connector::Result<usize> {
        self.catch(async move { write::delete_records(&self.inner, model, record_filter).await })
            .await
    }

    async fn m2m_connect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        self.catch(async move { write::m2m_connect(&self.inner, field, parent_id, child_ids).await })
            .await
    }

    async fn m2m_disconnect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        self.catch(async move { write::m2m_disconnect(&self.inner, field, parent_id, child_ids).await })
            .await
    }

    async fn execute_raw(&self, query: String, parameters: Vec<PrismaValue>) -> connector::Result<usize> {
        self.catch(async move { write::execute_raw(&self.inner, query, parameters).await })
            .await
    }

    async fn query_raw(&self, query: String, parameters: Vec<PrismaValue>) -> connector::Result<serde_json::Value> {
        self.catch(async move { write::query_raw(&self.inner, query, parameters).await })
            .await
    }
}
