use super::{catch, transaction::SqlConnectorTransaction};
use crate::{database::operations::*, sql_info::SqlInfo, QueryExt, SqlError};
use async_trait::async_trait;
use connector::{ConnectionLike, RelAggregationSelection};
use connector_interface::{
    self as connector, filter::Filter, AggregationRow, AggregationSelection, Connection, QueryArguments,
    ReadOperations, RecordFilter, Transaction, WriteArgs, WriteOperations,
};
use prisma_models::{prelude::*, SelectionResult};
use prisma_value::PrismaValue;
use psl::PreviewFeature;
use quaint::{
    connector::{IsolationLevel, TransactionCapable},
    prelude::ConnectionInfo,
};
use std::{collections::HashMap, str::FromStr};

pub struct SqlConnection<C> {
    inner: C,
    connection_info: ConnectionInfo,
    features: Vec<PreviewFeature>,
}

impl<C> SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    pub fn new(inner: C, connection_info: &ConnectionInfo, features: Vec<PreviewFeature>) -> Self {
        let connection_info = connection_info.clone();

        Self {
            inner,
            connection_info,
            features,
        }
    }
}

impl<C> ConnectionLike for SqlConnection<C> where C: QueryExt + TransactionCapable + Send + Sync + 'static {}

#[async_trait]
impl<C> Connection for SqlConnection<C>
where
    C: QueryExt + TransactionCapable + Send + Sync + 'static,
{
    async fn start_transaction<'a>(
        &'a mut self,
        isolation_level: Option<String>,
    ) -> connector::Result<Box<dyn Transaction + 'a>> {
        let connection_info = &self.connection_info;
        let features = self.features.clone();
        let isolation_level = match isolation_level {
            Some(level) => {
                let transformed = IsolationLevel::from_str(&level)
                    .map_err(SqlError::from)
                    .map_err(|err| err.into_connector_error(&connection_info))?;

                Some(transformed)
            }
            None => None,
        };

        let fut_tx = self.inner.start_transaction(isolation_level);

        catch(self.connection_info.clone(), async move {
            let tx: quaint::connector::Transaction = fut_tx.await.map_err(SqlError::from)?;

            Ok(Box::new(SqlConnectorTransaction::new(tx, connection_info, features)) as Box<dyn Transaction>)
        })
        .await
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl<C> ReadOperations for SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    async fn get_single_record(
        &mut self,
        model: &ModelRef,
        filter: &Filter,
        selected_fields: &FieldSelection,
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> connector::Result<Option<SingleRecord>> {
        // [Composites] todo: FieldSelection -> ModelProjection conversion
        catch(self.connection_info.clone(), async move {
            read::get_single_record(
                &self.inner,
                model,
                filter,
                &selected_fields.into(),
                aggr_selections,
                trace_id,
            )
            .await
        })
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &FieldSelection,
        aggr_selections: &[RelAggregationSelection],
        trace_id: Option<String>,
    ) -> connector::Result<ManyRecords> {
        catch(self.connection_info.clone(), async move {
            read::get_many_records(
                &self.inner,
                model,
                query_arguments,
                &selected_fields.into(),
                aggr_selections,
                SqlInfo::from(&self.connection_info),
                trace_id,
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
            read::get_related_m2m_record_ids(&self.inner, from_field, from_record_ids, trace_id).await
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
        trace_id: Option<String>,
    ) -> connector::Result<Vec<AggregationRow>> {
        catch(self.connection_info.clone(), async move {
            read::aggregate(
                &self.inner,
                model,
                query_arguments,
                selections,
                group_by,
                having,
                trace_id,
            )
            .await
        })
        .await
    }
}

#[async_trait]
impl<C> WriteOperations for SqlConnection<C>
where
    C: QueryExt + Send + Sync + 'static,
{
    async fn create_record(
        &mut self,
        model: &ModelRef,
        args: WriteArgs,
        trace_id: Option<String>,
    ) -> connector::Result<SelectionResult> {
        catch(self.connection_info.clone(), async move {
            write::create_record(&self.inner, &self.connection_info.sql_family(), model, args, trace_id).await
        })
        .await
    }

    async fn create_records(
        &mut self,
        model: &ModelRef,
        args: Vec<WriteArgs>,
        skip_duplicates: bool,
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::create_records(
                &self.inner,
                SqlInfo::from(&self.connection_info),
                model,
                args,
                skip_duplicates,
                trace_id,
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
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::update_records(&self.inner, model, record_filter, args, trace_id).await
        })
        .await
    }

    async fn update_record(
        &mut self,
        model: &ModelRef,
        record_filter: RecordFilter,
        args: WriteArgs,
        trace_id: Option<String>,
    ) -> connector::Result<Option<SelectionResult>> {
        catch(self.connection_info.clone(), async move {
            let mut res = write::update_record(&self.inner, model, record_filter, args, trace_id).await?;
            Ok(res.pop())
        })
        .await
    }

    async fn delete_records(
        &mut self,
        model: &ModelRef,
        record_filter: RecordFilter,
        trace_id: Option<String>,
    ) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::delete_records(&self.inner, model, record_filter, trace_id).await
        })
        .await
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
    ) -> connector::Result<()> {
        catch(self.connection_info.clone(), async move {
            write::m2m_connect(&self.inner, field, parent_id, child_ids).await
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
            write::m2m_disconnect(&self.inner, field, parent_id, child_ids, trace_id).await
        })
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector::Result<usize> {
        catch(self.connection_info.clone(), async move {
            write::execute_raw(&self.inner, &self.features, inputs).await
        })
        .await
    }

    async fn query_raw(
        &mut self,
        _model: Option<&ModelRef>,
        inputs: HashMap<String, PrismaValue>,
        _query_type: Option<String>,
    ) -> connector::Result<serde_json::Value> {
        catch(self.connection_info.clone(), async move {
            write::query_raw(
                &self.inner,
                SqlInfo::from(&self.connection_info),
                &self.features,
                inputs,
            )
            .await
        })
        .await
    }
}
