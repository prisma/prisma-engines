use super::catch;
use crate::{SqlError, database::operations::*};
use async_trait::async_trait;
use connector::ConnectionLike;
use connector_interface::{self as connector, AggregationRow, ReadOperations, Transaction, WriteOperations};
use prisma_value::PrismaValue;
use quaint::prelude::ConnectionInfo;
use query_structure::{
    AggregationSelection, Filter, QueryArguments, RecordFilter, RelationLoadStrategy, SelectionResult, WriteArgs,
    prelude::*,
};
use sql_query_builder::Context;
use std::collections::HashMap;
use telemetry::TraceParent;

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

impl ConnectionLike for SqlConnectorTransaction<'_> {}

#[async_trait]
impl Transaction for SqlConnectorTransaction<'_> {
    async fn commit(&mut self) -> connector::Result<()> {
        catch(&self.connection_info, async {
            self.inner.commit().await.map_err(SqlError::from)
        })
        .await
    }

    async fn rollback(&mut self) -> connector::Result<()> {
        catch(&self.connection_info, async {
            let res = self.inner.rollback().await.map_err(SqlError::from);

            match res {
                Err(SqlError::TransactionAlreadyClosed(_)) | Err(SqlError::RollbackWithoutBegin) => Ok(()),
                _ => res,
            }
        })
        .await
    }

    async fn version(&self) -> Option<String> {
        self.connection_info.version().map(|v| v.to_string())
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl ReadOperations for SqlConnectorTransaction<'_> {
    async fn get_single_record(
        &mut self,
        model: &Model,
        filter: &Filter,
        selected_fields: &FieldSelection,
        relation_load_strategy: RelationLoadStrategy,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<Option<SingleRecord>> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            read::get_single_record(
                self.inner.as_queryable(),
                model,
                filter,
                selected_fields,
                relation_load_strategy,
                &ctx,
            ),
        )
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &Model,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
        relation_load_strategy: RelationLoadStrategy,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<ManyRecords> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            read::get_many_records(
                self.inner.as_queryable(),
                model,
                query_arguments,
                selected_fields,
                relation_load_strategy,
                &ctx,
            ),
        )
        .await
    }

    async fn get_related_m2m_record_ids(
        &mut self,
        from_field: &RelationFieldRef,
        from_record_ids: &[SelectionResult],
        traceparent: Option<TraceParent>,
    ) -> connector::Result<Vec<(SelectionResult, SelectionResult)>> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(&self.connection_info, async {
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
        traceparent: Option<TraceParent>,
    ) -> connector::Result<Vec<AggregationRow>> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            read::aggregate(
                self.inner.as_queryable(),
                model,
                query_arguments,
                selections,
                group_by,
                having,
                &ctx,
            ),
        )
        .await
    }
}

#[async_trait]
impl WriteOperations for SqlConnectorTransaction<'_> {
    async fn create_record(
        &mut self,
        model: &Model,
        args: WriteArgs,
        selected_fields: FieldSelection,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<SingleRecord> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::create_record(
                self.inner.as_queryable(),
                &self.connection_info.sql_family(),
                model,
                args,
                selected_fields,
                &ctx,
            ),
        )
        .await
    }

    async fn create_records(
        &mut self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<usize> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::create_records_count(self.inner.as_queryable(), model, args, skip_duplicates, &ctx),
        )
        .await
    }

    async fn create_records_returning(
        &mut self,
        model: &Model,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        selected_fields: FieldSelection,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<ManyRecords> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::create_records_returning(
                self.inner.as_queryable(),
                model,
                args,
                skip_duplicates,
                selected_fields,
                &ctx,
            ),
        )
        .await
    }

    async fn update_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        limit: Option<usize>,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<usize> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::update_records(self.inner.as_queryable(), model, record_filter, args, limit, &ctx),
        )
        .await
    }

    async fn update_records_returning(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: FieldSelection,
        limit: Option<usize>,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<ManyRecords> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::update_records_returning(
                self.inner.as_queryable(),
                model,
                record_filter,
                args,
                selected_fields,
                limit,
                &ctx,
            ),
        )
        .await
    }

    async fn update_record(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: Option<FieldSelection>,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<Option<SingleRecord>> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::update_record(
                self.inner.as_queryable(),
                model,
                record_filter,
                args,
                selected_fields,
                &ctx,
            ),
        )
        .await
    }

    async fn delete_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        limit: Option<usize>,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<usize> {
        catch(&self.connection_info, async {
            let ctx = Context::new(&self.connection_info, traceparent);
            write::delete_records(self.inner.as_queryable(), model, record_filter, limit, &ctx).await
        })
        .await
    }

    async fn delete_record(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        selected_fields: FieldSelection,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<SingleRecord> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::delete_record(self.inner.as_queryable(), model, record_filter, selected_fields, &ctx),
        )
        .await
    }

    async fn native_upsert_record(
        &mut self,
        upsert: connector_interface::NativeUpsert,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<SingleRecord> {
        catch(&self.connection_info, async {
            let ctx = Context::new(&self.connection_info, traceparent);
            upsert::native_upsert(self.inner.as_queryable(), upsert, &ctx).await
        })
        .await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        traceparent: Option<TraceParent>,
    ) -> connector::Result<()> {
        catch(&self.connection_info, async {
            let ctx = Context::new(&self.connection_info, traceparent);
            write::m2m_connect(self.inner.as_queryable(), field, parent_id, child_ids, &ctx).await
        })
        .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        traceparent: Option<TraceParent>,
    ) -> connector::Result<()> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::m2m_disconnect(self.inner.as_queryable(), field, parent_id, child_ids, &ctx),
        )
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector::Result<usize> {
        catch(
            &self.connection_info,
            write::execute_raw(self.inner.as_queryable(), self.features, inputs),
        )
        .await
    }

    async fn query_raw(
        &mut self,
        _model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        _query_type: Option<String>,
    ) -> connector::Result<RawJson> {
        catch(
            &self.connection_info,
            write::query_raw(self.inner.as_queryable(), inputs),
        )
        .await
    }
}
