use enumflags2::BitFlags;
use mongodb::{error::Error as MongoError, options::WriteConcern};
use mongodb_client::MongoConnectionString;
use mongodb_schema_describer::MongoSchema;
use psl::PreviewFeature;
use schema_connector::{ConnectorError, ConnectorResult};

/// Abstraction over a mongodb connection (exposed for tests).
pub struct Client {
    inner: mongodb::Client,
    db_name: String,
}

impl Client {
    pub async fn connect(connection_str: &str, _preview_features: BitFlags<PreviewFeature>) -> ConnectorResult<Client> {
        let MongoConnectionString { database, .. } = connection_str.parse().map_err(ConnectorError::url_parse_error)?;

        let inner = mongodb_client::create(connection_str)
            .await
            .map_err(|e| match &e.kind {
                mongodb_client::ErrorKind::InvalidArgument { .. } => ConnectorError::url_parse_error(e),
                mongodb_client::ErrorKind::Other(e) => mongo_error_to_connector_error(e.clone()),
            })?;

        Ok(Client {
            inner,
            db_name: database,
        })
    }

    pub(crate) fn database(&self) -> mongodb::Database {
        self.inner.database(&self.db_name)
    }

    pub(crate) async fn describe(&self) -> ConnectorResult<MongoSchema> {
        let mut schema = mongodb_schema_describer::describe(&self.inner, &self.db_name)
            .await
            .map_err(mongo_error_to_connector_error)?;

        schema.remove_fulltext_indexes();

        Ok(schema)
    }

    pub(crate) async fn drop_database(&self) -> ConnectorResult<()> {
        self.database()
            .drop()
            .with_options(
                mongodb::options::DropDatabaseOptions::builder()
                    .write_concern(WriteConcern::builder().journal(true).build())
                    .build(),
            )
            .await
            .map_err(mongo_error_to_connector_error)
    }

    pub(crate) fn db_name(&self) -> &str {
        &self.db_name
    }
}

pub(crate) fn mongo_error_to_connector_error(mongo_error: MongoError) -> ConnectorError {
    ConnectorError::from_source(mongo_error, "MongoDB error")
}
