use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorResult};
use mongodb::{
    error::Error as MongoError,
    options::{ClientOptions, WriteConcern},
};
use mongodb_schema_describer::{IndexData, IndexFieldProperty, MongoSchema};
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
            #[allow(clippy::needless_collect)] // well, mr. clippy, maybe you should read about the borrow checker...
            let kept_indexes: Vec<_> = schema.drain_indexes().filter(|i| !i.is_fulltext()).collect();

            for index in kept_indexes.into_iter() {
                let IndexData {
                    name,
                    r#type,
                    fields,
                    collection_id,
                } = index;

                // because this here is a mutable reference, so we must collect...
                schema.push_index(collection_id, name, r#type, fields);
            }
        }

        if !self.preview_features.contains(PreviewFeature::ExtendedIndexes) {
            for field in schema.walk_indexes_mut().flat_map(|i| i.fields.iter_mut()) {
                field.property = IndexFieldProperty::Ascending;
            }
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
}

pub(crate) fn mongo_error_to_connector_error(mongo_error: MongoError) -> ConnectorError {
    ConnectorError::from_source(mongo_error, "MongoDB error")
}
