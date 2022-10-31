use super::*;
use crate::{
    error::MongoError,
    root_queries::{aggregate, read, write},
};
use connector_interface::{
    ConnectionLike, ReadOperations, RelAggregationSelection, Transaction, UpdateType, WriteOperations,
};
use mongodb::options::{Acknowledgment, ReadConcern, TransactionOptions, WriteConcern};
use prisma_models::SelectionResult;
use query_engine_metrics::{decrement_gauge, increment_gauge, metrics, PRISMA_CLIENT_QUERIES_ACTIVE};
use std::collections::HashMap;

pub struct MongoDbTransaction<'conn> {
    connection: &'conn mut MongoDbConnection,
}

impl<'conn> ConnectionLike for MongoDbTransaction<'conn> {}

impl<'conn> MongoDbTransaction<'conn> {
    pub(crate) async fn new(
        connection: &'conn mut MongoDbConnection,
    ) -> connector_interface::Result<MongoDbTransaction<'conn>> {
        let options = TransactionOptions::builder()
            .read_concern(ReadConcern::majority())
            .write_concern(WriteConcern::builder().w(Acknowledgment::Majority).build())
            .build();

        connection
            .session
            .start_transaction(options)
            .await
            .map_err(|err| MongoError::from(err).into_connector_error())?;

        increment_gauge!(PRISMA_CLIENT_QUERIES_ACTIVE, 1.0);

        Ok(Self { connection })
    }
}

#[async_trait]
impl<'conn> Transaction for MongoDbTransaction<'conn> {
    async fn commit(&mut self) -> connector_interface::Result<()> {
        decrement_gauge!(PRISMA_CLIENT_QUERIES_ACTIVE, 1.0);
        self.connection
            .session
            .commit_transaction()
            .await
            .map_err(|err| MongoError::from(err).into_connector_error())?;

        Ok(())
    }

    async fn rollback(&mut self) -> connector_interface::Result<()> {
        decrement_gauge!(PRISMA_CLIENT_QUERIES_ACTIVE, 1.0);
        self.connection
            .session
            .abort_transaction()
            .await
            .map_err(|err| MongoError::from(err).into_connector_error())?;

        Ok(())
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl<'conn> WriteOperations for MongoDbTransaction<'conn> {
    async fn create_record(
        &mut self,
        model: &ModelRef,
        args: connector_interface::WriteArgs,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<SelectionResult> {
        catch(async move {
            write::create_record(&self.connection.database, &mut self.connection.session, model, args).await
        })
        .await
    }

    async fn create_records(
        &mut self,
        model: &ModelRef,
        args: Vec<connector_interface::WriteArgs>,
        skip_duplicates: bool,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<usize> {
        catch(async move {
            write::create_records(
                &self.connection.database,
                &mut self.connection.session,
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
        record_filter: connector_interface::RecordFilter,
        args: connector_interface::WriteArgs,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<usize> {
        catch(async move {
            let result = write::update_records(
                &self.connection.database,
                &mut self.connection.session,
                model,
                record_filter,
                args,
                UpdateType::Many,
            )
            .await?;
            Ok(result.len())
        })
        .await
    }

    async fn update_record(
        &mut self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
        args: connector_interface::WriteArgs,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Option<SelectionResult>> {
        catch(async move {
            let mut res = write::update_records(
                &self.connection.database,
                &mut self.connection.session,
                model,
                record_filter,
                args,
                UpdateType::One,
            )
            .await?;
            Ok(res.pop())
        })
        .await
    }

    async fn delete_records(
        &mut self,
        model: &ModelRef,
        record_filter: connector_interface::RecordFilter,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<usize> {
        catch(async move {
            write::delete_records(
                &self.connection.database,
                &mut self.connection.session,
                model,
                record_filter,
            )
            .await
        })
        .await
    }

    async fn native_upsert_record(
        &mut self,
        _upsert: connector_interface::NativeUpsert,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<SingleRecord> {
        unimplemented!("Native upsert is not currently supported.")
    }

    async fn m2m_connect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
    ) -> connector_interface::Result<()> {
        catch(async move {
            write::m2m_connect(
                &self.connection.database,
                &mut self.connection.session,
                field,
                parent_id,
                child_ids,
            )
            .await
        })
        .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<()> {
        catch(async move {
            write::m2m_disconnect(
                &self.connection.database,
                &mut self.connection.session,
                field,
                parent_id,
                child_ids,
            )
            .await
        })
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector_interface::Result<usize> {
        catch(async move { write::execute_raw(&self.connection.database, &mut self.connection.session, inputs).await })
            .await
    }

    async fn query_raw(
        &mut self,
        model: Option<&ModelRef>,
        inputs: HashMap<String, PrismaValue>,
        query_type: Option<String>,
    ) -> connector_interface::Result<serde_json::Value> {
        catch(async move {
            write::query_raw(
                &self.connection.database,
                &mut self.connection.session,
                model,
                inputs,
                query_type,
            )
            .await
        })
        .await
    }
}

#[async_trait]
impl<'conn> ReadOperations for MongoDbTransaction<'conn> {
    async fn get_single_record(
        &mut self,
        model: &ModelRef,
        filter: &connector_interface::Filter,
        selected_fields: &FieldSelection,
        aggr_selections: &[RelAggregationSelection],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        catch(async move {
            read::get_single_record(
                &self.connection.database,
                &mut self.connection.session,
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
        selected_fields: &FieldSelection,
        aggregation_selections: &[RelAggregationSelection],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<ManyRecords> {
        catch(async move {
            read::get_many_records(
                &self.connection.database,
                &mut self.connection.session,
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
        from_record_ids: &[SelectionResult],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Vec<(SelectionResult, SelectionResult)>> {
        catch(async move {
            read::get_related_m2m_record_ids(
                &self.connection.database,
                &mut self.connection.session,
                from_field,
                from_record_ids,
            )
            .await
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
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Vec<connector_interface::AggregationRow>> {
        catch(async move {
            aggregate::aggregate(
                &self.connection.database,
                &mut self.connection.session,
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
