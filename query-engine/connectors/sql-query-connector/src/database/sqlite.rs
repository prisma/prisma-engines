use super::transaction::SqlConnectorTransaction;
use crate::{FromSource, QueryExt, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use prisma_query::{
    connector::{Queryable, SqliteParams},
    pool::{self, SqliteManager},
};
use std::convert::TryFrom;
use tokio_resource_pool::{CheckOut, Pool};

pub struct Sqlite {
    pool: Pool<SqliteManager>,
    file_path: String,
}

impl QueryExt for CheckOut<SqliteManager> {}

impl Sqlite {
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }
}

impl FromSource for Sqlite {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let params = SqliteParams::try_from(source.url().value.as_str())?;
        let file_path = params.file_path.to_str().unwrap().to_string();

        Self::new(file_path)
    }
}

impl Connector for Sqlite {
    fn get_connection(&self) -> IO<Box<dyn Connection>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            unimplemented!();
            // Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
