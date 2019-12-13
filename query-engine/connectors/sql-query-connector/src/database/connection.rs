use super::transaction::SqlConnectorTransaction;
use crate::{database::operations::*, query_builder::read::ManyRelatedRecordsQueryBuilder, QueryExt, SqlError};
use connector_interface::{
    self as connector, filter::Filter, Connection, QueryArguments, ReadOperations, Transaction,
    WriteArgs, WriteOperations, IO,
};
use prisma_models::prelude::*;
use quaint::{connector::TransactionCapable, prelude::ConnectionInfo};
use std::marker::PhantomData;

pub struct SqlConnection<'a, C, T> {
    inner: C,
    connection_info: &'a ConnectionInfo,
    _p: PhantomData<T>,
}

impl<'a, C, T> SqlConnection<'a, C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    pub fn new(inner: C, connection_info: &'a ConnectionInfo) -> Self {
        Self {
            inner,
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

impl<'conninfo, C, T> Connection for SqlConnection<'conninfo, C, T>
where
    C: QueryExt + TransactionCapable + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn start_transaction<'a>(&'a self) -> IO<'a, Box<dyn Transaction<'a> + 'a>> {
        let fut_tx = self.inner.start_transaction();
        let connection_info = self.connection_info;

        IO::new(self.catch(async move {
            let tx: quaint::connector::Transaction<'a> = fut_tx.await.map_err(SqlError::from)?;
            Ok(Box::new(SqlConnectorTransaction::<T>::new(tx, connection_info)) as Box<dyn Transaction<'a> + 'a>)
        }))
    }
}

impl<'a, C, T> ReadOperations for SqlConnection<'a, C, T>
where
    C: QueryExt + Send + Sync + 'static,
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
        from_record_ids: &'b [GraphqlId],
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

impl<'conn, C, T> WriteOperations for SqlConnection<'conn, C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> connector::IO<GraphqlId> {
        IO::new(self.catch(async move { write::create_record(&self.inner, model, args).await }))
    }

    fn update_records<'a>(
        &'a self,
        model: &'a ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector::IO<Vec<GraphqlId>> {
        IO::new(self.catch(async move { write::update_records(&self.inner, model, where_, args).await }))
    }

    fn delete_records<'a>(&'a self, model: &'a ModelRef, where_: Filter) -> connector::IO<usize> {
        IO::new(self.catch(async move { write::delete_records(&self.inner, model, where_).await }))
    }

    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_ids: &'a [GraphqlId],
    ) -> connector::IO<()> {
        IO::new(self.catch(async move { write::connect(&self.inner, field, parent_id, child_ids).await }))
    }

    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_ids: &'a [GraphqlId],
    ) -> connector::IO<()> {
        IO::new(self.catch(async move { write::disconnect(&self.inner, field, parent_id, child_ids).await }))
    }
}
