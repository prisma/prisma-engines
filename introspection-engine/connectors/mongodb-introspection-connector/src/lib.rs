mod error;
mod sampler;
mod warnings;

pub use error::*;

use datamodel::{common::preview_features::PreviewFeature, Datamodel};
use futures::TryStreamExt;
use indoc::formatdoc;
use introspection_connector::{
    ConnectorError, ConnectorResult, DatabaseMetadata, ErrorKind, IntrospectionConnector, IntrospectionContext,
    IntrospectionResult,
};
use mongodb::{Client, Database};
use url::Url;
use user_facing_errors::{
    common::{InvalidConnectionString, UnsupportedFeatureError},
    KnownError,
};

#[derive(Debug)]
pub struct MongoDbIntrospectionConnector {
    connection: Client,
    database: String,
}

impl MongoDbIntrospectionConnector {
    pub async fn new(connection_string: &str) -> ConnectorResult<Self> {
        let url = Url::parse(connection_string).map_err(|err| {
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
        })?;

        let connection = Client::with_uri_str(connection_string)
            .await
            .map_err(|err| error::map_connection_errors(err, &url))?;

        let database = url.path().trim_start_matches('/').to_string();

        Ok(Self { connection, database })
    }

    fn database(&self) -> Database {
        self.connection.database(&self.database)
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
        Ok(Default::default())
    }

    async fn get_database_version(&self) -> ConnectorResult<String> {
        Ok(Default::default())
    }

    async fn introspect(
        &self,
        // TODO: Re-introspection.
        _existing_data_model: &Datamodel,
        ctx: IntrospectionContext,
    ) -> ConnectorResult<IntrospectionResult> {
        if !ctx.preview_features.contains(PreviewFeature::MongoDb) {
            let mut error = ConnectorError::from_kind(ErrorKind::PreviewFeatureNotEnabled(
                "MongoDB introspection connector (experimental feature, needs to be enabled)",
            ));

            error.user_facing_error = Some(KnownError::new(UnsupportedFeatureError {
                message: error.to_string(),
            }));

            return Err(error);
        }

        Ok(sampler::sample(self.database(), ctx.composite_type_depth).await?)
    }
}
