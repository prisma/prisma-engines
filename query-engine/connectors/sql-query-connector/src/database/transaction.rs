use crate::database::operations::*;
use crate::{query_builder::read::ManyRelatedRecordsQueryBuilder, SqlError};
use connector_interface::{
    self as connector, filter::Filter, QueryArguments, ReadOperations, Transaction, WriteArgs, WriteOperations, IO,
};
use prisma_models::prelude::*;
use prisma_value::PrismaValue;
use quaint::prelude::ConnectionInfo;
use std::marker::PhantomData;

pub struct SqlConnectorTransaction<'a, T> {
    inner: quaint::connector::Transaction<'a>,
    connection_info: &'a ConnectionInfo,
    _p: PhantomData<T>,
}

impl<'a, T> SqlConnectorTransaction<'a, T> {
    pub fn new<'b: 'a>(tx: quaint::connector::Transaction<'a>, connection_info: &'b ConnectionInfo) -> Self {
        Self {
            inner: tx,
            connection_info,
            _p: PhantomData,
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

impl<'a, T> Transaction<'a> for SqlConnectorTransaction<'a, T>
where
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn commit<'b>(&'b self) -> IO<'b, ()> {
        IO::new(self.catch(async move { Ok(self.inner.commit().await.map_err(SqlError::from)?) }))
    }

    fn rollback<'b>(&'b self) -> IO<'b, ()> {
        IO::new(self.catch(async move { Ok(self.inner.rollback().await.map_err(SqlError::from)?) }))
    }
}

impl<'a, T> ReadOperations for SqlConnectorTransaction<'a, T>
where
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn get_single_record<'b>(
        &'b self,
        model: &'b ModelRef,
        filter: &'b Filter,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, Option<SingleRecord>> {
        IO::new(self.catch(async move { read::get_single_record(&self.inner, model, filter, selected_fields).await }))
    }

    fn get_many_records<'b>(
        &'b self,
        model: &'b ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, ManyRecords> {
        IO::new(
            self.catch(
                async move { read::get_many_records(&self.inner, model, query_arguments, selected_fields).await },
            ),
        )
    }

    fn get_related_records<'b>(
        &'b self,
        from_field: &'b RelationFieldRef,
        from_record_ids: &'b [RecordIdentifier],
        query_arguments: QueryArguments,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, ManyRecords> {
        IO::new(self.catch(async move {
            read::get_related_records::<T>(
                &self.inner,
                from_field,
                from_record_ids,
                query_arguments,
                selected_fields,
            )
            .await
        }))
    }

    fn count_by_model<'b>(&'b self, model: &'b ModelRef, query_arguments: QueryArguments) -> connector::IO<'b, usize> {
        IO::new(self.catch(async move { read::count_by_model(&self.inner, model, query_arguments).await }))
    }
}

impl<'a, T> WriteOperations for SqlConnectorTransaction<'a, T>
where
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn create_record<'b>(&'b self, model: &'b ModelRef, args: WriteArgs) -> connector::IO<RecordIdentifier> {
        IO::new(self.catch(async move { write::create_record(&self.inner, model, args).await }))
    }

    fn update_records<'b>(
        &'b self,
        model: &'b ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector::IO<Vec<RecordIdentifier>> {
        IO::new(self.catch(async move { write::update_records(&self.inner, model, where_, args).await }))
    }

    fn delete_records<'b>(&'b self, model: &'b ModelRef, where_: Filter) -> connector::IO<usize> {
        IO::new(self.catch(async move { write::delete_records(&self.inner, model, where_).await }))
    }

    fn connect<'b>(
        &'b self,
        field: &'b RelationFieldRef,
        parent_id: &'b RecordIdentifier,
        child_ids: &'b [RecordIdentifier],
    ) -> connector::IO<()> {
        IO::new(self.catch(async move { write::connect(&self.inner, field, parent_id, child_ids).await }))
    }

    fn disconnect<'b>(
        &'b self,
        field: &'b RelationFieldRef,
        parent_id: &'b RecordIdentifier,
        child_ids: &'b [RecordIdentifier],
    ) -> connector::IO<()> {
        IO::new(self.catch(async move { write::disconnect(&self.inner, field, parent_id, child_ids).await }))
    }

    fn execute_raw(&self, query: String, parameters: Vec<PrismaValue>) -> connector::IO<serde_json::Value> {
        IO::new(self.catch(async move { write::execute_raw(&self.inner, query, parameters).await }))
    }
}
