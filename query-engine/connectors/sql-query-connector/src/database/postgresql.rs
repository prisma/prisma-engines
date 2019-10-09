use super::transaction::SqlConnectorTransaction;
use crate::{
    query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError,
};
use connector_interface::*;
use datamodel::Source;
use prisma_query::{
    connector::{PostgresParams, Queryable},
    pool::{postgres::PostgresManager, PrismaConnectionManager},
};
use std::convert::TryFrom;

type Pool = r2d2::Pool<PrismaConnectionManager<PostgresManager>>;

pub struct PostgreSql {
    pool: Pool,
}

impl FromSource for PostgreSql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let url = url::Url::parse(&source.url().value)?;
        let params = PostgresParams::try_from(url)?;
        let pool = r2d2::Pool::try_from(params).unwrap();

        Ok(PostgreSql { pool })
    }
}

impl Connector for PostgreSql {
    fn with_transaction<F, T>(&self, f: F) -> connector_interface::Result<T>
    where
        F: FnOnce(&mut dyn connector_interface::TransactionLike) -> connector_interface::Result<T>,
    {
        let mut conn = self.pool.get().map_err(SqlError::from)?;
        let tx = conn.start_transaction().map_err(SqlError::from)?;
        let mut tx = SqlConnectorTransaction::<ManyRelatedRecordsWithRowNumber>::new(tx);
        let result = f(&mut tx);

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }
}
