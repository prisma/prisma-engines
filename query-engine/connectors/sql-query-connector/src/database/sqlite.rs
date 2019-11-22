use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use connector_interface::{Connection, Connector, error::ConnectorError, IO};
use datamodel::Source;
use quaint::{connector::SqliteParams, prelude::ConnectionInfo, Quaint};
use std::convert::TryFrom;
use async_trait::async_trait;

pub struct Sqlite {
    pool: Quaint,
    file_path: String,
}

impl Sqlite {
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.pool.connection_info()
    }

    async fn catch<O>(&self, fut: impl std::future::Future<Output = Result<O, crate::SqlError>>) -> Result<O, ConnectorError> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(err) => Err(err.into_connector_error(self.connection_info())),
        }
    }
}

#[async_trait]
impl FromSource for Sqlite {
    async fn from_source(source: &dyn Source) -> crate::Result<Sqlite> {
        let params = SqliteParams::try_from(source.url().value.as_str())?;

        let file_path = params.file_path;

        let url_with_db = {
            let db_name = std::path::Path::new(&file_path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();

            let mut splitted = source.url().value.split("?");
            let url = splitted.next().unwrap();
            let params = splitted.next();

            let mut params: Vec<&str> = match params {
                Some(params) => params.split("&").collect(),
                None => Vec::with_capacity(1),
            };

            let db_name_param = format!("db_name={}", db_name);
            params.push(&db_name_param);

            format!("{}?{}", url, params.join("&"))
        };

        let pool = Quaint::new(url_with_db.as_str()).await?;

        Ok(Sqlite { pool, file_path })
    }
}

impl Connector for Sqlite {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(self.catch(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn, self.connection_info());

            Ok(Box::new(conn) as Box<dyn Connection>)
        }))
    }
}
