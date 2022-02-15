mod connection;
mod transaction;

pub use connection::*;
pub use transaction::*;

use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connector,
};
use datamodel::Datasource;
use futures::Future;
use mongodb::Client;
use prisma_models::prelude::*;
use url::Url;

use crate::error::MongoError;

/// The MongoDB connector struct.
pub struct MongoDb {
    /// The MongoDB client has a connection pool internally.
    client: Client,

    /// The database used for all connections.
    database: String,
}

impl MongoDb {
    pub async fn new(_source: &Datasource, url: &str) -> connector_interface::Result<Self> {
        let database_str = url;
        let url = Url::parse(database_str).map_err(|_err| {
            ConnectorError::from_kind(ErrorKind::InvalidDatabaseUrl {
                details: "Unable to parse URL.".to_owned(),
                url: url.to_owned(),
            })
        })?;

        let database = url.path().trim_start_matches('/').to_string();

        let client = mongodb_client::create(&url)
            .await
            .map_err(|err| ConnectorError::from_kind(ErrorKind::ConnectionError(err.into())))?;

        Ok(Self { client, database })
    }

    pub fn db_name(&self) -> &str {
        &self.database
    }
}

#[async_trait]
impl Connector for MongoDb {
    async fn get_connection(
        &self,
    ) -> connector_interface::Result<Box<dyn connector_interface::Connection + Send + Sync>> {
        let session = self
            .client
            .start_session(None)
            .await
            .map_err(|err| MongoError::from(err).into_connector_error())?;

        Ok(Box::new(MongoDbConnection {
            session,
            database: self.client.database(&self.database),
        }))
    }

    fn name(&self) -> String {
        "mongodb".to_owned()
    }
}

async fn catch<O>(
    fut: impl Future<Output = Result<O, MongoError>>,
) -> Result<O, connector_interface::error::ConnectorError> {
    match fut.await {
        Ok(o) => Ok(o),
        Err(err) => Err(err.into_connector_error()),
    }
}
