use super::transaction::SqlConnectorTransaction;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use connector_interface::Connector;
use datamodel::Source;
use prisma_query::{
    connector::{Queryable, SqliteParams},
    pool::{sqlite::SqliteConnectionManager, PrismaConnectionManager},
};
use std::convert::TryFrom;

type Pool = r2d2::Pool<PrismaConnectionManager<SqliteConnectionManager>>;

pub struct Sqlite {
    pool: Pool,
    file_path: String,
}

impl Sqlite {
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }
}

impl FromSource for Sqlite {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let params = SqliteParams::try_from(source.url().value.as_str())?;
        let file_path = params.file_path.clone();
        let pool = r2d2::Pool::try_from(params).unwrap();
        let sqlite = Sqlite {
            pool,
            file_path: file_path.to_str().unwrap().to_string(),
        };

        Ok(sqlite)
    }
}

impl Connector for Sqlite {
    fn with_transaction<F, T>(&self, f: F) -> connector_interface::Result<T>
    where
        F: FnOnce(&mut dyn connector_interface::TransactionLike) -> connector_interface::Result<T>,
    {
        let mut conn = self.pool.get().map_err(SqlError::from)?;
        let tx = conn.start_transaction().map_err(SqlError::from)?;
        let mut connector_transaction = SqlConnectorTransaction::<ManyRelatedRecordsWithRowNumber>::new(tx);
        let result = f(&mut connector_transaction);

        if result.is_ok() {
            connector_transaction.commit()?;
        }

        result
    }
}
