use super::connection::SqlConnection;
use crate::{FromSource, SqlError};
use async_trait::async_trait;
use connector_interface::{
    self as connector,
    error::{ConnectorError, ErrorKind},
    Connection, Connector,
};
use datamodel::{common::preview_features::PreviewFeature, Datasource};
use quaint::{connector::SqliteParams, error::ErrorKind as QuaintKind, pooled::Quaint, prelude::ConnectionInfo};
use std::{convert::TryFrom, time::Duration};

pub struct Sqlite {
    pool: Quaint,
    file_path: String,
    features: Vec<PreviewFeature>,
}

impl Sqlite {
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.pool.connection_info()
    }

    /// Get SQLite's preview features.
    pub fn features(&self) -> &[PreviewFeature] {
        self.features.as_ref()
    }
}

#[async_trait]
impl FromSource for Sqlite {
    async fn from_source(
        _source: &Datasource,
        url: &str,
        features: &[PreviewFeature],
    ) -> connector_interface::Result<Sqlite> {
        let database_str = url;

        let connection_info = ConnectionInfo::from_url(database_str)
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        let params = SqliteParams::try_from(database_str)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        let file_path = params.file_path;
        let url = database_str.split('?').next();

        if url.is_none() || std::path::Path::new(url.unwrap()).file_stem().is_none() {
            return Err(invalid_file_path_error(&file_path, &connection_info));
        }

        let mut builder = Quaint::builder(database_str)
            .map_err(SqlError::from)
            .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

        builder.health_check_interval(Duration::from_secs(15));
        builder.test_on_check_out(true);

        let pool = builder.build();

        Ok(Sqlite {
            pool,
            file_path,
            features: features.to_owned(),
        })
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
    async fn get_connection<'a>(&'a self) -> connector::Result<Box<dyn Connection + Send + Sync + 'static>> {
        super::catch(self.connection_info().clone(), async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::new(conn, self.connection_info(), self.features.clone());

            Ok(Box::new(conn) as Box<dyn Connection + Send + Sync + 'static>)
        })
        .await
    }

    fn name(&self) -> String {
        "sqlite".to_owned()
    }
}
