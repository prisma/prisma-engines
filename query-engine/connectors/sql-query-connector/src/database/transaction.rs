use crate::database::operations::*;
use crate::SqlError;
use async_trait::async_trait;
use connector_interface::{
    self as connector, filter::Filter, QueryArguments, ReadOperations, RecordFilter, Transaction, WriteArgs,
    WriteOperations,
};
use prisma_models::prelude::*;
use prisma_value::PrismaValue;
use quaint::prelude::ConnectionInfo;

pub struct SqlConnectorTransaction<'tx> {
    inner: quaint::connector::Transaction<'tx>,
    connection_info: ConnectionInfo,
}

impl<'tx> SqlConnectorTransaction<'tx> {
    pub fn new<'b: 'tx>(tx: quaint::connector::Transaction<'tx>, connection_info: &ConnectionInfo) -> Self {
        let connection_info = connection_info.clone();
        Self {
            inner: tx,
            connection_info,
        }
    }

    async fn catch<O>(
        &self,
        fut: impl std::future::Future<Output = Result<O, SqlError>>,
    ) -> Result<O, connector_interface::error::ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error(&self.connection_info)),
        }
    }
}

#[async_trait]
impl<'tx> Transaction for SqlConnectorTransaction<'tx> {
    async fn commit(&self) -> connector::Result<()> {
        self.catch(async move { Ok(self.inner.commit().await.map_err(SqlError::from)?) })
            .await
    }

    async fn rollback(&self) -> connector::Result<()> {
        self.catch(async move { Ok(self.inner.rollback().await.map_err(SqlError::from)?) })
            .await
    }
}

#[async_trait]
impl<'tx> ReadOperations for SqlConnectorTransaction<'tx> {
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
    ) -> connector::Result<ManyRecords> {
        self.catch(async move { read::get_many_records(&self.inner, model, query_arguments, selected_fields).await })
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

    async fn count_by_model(&self, model: &ModelRef, query_arguments: QueryArguments) -> connector::Result<usize> {
        self.catch(async move { read::count_by_model(&self.inner, model, query_arguments).await })
            .await
    }
}

#[async_trait]
impl<'tx> WriteOperations for SqlConnectorTransaction<'tx> {
    async fn create_record(&self, model: &ModelRef, args: WriteArgs) -> connector::Result<RecordProjection> {
        self.catch(async move { write::create_record(&self.inner, model, args).await })
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

    async fn connect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        self.catch(async move { write::connect(&self.inner, field, parent_id, child_ids).await })
            .await
    }

    async fn disconnect(
        &self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        self.catch(async move { write::disconnect(&self.inner, field, parent_id, child_ids).await })
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
