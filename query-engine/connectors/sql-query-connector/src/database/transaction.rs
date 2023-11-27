use super::catch;
use crate::{database::operations::*, Context, SqlError};
use async_trait::async_trait;
use connector::{ConnectionLike, RelAggregationSelection};
use connector_interface::{
    self as connector, AggregationRow, AggregationSelection, ReadOperations, RecordFilter, Transaction, WriteArgs,
    WriteOperations,
};
use prisma_value::PrismaValue;
use quaint::prelude::ConnectionInfo;
use query_structure::{prelude::*, Filter, QueryArguments, SelectionResult};
use std::collections::HashMap;

pub struct SqlConnectorTransaction<'tx> {
    inner: Box<dyn quaint::connector::Transaction + 'tx>,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl<'tx> SqlConnectorTransaction<'tx> {
    pub fn new(
        tx: Box<dyn quaint::connector::Transaction + 'tx>,
        connection_info: &ConnectionInfo,
        features: psl::PreviewFeatures,
    ) -> Self {
        let connection_info = connection_info.clone();

        Self {
            inner: tx,
            connection_info,
            features,
        }
    }
}

impl<'tx> ConnectionLike for SqlConnectorTransaction<'tx> {}

#[async_trait]
impl<'tx> Transaction for SqlConnectorTransaction<'tx> {
    async fn commit(&mut self) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            self.inner.commit().await.map_err(SqlError::from)
        })
        .await
    }

    async fn rollback(&mut self) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            let res = self.inner.rollback().await.map_err(SqlError::from);

            match res {
                Err(SqlError::TransactionAlreadyClosed(_)) | Err(SqlError::RollbackWithoutBegin) => Ok(()),
                _ => res,
            }
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
        model: &Model,
        filter: &Filter,
        selected_fields: &FieldSelection,
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> connector::Result<Option<SingleRecord>> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            read::get_single_record(
                self.inner.as_queryable(),
                model,
                filter,
                &selected_fields.into(),
                aggr_selections,
                &ctx,
            )
            .await
        })
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> connector::Result<ManyRecords> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            read::get_many_records(
                self.inner.as_queryable(),
                model,
                query_arguments,
                &selected_fields.into(),
                aggr_selections,
                &ctx,
            )
            .await
        })
        .await
    }

    async fn get_related_m2m_record_ids(
        &mut self,
        from_field: &RelationFieldRef,
        from_record_ids: &[SelectionResult],
        trace_id: Option<String>,
    ) -> connector::Result<Vec<(SelectionResult, SelectionResult)>> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            read::get_related_m2m_record_ids(self.inner.as_queryable(), from_field, from_record_ids, &ctx).await
        })
        .await
    }

    async fn aggregate_records(
        &mut self,
        model: &Model,
        query_arguments: QueryArguments,
        selections: Vec<AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<Filter>,
        trace_id: Option<String>,
    ) -> connector::Result<Vec<AggregationRow>> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            read::aggregate(
                self.inner.as_queryable(),
                model,
                query_arguments,
                selections,
                group_by,
                having,
                &ctx,
            )
            .await
        })
        .await
    }
}

#[async_trait]
impl<'tx> WriteOperations for SqlConnectorTransaction<'tx> {
    async fn create_record(
        &mut self,
        model: &Model,
        args: WriteArgs,
        selected_fields: FieldSelection,
        trace_id: Option<String>,
    ) -> connector::Result<SingleRecord> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::create_record(
                self.inner.as_queryable(),
                &self.connection_info.sql_family(),
                model,
                args,
                selected_fields,
                &ctx,
            )
            .await
        })
        .await
    }

    async fn create_records(
        &mut self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::create_records(self.inner.as_queryable(), model, args, skip_duplicates, &ctx).await
        })
        .await
    }

    async fn update_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::update_records(self.inner.as_queryable(), model, record_filter, args, &ctx).await
        })
        .await
    }

    async fn update_record(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<FieldSelection>,
        trace_id: Option<String>,
    ) -> connector::Result<Option<SingleRecord>> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());

            write::update_record(
                self.inner.as_queryable(),
                model,
                record_filter,
                args,
                selected_fields,
                &ctx,
            )
            .await
        })
        .await
    }

    async fn delete_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::delete_records(self.inner.as_queryable(), model, record_filter, &ctx).await
        })
        .await
    }

    async fn native_upsert_record(
        &mut self,
        upsert: connector_interface::NativeUpsert,
        trace_id: Option<String>,
    ) -> connector::Result<SingleRecord> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            upsert::native_upsert(self.inner.as_queryable(), upsert, &ctx).await
        })
        .await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        trace_id: Option<String>,
    ) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::m2m_connect(self.inner.as_queryable(), field, parent_id, child_ids, &ctx).await
        })
        .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        trace_id: Option<String>,
    ) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            let ctx = Context::new(&self.connection_info, trace_id.as_deref());
            write::m2m_disconnect(self.inner.as_queryable(), field, parent_id, child_ids, &ctx).await
        })
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::execute_raw(self.inner.as_queryable(), self.features, inputs).await
        })
        .await
    }

    async fn query_raw(
        &mut self,
        _model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        _query_type: Option<String>,
    ) -> connector::Result<serde_json::Value> {
        catch(self.connection_info.clone(), async move {
            write::query_raw(self.inner.as_queryable(), inputs).await
        })
        .await
    }
}
