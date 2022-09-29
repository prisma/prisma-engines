mod error;
mod sampler;
mod warnings;

pub use error::*;

use enumflags2::BitFlags;
use futures::TryStreamExt;
use indoc::formatdoc;
use introspection_connector::{
    ConnectorError, ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionContext,
    IntrospectionResult,
};
use mongodb::{Client, Database};
use mongodb_schema_describer::MongoSchema;
use psl::common::preview_features::PreviewFeature;
use user_facing_errors::{common::InvalidConnectionString, KnownError};

#[derive(Debug)]
pub struct MongoDbIntrospectionConnector {
    connection: Client,
    database: String,
}

impl MongoDbIntrospectionConnector {
    pub async fn new(connection_string: &str) -> ConnectorResult<Self> {
        let error_f = |err: mongodb_client::Error| {
            let docs = r#"https://www.prisma.io/docs/reference/database-reference/connection-urls"#;

            let details = formatdoc!(
                r#"
                    {} in database URL. Please refer to the documentation in {} for constructing a correct
                    connection string. In some cases, certain characters must be escaped. Please
                    check the string for any illegal characters."#,
                err,
                docs
            )
            .replace('\n', " ");

            let known = KnownError::new(InvalidConnectionString { details });

            ConnectorError {
                user_facing_error: Some(known),
                kind: introspection_connector::ErrorKind::InvalidDatabaseUrl(format!("{} in database URL", err)),
            }
        };

        let url = connection_string.parse().map_err(error_f)?;

        let connection = mongodb_client::create(connection_string)
            .await
            .map_err(|err| match err.kind {
                mongodb_client::ErrorKind::InvalidArgument { .. } => error_f(err),
                mongodb_client::ErrorKind::Other(err) => error::map_connection_errors(err, &url),
            })?;

        Ok(Self {
            connection,
            database: url.database,
        })
    }

    fn database(&self) -> Database {
        self.connection.database(&self.database)
    }

    async fn describe(&self, preview_features: BitFlags<PreviewFeature>) -> ConnectorResult<MongoSchema> {
        let mut schema = mongodb_schema_describer::describe(&self.connection, &self.database)
            .await
            .map_err(crate::Error::from)?;

        if !preview_features.contains(PreviewFeature::FullTextIndex) {
            schema.remove_fulltext_indexes();
        }

        Ok(schema)
    }

    async fn version(&self) -> ConnectorResult<String> {
        let version = mongodb_schema_describer::version(&self.connection, &self.database)
            .await
            .map_err(crate::Error::from)?;

        Ok(version)
    }
}

#[async_trait::async_trait]
impl IntrospectionConnector for MongoDbIntrospectionConnector {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>> {
        let names = self
            .connection
            .list_database_names(None, None)
            .await
            .map_err(Error::from)?;

        Ok(names)
    }

    async fn get_metadata(&self) -> ConnectorResult<DatabaseMetadata> {
        let collections: Vec<_> = self
            .database()
            .list_collections(None, None)
            .await
            .map_err(Error::from)?
            .try_collect()
            .await
            .map_err(Error::from)?;

        Ok(DatabaseMetadata {
            table_count: collections.len(),
            size_in_bytes: 0,
        })
    }

    async fn get_database_description(&self) -> ConnectorResult<String> {
        let mongo_schema = self.describe(BitFlags::all()).await?;
        let description = serde_json::to_string_pretty(&mongo_schema).unwrap();
        Ok(description)
    }

    async fn get_database_version(&self) -> ConnectorResult<String> {
        Ok(self.version().await.unwrap())
    }

    async fn introspect(&self, ctx: &IntrospectionContext) -> ConnectorResult<IntrospectionResult> {
        let schema = self.describe(ctx.preview_features).await?;
        Ok(sampler::sample(self.database(), ctx.composite_type_depth, schema).await?)
    }
}
