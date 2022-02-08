//! Query Engine test setup.

mod mongodb;
mod mssql;
mod mysql;
mod postgres;

pub use migration_core::migration_connector::ConnectorError;

use self::{mongodb::*, mssql::*, mysql::*, postgres::*};
use datamodel::{
    common::{
        preview_features::*,
        provider_names::{
            COCKROACHDB_SOURCE_NAME, MONGODB_SOURCE_NAME, MSSQL_SOURCE_NAME, MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME,
            SQLITE_SOURCE_NAME,
        },
    },
    Datasource,
};
use enumflags2::BitFlags;
use migration_core::{
    json_rpc::types::*,
    migration_connector::{BoxFuture, ConnectorResult},
};
use std::{env, sync::Arc};

fn parse_configuration(datamodel: &str) -> ConnectorResult<(Datasource, String, BitFlags<PreviewFeature>)> {
    let config = datamodel::parse_configuration(datamodel)
        .map(|validated_config| validated_config.subject)
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
pub async fn setup(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, _preview_features) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES_SOURCE_NAME, COCKROACHDB_SOURCE_NAME].contains(&provider.as_str()) => {
            postgres_setup(url, prisma_schema).await?
        }
        provider if [MSSQL_SOURCE_NAME].contains(&provider.as_str()) => mssql_setup(url, prisma_schema).await?,
        provider if [MYSQL_SOURCE_NAME].contains(&provider.as_str()) => {
            mysql_reset(&url).await?;
            diff_and_apply(prisma_schema).await;
        }
        provider if [SQLITE_SOURCE_NAME].contains(&provider.as_str()) => {
            // 1. creates schema & database
            let api = migration_core::migration_api(Some(prisma_schema.to_owned()), None)?;
            api.drop_database(url).await.ok();
            api.create_database(CreateDatabaseParams {
                datasource: DatasourceParam::SchemaString(SchemaContainer {
                    schema: prisma_schema.to_owned(),
                }),
            })
            .await?;

            // 2. create the database schema for given Prisma schema
            diff_and_apply(prisma_schema).await;
        }

        provider if provider == MONGODB_SOURCE_NAME => mongo_setup(prisma_schema, &url).await?,

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

/// Database teardown for connector-test-kit-rs.
pub async fn teardown(prisma_schema: &str) -> ConnectorResult<()> {
    let (source, url, _) = parse_configuration(prisma_schema)?;

    match &source.active_provider {
        provider if [POSTGRES_SOURCE_NAME, COCKROACHDB_SOURCE_NAME].contains(&provider.as_str()) => {
            postgres_teardown(&url).await?;
        }

        provider
            if [
                SQLITE_SOURCE_NAME,
                MSSQL_SOURCE_NAME,
                MYSQL_SOURCE_NAME,
                MONGODB_SOURCE_NAME,
            ]
            .contains(&provider.as_str()) => {}

        x => unimplemented!("Connector {} is not supported yet", x),
    };

    Ok(())
}

#[derive(Default)]
struct LoggingHost {
    printed: std::sync::Mutex<Vec<String>>,
}

impl migration_core::migration_connector::ConnectorHost for LoggingHost {
    fn print(&self, text: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        let mut msgs = self.printed.lock().unwrap();
        msgs.push(text.to_owned());
        Box::pin(std::future::ready(Ok(())))
    }
}

async fn diff_and_apply(schema: &str) {
    let tmpdir = tempfile::tempdir().unwrap();
    let host = Arc::new(LoggingHost::default());
    let api = migration_core::migration_api(Some(schema.to_owned()), Some(host.clone())).unwrap();
    let schema_file_path = tmpdir.path().join("schema.prisma");
    std::fs::write(&schema_file_path, schema).unwrap();

    // 2. create the database schema for given Prisma schema
    api.diff(DiffParams {
        from: DiffTarget::Empty,
        to: DiffTarget::SchemaDatamodel(SchemaContainer {
            schema: schema_file_path.to_string_lossy().into(),
        }),
        script: true,
        shadow_database_url: None,
    })
    .await
    .unwrap();
    let migrations = host.printed.lock().unwrap();
    let migration = &migrations[0];

    api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemaContainer {
            schema: schema_file_path.to_string_lossy().into(),
        }),
        script: migration.to_owned(),
    })
    .await
    .unwrap();
}
