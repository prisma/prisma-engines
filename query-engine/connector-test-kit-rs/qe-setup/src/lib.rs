//! Query Engine test setup.

#![allow(clippy::await_holding_lock)]

mod cockroachdb;
pub mod driver_adapters;
mod mongodb;
mod mssql;
mod mysql;
mod postgres;

pub use schema_core::schema_connector::ConnectorError;

use self::{cockroachdb::*, mongodb::*, mssql::*, mysql::*, postgres::*};
use enumflags2::BitFlags;
use psl::{builtin_connectors::*, Datasource};
use schema_core::schema_connector::{ConnectorResult, DiffTarget, SchemaConnector};
use std::env;

fn parse_configuration(datamodel: &str) -> ConnectorResult<(Datasource, String, BitFlags<psl::PreviewFeature>)> {
    let config = psl::parse_configuration(datamodel)
        .map_err(|err| ConnectorError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let url = config.datasources[0]
        .load_url(|key| env::var(key).ok())
        .map_err(|err| ConnectorError::new_schema_parser_error(err.to_pretty_string("schema.prisma", datamodel)))?;

    let preview_features = config.preview_features();

    let source = config
        .datasources
        .into_iter()
        .next()
        .ok_or_else(|| ConnectorError::from_msg("There is no datasource in the schema.".into()))?;

    Ok((source, url, preview_features))
}

/// Database setup for connector-test-kit-rs.
pub async fn setup(prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let (source, url, _preview_features) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES.provider_name()].contains(provider) => {
            postgres_setup(url, prisma_schema, db_schemas).await
        }
        provider if COCKROACH.is_provider(provider) => cockroach_setup(url, prisma_schema).await,
        provider if MSSQL.is_provider(provider) => mssql_setup(url, prisma_schema, db_schemas).await,
        provider if MYSQL.is_provider(provider) => {
            mysql_reset(&url).await?;
            let mut connector = sql_schema_connector::SqlSchemaConnector::new_mysql();
            diff_and_apply(prisma_schema, url, &mut connector).await
        }
        provider if SQLITE.is_provider(provider) => {
            std::fs::remove_file(source.url.as_literal().unwrap().trim_start_matches("file:")).ok();
            let mut connector = sql_schema_connector::SqlSchemaConnector::new_sqlite();
            diff_and_apply(prisma_schema, url, &mut connector).await
        }

        provider if MONGODB.is_provider(provider) => mongo_setup(prisma_schema, &url).await,

        x => unimplemented!("Connector {} is not supported yet", x),
    }
}

/// Database teardown for connector-test-kit-rs.
pub async fn teardown(prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let (source, url, _) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES.provider_name()].contains(provider) => {
            postgres_teardown(&url, db_schemas).await?;
        }

        provider
            if [
                SQLITE.provider_name(),
                MSSQL.provider_name(),
                MYSQL.provider_name(),
                MONGODB.provider_name(),
                COCKROACH.provider_name(),
            ]
            .contains(provider) => {}

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

async fn diff_and_apply(schema: &str, url: String, connector: &mut dyn SchemaConnector) -> ConnectorResult<()> {
    connector.set_params(schema_core::schema_connector::ConnectorParams {
        connection_string: url,
        preview_features: Default::default(),
        shadow_database_connection_string: None,
    })?;
    let from = connector
        .database_schema_from_diff_target(DiffTarget::Empty, None, None)
        .await?;
    let to = connector
        .database_schema_from_diff_target(DiffTarget::Datamodel(schema.into()), None, None)
        .await?;
    let migration = connector.diff(from, to);
    let script = connector.render_script(&migration, &Default::default()).unwrap();
    connector.db_execute(script).await
}
