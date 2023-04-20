//! Query Engine test setup.

#![allow(clippy::await_holding_lock)]

mod cockroachdb;
mod mongodb;
mod mssql;
mod mysql;
mod postgres;

pub use schema_core::schema_connector::ConnectorError;

use self::{cockroachdb::*, mongodb::*, mssql::*, mysql::*, postgres::*};
use enumflags2::BitFlags;
use psl::{builtin_connectors::*, Datasource};
use schema_core::{
    json_rpc::types::*,
    schema_connector::{BoxFuture, ConnectorResult},
};
use std::{env, sync::Arc};

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
            postgres_setup(url, prisma_schema, db_schemas).await?
        }
        provider if COCKROACH.is_provider(provider) => cockroach_setup(url, prisma_schema).await?,
        provider if MSSQL.is_provider(provider) => mssql_setup(url, prisma_schema, db_schemas).await?,
        provider if MYSQL.is_provider(provider) => {
            mysql_reset(&url).await?;
            diff_and_apply(prisma_schema).await;
        }
        provider if SQLITE.is_provider(provider) => {
            std::fs::remove_file(source.url.as_literal().unwrap().trim_start_matches("file:")).ok();
            diff_and_apply(prisma_schema).await;
        }

        provider if MONGODB.is_provider(provider) => mongo_setup(prisma_schema, &url).await?,

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
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

#[derive(Default)]
struct LoggingHost {
    printed: parking_lot::Mutex<Vec<String>>,
}

impl schema_core::schema_connector::ConnectorHost for LoggingHost {
    fn print(&self, text: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        let mut msgs = self.printed.lock();
        msgs.push(text.to_owned());
        Box::pin(std::future::ready(Ok(())))
    }
}

async fn diff_and_apply(schema: &str) {
    let tmpdir = tempfile::tempdir().unwrap();
    let host = Arc::new(LoggingHost::default());
    let api = schema_core::schema_api(Some(schema.to_owned()), Some(host.clone())).unwrap();
    let schema_file_path = tmpdir.path().join("schema.prisma");
    std::fs::write(&schema_file_path, schema).unwrap();

    // 2. create the database schema for given Prisma schema
    api.diff(DiffParams {
        exit_code: None,
        from: DiffTarget::Empty,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: schema_file_path.to_string_lossy().into(),
        }),
        script: true,
        shadow_database_url: None,
    })
    .await
    .unwrap();
    let migrations = host.printed.lock();
    let migration = migrations[0].clone();
    drop(migrations);

    api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemaContainer {
            schema: schema_file_path.to_string_lossy().into(),
        }),
        script: migration,
    })
    .await
    .unwrap();
}
