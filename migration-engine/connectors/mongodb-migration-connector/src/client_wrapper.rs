use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorResult};
use mongodb::{
    error::Error as MongoError,
    options::{ClientOptions, WriteConcern},
};
use mongodb_schema_describer::MongoSchema;
use url::Url;

/// Abstraction over a mongodb connection (exposed for tests).
pub struct Client {
    inner: mongodb::Client,
    db_name: String,
    preview_features: BitFlags<PreviewFeature>,
}

impl Client {
    pub async fn connect(connection_str: &str, preview_features: BitFlags<PreviewFeature>) -> ConnectorResult<Client> {
        let url = Url::parse(connection_str).map_err(ConnectorError::url_parse_error)?;
        let db_name = url.path().trim_start_matches('/').to_string();

        let client_options = ClientOptions::parse(connection_str)
            .await
            .map_err(mongo_error_to_connector_error)?;

        let inner = mongodb::Client::with_options(client_options).map_err(mongo_error_to_connector_error)?;

        Ok(Client {
            inner,
            db_name,
            preview_features,
        })
    }

    pub(crate) fn database(&self) -> mongodb::Database {
        self.inner.database(&self.db_name)
    }

    pub(crate) async fn describe(&self) -> ConnectorResult<MongoSchema> {
        let mut schema = mongodb_schema_describer::describe(&self.inner, &self.db_name)
            .await
            .map_err(mongo_error_to_connector_error)?;

        if !self.preview_features.contains(PreviewFeature::FullTextIndex) {
            schema.remove_fulltext_indexes();
        }

        if !self.preview_features.contains(PreviewFeature::ExtendedIndexes) {
            schema.normalize_index_attributes();
        }

        Ok(schema)
    }

    pub(crate) async fn drop_database(&self) -> ConnectorResult<()> {
        self.database()
            .drop(Some(
                mongodb::options::DropDatabaseOptions::builder()
                    .write_concern(WriteConcern::builder().journal(true).build())
                    .build(),
            ))
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
