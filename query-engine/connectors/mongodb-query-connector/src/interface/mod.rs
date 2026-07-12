mod connection;
mod transaction;
mod utils;

pub use connection::*;
pub use transaction::*;

use async_trait::async_trait;
use connector_interface::{
    Connector,
    error::{ConnectorError, ErrorKind},
};
use futures::Future;
use mongodb::Client;
use psl::Datasource;
use query_structure::prelude::*;

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
            .start_session()
            .await
            .map_err(|err| MongoError::from(err).into_connector_error())?;

        Ok(Box::new(MongoDbConnection {
            session,
            database: self.client.database(&self.database),
        }))
    }

    fn name(&self) -> &'static str {
        "mongodb"
    }

    fn should_retry_on_transient_error(&self) -> bool {
        true
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

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn test_schema() -> String {
        indoc!(
            r#"
            datasource db {
              provider = "mongodb"
            }

            model User {
              id    String @id @map("_id") @default(auto()) @db.ObjectId
            }
            "#
        )
        .into()
    }

    async fn mongodb_connector(url: &str) -> connector_interface::Result<MongoDb> {
        let schema = psl::validate_without_extensions(test_schema().into());
        let datasource = &schema.configuration.datasources[0];
        MongoDb::new(datasource, url).await
    }

    /// Regression test for https://github.com/prisma/prisma/issues/13388
    #[tokio::test]
    async fn test_error_details_forwarding_srv_port() {
        let url = "mongodb+srv://root:example@localhost:27017/myDatabase";
        let error = mongodb_connector(url).await.err().unwrap();

        assert!(
            error
                .to_string()
                .contains("a port cannot be specified with 'mongodb+srv'")
        );
    }

    /// Regression test for https://github.com/prisma/prisma/issues/11883
    #[tokio::test]
    async fn test_error_details_forwarding_illegal_characters() {
        let url = "mongodb://localhost:C2y6yDjf5/R+ob0N8A7Cgv30VRDJIWEHLM+4QDU5DE2nQ9nDuVTqobD4b8mGGyPMbIZnqyMsEcaGQy67XIw/Jw==@localhost:10255/e2e-tests?ssl=true";
        let error = mongodb_connector(url).await.err().unwrap();

        assert!(error.to_string().contains("password must be URL encoded"));
    }
}
