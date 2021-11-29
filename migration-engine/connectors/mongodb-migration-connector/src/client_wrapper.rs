use crate::schema::MongoSchema;
use datamodel::{common::preview_features::PreviewFeature, IndexType};
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

                let tpe = match (options.unique, options.text_index_version.as_ref()) {
                    (Some(_), _) => IndexType::Unique,
                    (_, Some(_)) if preview_features.contains(PreviewFeature::FullTextIndex) => IndexType::Fulltext,
                    (_, Some(_)) => continue,
                    _ => IndexType::Normal,
                };

                if name == AUTOMATIC_ID_INDEX_NAME {
                    continue; // do not introspect or diff these
                }

                let path = if tpe.is_fulltext() {
                    let is_fts = |k: &str| k == "_fts" || k == "_ftsx";

                    // First we take all items that are not using the special fulltext keys,
                    // stopping when we find the first one.
                    let head = index
                        .keys
                        .iter()
                        .take_while(|(k, _)| !is_fts(k))
                        .map(|(k, v)| (k, v.clone()));

                    // Then go through the weights, we have the fields presented as part of the
                    // fulltext index here.
                    let middle = options
                        .weights
                        .iter()
                        .flat_map(|weights| weights.keys())
                        .map(|k| (k, Bson::String("text".to_string())));

                    // And in the end add whatever fields were left in the index keys that are not
                    // special fulltext keys.
                    let tail = index
                        .keys
                        .iter()
                        .skip_while(|(k, _)| !is_fts(k))
                        .skip_while(|(k, _)| is_fts(k))
                        .map(|(k, v)| (k, v.clone()));

                    head.chain(middle).chain(tail).fold(Document::new(), |mut acc, (k, v)| {
                        acc.insert(k, v);
                        acc
                    })
                } else {
                    index.keys
                };

                let path = if preview_features.contains(PreviewFeature::ExtendedIndexes) {
                    path
                } else {
                    path.iter().fold(Document::new(), |mut acc, (k, _)| {
                        acc.insert(k, Bson::Int32(1));
                        acc
                    })
                };

                schema.push_index(collection_id, name, tpe, path);
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
