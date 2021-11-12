use crate::schema::MongoSchema;
use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use futures::stream::TryStreamExt;
use migration_connector::{ConnectorError, ConnectorResult};
use mongodb::{
    bson::{Bson, Document},
    error::Error as MongoError,
    options::{ClientOptions, WriteConcern},
};
use url::Url;

/// The indexes MongoDB automatically creates for the object id in each collection.
const AUTOMATIC_ID_INDEX_NAME: &str = "_id_";

/// Abstraction over a mongodb connection (exposed for tests).
pub struct Client {
    inner: mongodb::Client,
    db_name: String,
}

impl Client {
    pub async fn connect(connection_str: &str) -> ConnectorResult<Client> {
        let url = Url::parse(connection_str).map_err(ConnectorError::url_parse_error)?;
        let db_name = url.path().trim_start_matches('/').to_string();

        let client_options = ClientOptions::parse(connection_str)
            .await
            .map_err(mongo_error_to_connector_error)?;

        let inner = mongodb::Client::with_options(client_options).map_err(mongo_error_to_connector_error)?;

        Ok(Client { inner, db_name })
    }

    pub(crate) fn database(&self) -> mongodb::Database {
        self.inner.database(&self.db_name)
    }

    pub(crate) async fn describe(&self, preview_features: BitFlags<PreviewFeature>) -> ConnectorResult<MongoSchema> {
        let mut schema = MongoSchema::default();
        let database = self.database();

        let mut cursor = database
            .list_collections(None, None)
            .await
            .map_err(mongo_error_to_connector_error)?;

        while let Some(collection) = cursor.try_next().await.map_err(mongo_error_to_connector_error)? {
            let collection_name = collection.name;
            let collection = database.collection::<Document>(&collection_name);
            let collection_id = schema.push_collection(collection_name);

            let mut indexes_cursor = collection
                .list_indexes(None)
                .await
                .map_err(mongo_error_to_connector_error)?;

            while let Some(index) = indexes_cursor
                .try_next()
                .await
                .map_err(mongo_error_to_connector_error)?
            {
                let options = index.options.unwrap();
                let name = options.name.unwrap();
                let is_unique = options.unique.unwrap_or(false); // 3-valued boolean where null means false

                if name == AUTOMATIC_ID_INDEX_NAME {
                    continue; // do not introspect or diff these
                }

                let path = if preview_features.contains(PreviewFeature::ExtendedIndexes) {
                    index.keys
                } else {
                    index.keys.iter().fold(Document::new(), |mut acc, (k, _)| {
                        acc.insert(k, Bson::Int32(1));
                        acc
                    })
                };

                schema.push_index(collection_id, name, is_unique, dbg!(path));
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
