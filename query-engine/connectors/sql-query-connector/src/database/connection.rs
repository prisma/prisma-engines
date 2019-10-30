use super::transaction::SqlConnectorTransaction;
use crate::{query_builder::read::ManyRelatedRecordsQueryBuilder, QueryExt, SqlError};
use connector_interface::{
    self as connector,
    filter::{Filter, RecordFinder},
    Connection, QueryArguments, ReadOperations, ScalarListValues, Transaction, WriteArgs, WriteOperations, IO,
};
use prisma_models::prelude::*;
use prisma_query::connector::Queryable;
use std::marker::PhantomData;

pub struct SqlConnection<C, T> {
    inner: C,
    _p: PhantomData<T>,
}

impl<'a, C, T> SqlConnection<C, T>
where
    C: QueryExt,
    T: ManyRelatedRecordsQueryBuilder,
{
    pub fn new(inner: C) -> Self {
        Self { inner, _p: PhantomData }
    }
}

impl<C, T> Connection for SqlConnection<C, T>
where
    C: QueryExt,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn start_transaction<'a>(&'a self) -> IO<Box<dyn Transaction + 'a>> {
        IO::new(async {
            let fut_tx = self.inner.start_transaction();
            let tx: prisma_query::connector::Transaction = fut_tx.await.map_err(SqlError::from)?;

            Ok(Box::new(SqlConnectorTransaction::<T>::new(tx)) as Box<dyn Transaction + 'a>)
        })
    }
}

impl<C, T> ReadOperations for SqlConnection<C, T>
where
    C: QueryExt,
    T: ManyRelatedRecordsQueryBuilder + Send + Sync + 'static,
{
    fn get_single_record(
        &mut self,
        record_finder: &RecordFinder,
        selected_fields: &SelectedFields,
    ) -> connector::Result<Option<SingleRecord>> {
        unimplemented!()
    }

    fn get_many_records(
        &mut self,
        model: ModelRef,
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector::Result<ManyRecords> {
        unimplemented!()
    }

    fn get_related_records(
        &mut self,
        from_field: RelationFieldRef,
        from_record_ids: &[GraphqlId],
        query_arguments: QueryArguments,
        selected_fields: &SelectedFields,
    ) -> connector::Result<ManyRecords> {
        unimplemented!()
    }

    // This method is temporary
    fn get_scalar_list_values(
        &mut self,
        list_field: ScalarFieldRef,
        record_ids: Vec<GraphqlId>,
    ) -> connector::Result<Vec<ScalarListValues>> {
        unimplemented!()
    }

    // This will eventually become a more generic `aggregate`
    fn count_by_model(&mut self, model: ModelRef, query_arguments: QueryArguments) -> connector::Result<usize> {
        unimplemented!()
    }
}

impl<T, C> WriteOperations for SqlConnection<T, C> {
    fn create_record(&mut self, model: ModelRef, args: WriteArgs) -> connector::Result<GraphqlId> {
        unimplemented!()
    }

    fn update_records(
        &mut self,
        model: ModelRef,
        where_: Filter,
        args: WriteArgs,
    ) -> connector::Result<Vec<GraphqlId>> {
        unimplemented!()
    }

    fn delete_records(&mut self, model: ModelRef, where_: Filter) -> connector::Result<usize> {
        unimplemented!()
    }

    // We plan to remove the methods below in the future. We want emulate them with the ones above. Those should suffice.

    fn connect(
        &mut self,
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector::Result<()> {
        unimplemented!()
    }

    fn disconnect(
        &mut self,
        field: RelationFieldRef,
        parent_id: &GraphqlId,
        child_id: &GraphqlId,
    ) -> connector::Result<()> {
        unimplemented!()
    }

    fn set(
        &mut self,
        relation_field: RelationFieldRef,
        parent: GraphqlId,
        wheres: Vec<GraphqlId>,
    ) -> connector::Result<()> {
        unimplemented!()
    }
}
