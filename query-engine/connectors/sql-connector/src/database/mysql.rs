use super::connector_transaction::ConnectorTransaction;
use crate::{
    query_builder::ManyRelatedRecordsWithUnionAll, FromSource, SqlCapabilities, SqlError, Transaction, Transactional,
};
use connector_interface::Connector;
use datamodel::Source;
use prisma_query::{
    connector::{MysqlParams, Queryable},
    pool::{mysql::MysqlConnectionManager, PrismaConnectionManager},
};
use std::convert::TryFrom;
use url::Url;

type Pool = r2d2::Pool<PrismaConnectionManager<MysqlConnectionManager>>;

pub struct Mysql {
    pool: Pool,
}

impl FromSource for Mysql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let url = Url::parse(&source.url().value)?;
        let params = MysqlParams::try_from(url)?;
        let pool = r2d2::Pool::try_from(params).unwrap();

        Ok(Mysql { pool })
    }
}

impl SqlCapabilities for Mysql {
    type ManyRelatedRecordsBuilder = ManyRelatedRecordsWithUnionAll;
}

impl Transactional for Mysql {
    fn with_transaction<F, T>(&self, _: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn Transaction) -> crate::Result<T>,
    {
        let mut conn = self.pool.get()?;
        let mut tx = conn.start_transaction()?;

        let result = f(&mut tx);

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }
}

impl Connector for Mysql {
    fn with_transaction<F, T>(&self, f: F) -> connector_interface::Result<T>
    where
        F: FnOnce(&mut dyn connector_interface::MaybeTransaction) -> connector_interface::Result<T>,
    {
        let mut conn = self.pool.get().map_err(SqlError::from)?;
        let tx = conn.start_transaction().map_err(SqlError::from)?;
        let mut connector_transaction = ConnectorTransaction::new(tx);
        let result = f(&mut connector_transaction);

        if result.is_ok() {
            connector_transaction.commit()?;
        }

        result
    }
}
