use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use datamodel::Datasource;
use quaint::{connector::SqliteParams, error::ErrorKind as QuaintKind, pooled::Quaint, prelude::ConnectionInfo};
use std::{convert::TryFrom, time::Duration};

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
}

#[async_trait]
impl FromSource for Sqlite {
    async fn from_source(source: &Datasource) -> connector_interface::Result<Sqlite> {
        let database_str = &source.url().value;

        let connection_info = ConnectionInfo::from_url(database_str)
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        let params = SqliteParams::try_from(database_str.as_str())
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        let file_path = params.file_path;
        let url = database_str.split('?').next();

        if url.is_none() || std::path::Path::new(url.unwrap()).file_stem().is_none() {
            return Err(invalid_file_path_error(&file_path, &connection_info));
        }

        let mut builder = Quaint::builder(database_str.as_str())
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        builder.max_idle_lifetime(Duration::from_secs(300));
        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();

        Ok(Sqlite { pool, file_path })
    }
}

fn invalid_file_path_error(file_path: &str, connection_info: &ConnectionInfo) -> ConnectorError {
    SqlError::ConnectionError(QuaintKind::DatabaseUrlIsInvalid(format!(
        "\"{}\" is not a valid sqlite file path",
        file_path
    )))
    .into_connector_error(&connection_info)
}

#[async_trait]
impl Connector for Sqlite {
    #[tracing::instrument(skip(self))]
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(&self.connection_info(), async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, self.connection_info());

            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> String {
        "sqlite".to_owned()
    }
}
