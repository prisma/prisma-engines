use super::transaction::SqlConnectorTransaction;
use crate::{database::operations::*, query_builder::read::ManyRelatedRecordsQueryBuilder, QueryExt, SqlError};
use connector_interface::{
    self as connector,
    filter::{Filter, RecordFinder},
    Connection, QueryArguments, ReadOperations, ScalarListValues, Transaction, WriteArgs, WriteOperations, IO
};
use prisma_models::prelude::*;
use std::marker::PhantomData;

pub struct SqlConnection<C, T> {
    inner: C,
    _p: PhantomData<T>,
}

impl<'a, C, T> SqlConnection<C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    pub fn new(inner: C) -> Self {
        Self { inner, _p: PhantomData }
    }
}

impl<C, T> Connection for SqlConnection<C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn start_transaction<'a>(&'a self) -> IO<'a, Box<dyn Transaction<'a> + 'a>> {
        let fut_tx = self.inner.start_transaction();

        IO::new(async move {
            let tx: quaint::connector::Transaction<'a> = fut_tx.await.map_err(SqlError::from)?;

            Ok(Box::new(SqlConnectorTransaction::<T>::new(tx)) as Box<dyn Transaction<'a> + 'a>)
        })
    }
}

impl<C, T> ReadOperations for SqlConnection<C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn get_single_record<'b>(
        &'b self,
        record_finder: &'b RecordFinder,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, Option<SingleRecord>> {
        IO::new(async move { read::get_single_record(&self.inner, record_finder, selected_fields).await })
    }

    fn get_many_records<'b>(
        &'b self,
        model: &'b ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, ManyRecords> {
        IO::new(async move { read::get_many_records(&self.inner, model, query_arguments, selected_fields).await })
    }

    fn get_related_records<'b>(
        &'b self,
        from_field: &'b RelationFieldRef,
        from_record_ids: &'b [GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &'b SelectedFields,
    ) -> connector::IO<'b, ManyRecords> {
        IO::new(async move {
            read::get_related_records::<T>(
                &self.inner,
                from_field,
                from_record_ids,
                query_arguments,
                selected_fields,
            )
            .await
        })
    }

    fn get_scalar_list_values<'b>(
        &'b self,
        list_field: &'b ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> connector::IO<'b, Vec<ScalarListValues>> {
        IO::new(async move { read::get_scalar_list_values(&self.inner, list_field, record_ids).await })
    }

    fn count_by_model<'b>(&'b self, model: &'b ModelRef, query_arguments: QueryArguments) -> connector::IO<'b, usize> {
        IO::new(async move { read::count_by_model(&self.inner, model, query_arguments).await })
    }
}

impl<C, T> WriteOperations for SqlConnection<C, T>
where
    C: QueryExt + Send + Sync + 'static,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn create_record<'a>(&'a self, model: &'a ModelRef, args: WriteArgs) -> connector::IO<GraphqlId> {
        IO::new(async move { write::create_record(&self.inner, model, args).await })
    }

    fn update_records<'a>(&'a self, model: &'a ModelRef, where_: Filter, args: WriteArgs) -> connector::IO<Vec<GraphqlId>> {
        IO::new(async move { write::update_records(&self.inner, model, where_, args).await })
    }

    fn delete_records<'a>(&'a self, model: &'a ModelRef, where_: Filter) -> connector::IO<usize> {
        IO::new(async move { write::delete_records(&self.inner, model, where_).await })
    }

    fn connect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> connector::IO<()> {
        IO::new(async move { write::connect(&self.inner, field, parent_id, child_id).await })
    }

    fn disconnect<'a>(
        &'a self,
        field: &'a RelationFieldRef,
        parent_id: &'a GraphqlId,
        child_id: &'a GraphqlId,
    ) -> connector::IO<()> {
        IO::new(async move { write::disconnect(&self.inner, field, parent_id, child_id).await })
    }

    fn set<'a>(
        &'a self,
        relation_field: &'a RelationFieldRef,
        parent_id: GraphqlId,
        wheres: Vec<GraphqlId>,
    ) -> connector::IO<()> {
        IO::new(async move { write::set(&self.inner, relation_field, parent_id, wheres).await })
    }
}
