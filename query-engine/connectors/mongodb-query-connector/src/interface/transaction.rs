use super::*;
use crate::{
    error::MongoError,
    root_queries::{aggregate, read, write},
};
use connector_interface::{ConnectionLike, ReadOperations, Transaction, UpdateType, WriteOperations};
use mongodb::options::{Acknowledgment, ReadConcern, TransactionOptions, WriteConcern};
use query_engine_metrics::{decrement_gauge, increment_gauge, metrics, PRISMA_CLIENT_QUERIES_ACTIVE};
use query_structure::{RelationLoadStrategy, SelectionResult};
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

        utils::commit_with_retry(&mut self.connection.session)
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

    async fn version(&self) -> Option<String> {
        None
    }

    fn as_connection_like(&mut self) -> &mut dyn ConnectionLike {
        self
    }
}

#[async_trait]
impl<'conn> WriteOperations for MongoDbTransaction<'conn> {
    async fn create_record(
        &mut self,
        model: &Model,
        args: connector_interface::WriteArgs,
        // The field selection on a create is never used on MongoDB as it cannot return more than the ID.
        _selected_fields: FieldSelection,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<SingleRecord> {
        catch(write::create_record(
            &self.connection.database,
            &mut self.connection.session,
            model,
            args,
        ))
        .await
    }

    async fn create_records(
        &mut self,
        model: &Model,
        args: Vec<connector_interface::WriteArgs>,
        skip_duplicates: bool,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<usize> {
        catch(write::create_records(
            &self.connection.database,
            &mut self.connection.session,
            model,
            args,
            skip_duplicates,
        ))
        .await
    }

    async fn create_records_returning(
        &mut self,
        _model: &Model,
        _args: Vec<connector_interface::WriteArgs>,
        _skip_duplicates: bool,
        _selected_fields: FieldSelection,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<ManyRecords> {
        unimplemented!()
    }

    async fn update_records(
        &mut self,
        model: &Model,
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
        model: &Model,
        record_filter: connector_interface::RecordFilter,
        args: connector_interface::WriteArgs,
        selected_fields: Option<FieldSelection>,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        catch(async move {
            let result = write::update_records(
                &self.connection.database,
                &mut self.connection.session,
                model,
                record_filter,
                args,
                UpdateType::One,
            )
            .await?;
            // NOTE: Atomic updates are not yet implemented for MongoDB, so we only return ids.
            let record = result.into_iter().next().map(|id| SingleRecord {
                record: Record::from(id),
                field_names: selected_fields
                    .unwrap_or_else(|| model.primary_identifier())
                    .db_names()
                    .collect(),
            });

            Ok(record)
        })
        .await
    }

    async fn delete_records(
        &mut self,
        model: &Model,
        record_filter: connector_interface::RecordFilter,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<usize> {
        catch(write::delete_records(
            &self.connection.database,
            &mut self.connection.session,
            model,
            record_filter,
        ))
        .await
    }

    async fn delete_record(
        &mut self,
        model: &Model,
        record_filter: connector_interface::RecordFilter,
        selected_fields: FieldSelection,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<SingleRecord> {
        catch(write::delete_record(
            &self.connection.database,
            &mut self.connection.session,
            model,
            record_filter,
            selected_fields,
        ))
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
        _trace_id: Option<String>,
    ) -> connector_interface::Result<()> {
        catch(write::m2m_connect(
            &self.connection.database,
            &mut self.connection.session,
            field,
            parent_id,
            child_ids,
        ))
        .await
    }

    async fn m2m_disconnect(
        &mut self,
        field: &RelationFieldRef,
        parent_id: &SelectionResult,
        child_ids: &[SelectionResult],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<()> {
        catch(write::m2m_disconnect(
            &self.connection.database,
            &mut self.connection.session,
            field,
            parent_id,
            child_ids,
        ))
        .await
    }

    async fn execute_raw(&mut self, inputs: HashMap<String, PrismaValue>) -> connector_interface::Result<usize> {
        catch(write::execute_raw(
            &self.connection.database,
            &mut self.connection.session,
            inputs,
        ))
        .await
    }

    async fn query_raw(
        &mut self,
        model: Option<&Model>,
        inputs: HashMap<String, PrismaValue>,
        query_type: Option<String>,
    ) -> connector_interface::Result<serde_json::Value> {
        catch(write::query_raw(
            &self.connection.database,
            &mut self.connection.session,
            model,
            inputs,
            query_type,
        ))
        .await
    }
}

#[async_trait]
impl<'conn> ReadOperations for MongoDbTransaction<'conn> {
    async fn get_single_record(
        &mut self,
        model: &Model,
        filter: &query_structure::Filter,
        selected_fields: &FieldSelection,
        _relation_load_strategy: RelationLoadStrategy,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Option<SingleRecord>> {
        catch(read::get_single_record(
            &self.connection.database,
            &mut self.connection.session,
            model,
            filter,
            selected_fields,
        ))
        .await
    }

    async fn get_many_records(
        &mut self,
        model: &Model,
        query_arguments: query_structure::QueryArguments,
        selected_fields: &FieldSelection,
        _relation_load_strategy: RelationLoadStrategy,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<ManyRecords> {
        catch(read::get_many_records(
            &self.connection.database,
            &mut self.connection.session,
            model,
            query_arguments,
            selected_fields,
        ))
        .await
    }

    async fn get_related_m2m_record_ids(
        &mut self,
        from_field: &RelationFieldRef,
        from_record_ids: &[SelectionResult],
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Vec<(SelectionResult, SelectionResult)>> {
        catch(read::get_related_m2m_record_ids(
            &self.connection.database,
            &mut self.connection.session,
            from_field,
            from_record_ids,
        ))
        .await
    }

    async fn aggregate_records(
        &mut self,
        model: &Model,
        query_arguments: query_structure::QueryArguments,
        selections: Vec<connector_interface::AggregationSelection>,
        group_by: Vec<ScalarFieldRef>,
        having: Option<query_structure::Filter>,
        _trace_id: Option<String>,
    ) -> connector_interface::Result<Vec<connector_interface::AggregationRow>> {
        catch(aggregate::aggregate(
            &self.connection.database,
            &mut self.connection.session,
            model,
            query_arguments,
            selections,
            group_by,
            having,
        ))
        .await
    }
}
