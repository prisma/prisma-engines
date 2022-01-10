#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::needless_collect)] // the implementation of that rule is way too eager, it rejects necessary collects

//! The top-level library crate for the migration engine.

pub mod commands;

mod api;
mod core_error;
mod rpc;

pub use self::{api::GenericApi, core_error::*, rpc::rpc_api};

pub use core_error::*;
pub use migration_connector;

use datamodel::{
    common::{
        preview_features::PreviewFeature,
        provider_names::{MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    },
    Datasource,
};
use enumflags2::BitFlags;
use std::env;
use user_facing_errors::{common::InvalidConnectionString, KnownError};

use datamodel::{schema_ast::ast::SchemaAst, ValidatedSchema};

fn parse_ast(schema: &str) -> CoreResult<SchemaAst> {
    datamodel::parse_schema_ast(schema)
        .map_err(|err| CoreError::new_schema_parser_error(err.to_pretty_string("schema.prisma", schema)))
}

fn parse_schema<'ast>(schema: &str, ast: &'ast SchemaAst) -> CoreResult<ValidatedSchema<'ast>> {
    datamodel::parse_schema_parserdb(schema, ast).map_err(CoreError::new_schema_parser_error)
}

#[cfg(feature = "mongodb")]
use datamodel::common::provider_names::MONGODB_SOURCE_NAME;
#[cfg(feature = "mongodb")]
use mongodb_migration_connector::MongoDbMigrationConnector;
#[cfg(feature = "sql")]
use sql_migration_connector::SqlMigrationConnector;

/// Top-level constructor for the migration engine API.
pub fn migration_api(datamodel: &str) -> CoreResult<Box<dyn api::GenericApi>> {
    let (source, url, preview_features, shadow_database_url) = parse_configuration(datamodel)?;

    match source.active_provider.as_str() {
        #[cfg(feature = "sql")]
        POSTGRES_SOURCE_NAME => {
            let mut u = url::Url::parse(&url).map_err(|err| {
                let details = user_facing_errors::quaint::invalid_connection_string_description(&format!(
                    "Error parsing connection string: {}",
                    err
                ));

                CoreError::from(KnownError::new(InvalidConnectionString { details }))
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
        MONGODB_SOURCE_NAME => Ok(Box::new(MongoDbMigrationConnector::new(url, preview_features))),
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
