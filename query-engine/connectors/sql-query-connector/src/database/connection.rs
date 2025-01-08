#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::{catch, transaction::SqlConnectorTransaction};
use crate::{database::operations::*, Context, SqlError};
use async_trait::async_trait;
use connector::ConnectionLike;
use connector_interface::{
    self as connector, AggregationRow, AggregationSelection, Connection, ReadOperations, RecordFilter, Transaction,
    WriteArgs, WriteOperations,
};
use prisma_value::PrismaValue;
use quaint::{
    connector::{IsolationLevel, TransactionCapable},
    prelude::{ConnectionInfo, Queryable},
};
use query_structure::{prelude::*, Filter, QueryArguments, RelationLoadStrategy, SelectionResult};
use std::{collections::HashMap, str::FromStr};
use telemetry::TraceParent;

pub(crate) struct SqlConnection<C> {
    inner: C,
    connection_info: ConnectionInfo,
    features: psl::PreviewFeatures,
}

impl<C> SqlConnection<C>
where
    C: TransactionCapable + Send + Sync + 'static,
{
    pub fn new(inner: C, connection_info: ConnectionInfo, features: psl::PreviewFeatures) -> Self {
        Self {
            inner,
            connection_info,
            features,
        }
    }
}

impl<C> ConnectionLike for SqlConnection<C> where C: Queryable + TransactionCapable + Send + Sync + 'static {}

#[async_trait]
impl<C> Connection for SqlConnection<C>
where
    C: Queryable + TransactionCapable + Send + Sync + 'static,
{
    async fn start_transaction<'a>(
        &'a mut self,
        isolation_level: Option<String>,
    ) -> connector::Result<Box<dyn Transaction + 'a>> {
        let connection_info = &self.connection_info;
        let features = self.features;
        let isolation_level = match isolation_level {
            Some(level) => {
                let transformed = IsolationLevel::from_str(&level)
                    .map_err(SqlError::from)
                    .map_err(|err| err.into_connector_error(connection_info))?;

                Some(transformed)
            }
            None => None,
        };

        let fut_tx = self.inner.start_transaction(isolation_level);

        catch(&self.connection_info, async move {
            let tx = fut_tx.await.map_err(SqlError::from)?;

            Ok(Box::new(SqlConnectorTransaction::new(tx, connection_info, features)) as Box<dyn Transaction>)
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
impl<C> ReadOperations for SqlConnection<C>
where
    C: Queryable + Send + Sync + 'static,
{
    async fn get_single_record(
        &mut self,
        model: &Model,
        filter: &Filter,
        selected_fields: &FieldSelection,
        relation_load_strategy: RelationLoadStrategy,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<Option<SingleRecord>> {
        // [Composites] todo: FieldSelection -> ModelProjection conversion
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            read::get_single_record(
                &self.inner,
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
                &self.inner,
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
        catch(
            &self.connection_info,
            read::get_related_m2m_record_ids(&self.inner, from_field, from_record_ids, &ctx),
        )
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
            read::aggregate(&self.inner, model, query_arguments, selections, group_by, having, &ctx),
        )
        .await
    }
}

#[async_trait]
impl<C> WriteOperations for SqlConnection<C>
where
    C: Queryable + Send + Sync + 'static,
{
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
                &self.inner,
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
            write::create_records_count(&self.inner, model, args, skip_duplicates, &ctx),
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
            write::create_records_returning(&self.inner, model, args, skip_duplicates, selected_fields, &ctx),
        )
        .await
    }

    async fn update_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<usize> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::update_records(&self.inner, model, record_filter, args, &ctx),
        )
        .await
    }

    async fn update_records_returning(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        args: WriteArgs,
        selected_fields: FieldSelection,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<ManyRecords> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::update_records_returning(&self.inner, model, record_filter, args, selected_fields, &ctx),
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
            write::update_record(&self.inner, model, record_filter, args, selected_fields, &ctx),
        )
        .await
    }

    async fn delete_records(
        &mut self,
        model: &Model,
        record_filter: RecordFilter,
        limit: Option<i64>,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<usize> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::delete_records(&self.inner, model, record_filter, limit, &ctx),
        )
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
            write::delete_record(&self.inner, model, record_filter, selected_fields, &ctx),
        )
        .await
    }

    async fn native_upsert_record(
        &mut self,
        upsert: connector_interface::NativeUpsert,
        traceparent: Option<TraceParent>,
    ) -> connector::Result<SingleRecord> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(&self.connection_info, upsert::native_upsert(&self.inner, upsert, &ctx)).await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        traceparent: Option<TraceParent>,
    ) -> connector::Result<()> {
        let ctx = Context::new(&self.connection_info, traceparent);
        catch(
            &self.connection_info,
            write::m2m_connect(&self.inner, field, parent_id, child_ids, &ctx),
        )
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
            write::m2m_disconnect(&self.inner, field, parent_id, child_ids, &ctx),
        )
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector::Result<usize> {
        catch(
            &self.connection_info,
            write::execute_raw(&self.inner, self.features, inputs),
        )
        .await
    }

    async fn query_raw(
        &mut self,
        _model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        _query_type: Option<String>,
    ) -> connector::Result<RawJson> {
        catch(&self.connection_info, write::query_raw(&self.inner, inputs)).await
    }
}
