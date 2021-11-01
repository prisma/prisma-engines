mod rpc;

pub mod qe_setup;

pub use crate::api::GenericApi;
pub use rpc::rpc_api;

use crate::{api, CoreError, CoreResult};
use datamodel::{
    common::{
        preview_features::PreviewFeature,
        provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    },
    Datasource,
};
use enumflags2::BitFlags;
use migration_connector::ConnectorError;
use sql_migration_connector::SqlMigrationConnector;
use std::env;
use user_facing_errors::{common::InvalidConnectionString, KnownError};

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;

/// Top-level constructor for the migration engine API.
pub async fn migration_api(datamodel: &str) -> CoreResult<Box<dyn api::GenericApi>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration(datamodel)?;

    match source.active_provider.as_str() {
        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => {
            let mut u = url::Url::parse(&url).map_err(|err| {
                let details = user_facing_errors::quaint::invalid_connection_string_description(&format!(
                    "Error parsing connection string: {}",
                    err
                ));

                ConnectorError::from(KnownError::new(InvalidConnectionString { details }))
            })?;

            let params: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

            u.query_pairs_mut().clear();

            for (k, v) in params.into_iter() {
                if k == "statement_cache_size" {
                    u.query_pairs_mut().append_pair("statement_cache_size", "0");
                } else {
                    u.query_pairs_mut().append_pair(&k, &v);
                }
            }

            if !u.query_pairs().any(|(k, _)| k == "statement_cache_size") {
                u.query_pairs_mut().append_pair("statement_cache_size", "0");
            }

            let connector = SqlMigrationConnector::new(u.to_string(), preview_features, shadow_database_url)?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "sql")]
        MYSQL_SOURCE_NAME | SQLITE_SOURCE_NAME | MSSQL_SOURCE_NAME => {
            let connector = SqlMigrationConnector::new(url, preview_features, shadow_database_url)?;

            Ok(Box::new(connector))
        }
        #[cfg(feature = "mongodb")]
        MONGODB_SOURCE_NAME => Ok(Box::new(MongoDbMigrationConnector::new(url))),
        provider => Err(CoreError::from_msg(format!(
            "`{}` is not a supported connector.",
            provider
        ))),
    }
}

fn parse_configuration(datamodel: &str) -> CoreResult<(Datasource, String, BitFlags<PreviewFeature>, Option<String>)> {
    let config = datamodel::parse_configuration(datamodel)
        .map(|validated_config| validated_config.subject)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let url = config.datasources[0]
        .load_url(|key| env::var(key).ok())
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let shadow_database_url = config.datasources[0]
        .load_shadow_database_url()
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, url, preview_features, shadow_database_url))
}
