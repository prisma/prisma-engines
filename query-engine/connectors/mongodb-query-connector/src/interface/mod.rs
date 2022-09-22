mod connection;
mod transaction;

pub use connection::*;
pub use transaction::*;

use async_trait::async_trait;
use connector_interface::{
    error::{ConnectorError, ErrorKind},
    Connector,
};
use futures::Future;
use mongodb::Client;
use prisma_models::prelude::*;
use psl::Datasource;

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
        let client = mongodb_client::create(&url).await.map_err(|err| {
            let kind = match err.kind {
                mongodb_client::ErrorKind::InvalidArgument { message } => ErrorKind::InvalidDatabaseUrl {
                    details: format!("MongoDB connection string error: {message}"),
                    url: url.to_owned(),
                },
                mongodb_client::ErrorKind::Other(err) => ErrorKind::ConnectionError(err.into()),
            };

            ConnectorError::from_kind(kind)
        })?;

        let database = client
            .default_database()
            .map(|d| d.name().to_owned())
            .unwrap_or_default();

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
