use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, QueryExt, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::{
    connector::SqliteParams,
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
        let db_name = std::path::Path::new(&params.file_path).file_stem().unwrap().to_str().unwrap().to_owned();
        let file_path = params.file_path;
        let pool = pool::sqlite(&file_path, &db_name)?;

        Ok(Self { pool, file_path })
    }
}

impl Connector for Sqlite {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn);

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
