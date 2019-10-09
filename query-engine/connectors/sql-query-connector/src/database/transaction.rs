mod read;
mod write;

use crate::query_builder::read::ManyRelatedRecordsQueryBuilder;
use crate::SqlError;
use connector_interface::*;
use std::marker::PhantomData;

pub struct SqlConnectorTransaction<'a, T> {
    inner: prisma_query::connector::Transaction<'a>,
    _p: PhantomData<T>,
}

impl<'a, T> SqlConnectorTransaction<'a, T> {
    pub fn new(tx: prisma_query::connector::Transaction<'a>) -> Self {
        Self {
            inner: tx,
            _p: PhantomData,
        }
    }

    pub fn commit(self) -> connector_interface::Result<()> {
        Ok(self.inner.commit().map_err(SqlError::from)?)
    }
}

impl<T> TransactionLike for SqlConnectorTransaction<'_, T> where T: ManyRelatedRecordsQueryBuilder {}


