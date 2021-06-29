use crate::SqlError;
use crate::{database::operations::*, sql_info::SqlInfo};
use async_trait::async_trait;
use connector::{ConnectionLike, RelAggregationSelection};
use connector_interface::{
    self as connector, filter::Filter, AggregationRow, AggregationSelection, QueryArguments, ReadOperations,
    RecordFilter, Transaction, WriteArgs, WriteOperations,
};
use prisma_models::prelude::*;
use prisma_value::PrismaValue;
use quaint::prelude::ConnectionInfo;

use super::catch;

pub struct SqlConnectorTransaction<'tx> {
    inner: quaint::connector::Transaction<'tx>,
    connection_info: ConnectionInfo,
}

impl<'tx> SqlConnectorTransaction<'tx> {
    pub fn new(tx: quaint::connector::Transaction<'tx>, connection_info: &ConnectionInfo) -> Self {
        let connection_info = connection_info.clone();
        Self {
            inner: tx,
            connection_info,
        }
    }
}

impl<'tx> ConnectionLike for SqlConnectorTransaction<'tx> {}

#[async_trait]
impl<'tx> Transaction for SqlConnectorTransaction<'tx> {
    #[tracing::instrument(skip(self))]
    async fn commit(&mut self) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            Ok(self.inner.commit().await.map_err(SqlError::from)?)
        })
        .await
    }

    #[tracing::instrument(skip(self))]
    async fn rollback(&mut self) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            Ok(self.inner.rollback().await.map_err(SqlError::from)?)
        })
        .await
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl<'tx> ReadOperations for SqlConnectorTransaction<'tx> {
    async fn get_single_record(
        &mut self,
        model: &ModelRef,
        filter: &Filter,
        selected_fields: &ModelProjection,
        aggr_selections: &[RelAggregationSelection],
    ) -> connector::Result<Option<SingleRecord>> {
        catch(self.connection_info.clone(), async move {
            read::get_single_record(&self.inner, model, filter, selected_fields, aggr_selections).await
        })
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &ModelProjection,
        aggr_selections: &[RelAggregationSelection],
    ) -> connector::Result<ManyRecords> {
        catch(self.connection_info.clone(), async move {
            read::get_many_records(&self.inner, model, query_arguments, selected_fields, aggr_selections).await
        })
        .await
    }

    async fn get_related_m2m_record_ids(
        &mut self,
        from_field: &RelationFieldRef,
        from_record_ids: &[RecordProjection],
    ) -> connector::Result<Vec<(RecordProjection, RecordProjection)>> {
        catch(self.connection_info.clone(), async move {
            read::get_related_m2m_record_ids(&self.inner, from_field, from_record_ids).await
        })
        .await
    }

    async fn aggregate_records(
        &mut self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selections: Vec<AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<Filter>,
    ) -> connector::Result<Vec<AggregationRow>> {
        catch(self.connection_info.clone(), async move {
            read::aggregate(&self.inner, model, query_arguments, selections, group_by, having).await
        })
        .await
    }
}

#[async_trait]
impl<'tx> WriteOperations for SqlConnectorTransaction<'tx> {
    async fn create_record(&mut self, model: &ModelRef, args: WriteArgs) -> connector::Result<RecordProjection> {
        catch(self.connection_info.clone(), async move {
            write::create_record(&self.inner, model, args).await
        })
        .await
    }

    async fn create_records(
        &mut self,
        model: &ModelRef,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
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
        &mut self,
        model: &ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
    ) -> connector::Result<Vec<RecordProjection>> {
        catch(self.connection_info.clone(), async move {
            write::update_records(&self.inner, model, record_filter, args).await
        })
        .await
    }

    async fn delete_records(&mut self, model: &ModelRef, record_filter: RecordFilter) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::delete_records(&self.inner, model, record_filter).await
        })
        .await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            write::m2m_connect(&self.inner, field, parent_id, child_ids).await
        })
        .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &RecordProjection,
        child_ids: &[RecordProjection],
    ) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            write::m2m_disconnect(&self.inner, field, parent_id, child_ids).await
        })
        .await
    }

    async fn execute_raw(&mut self, query: String, parameters: Vec<PrismaValue>) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::execute_raw(&self.inner, query, parameters).await
        })
        .await
    }

    async fn query_raw(&mut self, query: String, parameters: Vec<PrismaValue>) -> connector::Result<serde_json::Value> {
        catch(self.connection_info.clone(), async move {
            write::query_raw(&self.inner, query, parameters).await
        })
        .await
    }
}
