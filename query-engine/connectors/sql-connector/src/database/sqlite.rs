use super::connector_transaction::ConnectorTransaction;
use crate::{
    query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlCapabilities, SqlError, Transaction, Transactional,
};
use connector_interface::Connector;
use datamodel::Source;
use prisma_query::{
    ast::ParameterizedValue,
    connector::{Queryable, SqliteParams},
    pool::{sqlite::SqliteConnectionManager, PrismaConnectionManager},
};
use std::{collections::HashSet, convert::TryFrom};

type Pool = r2d2::Pool<PrismaConnectionManager<SqliteConnectionManager>>;

pub struct Sqlite {
    pool: Pool,
    test_mode: bool,
    file_path: String,
}

impl Sqlite {
    pub fn new(file_path: String, connection_limit: u32, test_mode: bool) -> crate::Result<Self> {
        let manager = PrismaConnectionManager::sqlite(&file_path)?;
        let pool = r2d2::Pool::builder().max_size(connection_limit).build(manager)?;

        Ok(Self {
            pool,
            test_mode,
            file_path,
        })
    }

    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }
}

impl FromSource for Sqlite {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let params = SqliteParams::try_from(source.url().value.as_str())?;

        let file_path = params.file_path.clone();
        let pool = r2d2::Pool::try_from(params).unwrap();

        Ok(Sqlite {
            pool,
            test_mode: false,
            file_path: file_path.to_str().unwrap().to_string(),
        })
    }
}

impl SqlCapabilities for Sqlite {
    type ManyRelatedRecordsBuilder = ManyRelatedRecordsWithRowNumber;
}

impl Transactional for Sqlite {
    fn with_transaction<F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut dyn Transaction) -> crate::Result<T>,
    {
        let mut conn = self.pool.get()?;

        let databases: HashSet<String> = conn
            .query_raw("PRAGMA database_list", &[])?
            .into_iter()
            .map(|rr| {
                let db_name = rr.into_iter().nth(1).unwrap();

                db_name.into_string().unwrap()
            })
            .collect();

        if !databases.contains(db) {
            // This is basically hacked until we have a full rust stack with a migration engine.
            // Currently, the scala tests use the JNA library to write to the database.
            conn.execute_raw(
                "ATTACH DATABASE ? AS ?",
                &[
                    ParameterizedValue::from(self.file_path.as_ref()),
                    ParameterizedValue::from(db),
                ],
            )?;
        }

        conn.execute_raw("PRAGMA foreign_keys = ON", &[])?;

        let result = {
            let mut tx = conn.start_transaction()?;
            let result = f(&mut tx);

            if result.is_ok() {
                tx.commit()?;
            }

            result
        };

        if self.test_mode {
            conn.execute_raw("DETACH DATABASE ?", &[ParameterizedValue::from(db)])?;
        }

        result
    }
}

impl Connector for Sqlite {
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
