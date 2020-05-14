use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use datamodel::Source;
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
    async fn from_source(source: &dyn Source) -> connector_interface::Result<Sqlite> {
        let connection_info = ConnectionInfo::from_url(&source.url().value)
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        let params = SqliteParams::try_from(source.url().value.as_str())
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        let file_path = params.file_path;

        let url_with_db = {
            let db_name = std::path::Path::new(&file_path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| invalid_file_path_error(&file_path, &connection_info))?
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

        let mut builder = Quaint::builder(url_with_db.as_str())
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
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + 'a>> {
        super::catch(&self.connection_info(), async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, self.connection_info());

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
        .await
    }
}
