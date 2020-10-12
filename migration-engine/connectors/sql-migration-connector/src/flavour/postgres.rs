use super::SqlFlavour;
use crate::{connect, connection_wrapper::Connection};
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory};
use quaint::{connector::PostgresUrl, error::ErrorKind as QuaintKind};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend, SqlSchemaDescriberError};
use std::collections::HashMap;
use url::Url;
use user_facing_errors::migration_engine;

#[derive(Debug)]
pub(crate) struct PostgresFlavour(pub(crate) PostgresUrl);

impl PostgresFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.schema()
    }
}

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = self.0.dbname();

        strip_schema_param_from_url(&mut url);

        let conn = create_postgres_admin_conn(url.clone()).await?;

        let query = format!("CREATE DATABASE \"{}\"", db_name);

        let mut database_already_exists_error = None;

        match conn.raw_cmd(&query).await {
            Ok(_) => (),
            Err(err) if matches!(err.kind(), QuaintKind::DatabaseAlreadyExists { .. }) => {
                database_already_exists_error = Some(err)
            }
            Err(err) if matches!(err.kind(), QuaintKind::UniqueConstraintViolation { .. }) => {
                database_already_exists_error = Some(err)
            }
            Err(err) => return Err(err.into()),
        };

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let conn = connect(&url.to_string()).await?;

        let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name());

        conn.raw_cmd(&schema_sql).await?;

        if let Some(err) = database_already_exists_error {
            return Err(err.into());
        }

        Ok(db_name.to_owned())
    }

    async fn create_imperative_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             TIMESTAMPTZ,
                migration_name          TEXT NOT NULL,
                logs                    TEXT NOT NULL,
                rolled_back_at          TIMESTAMPTZ,
                started_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
                applied_steps_count     INTEGER NOT NULL DEFAULT 0,
                script                  TEXT NOT NULL
            );
        "#;

        Ok(connection.raw_cmd(sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::postgres::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err {
                SqlSchemaDescriberError::UnknownError => {
                    ConnectorError::query_error(anyhow::anyhow!("An unknown error occurred in sql-schema-describer"))
                }
            })
    }

    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()> {
        let schema_name = connection.connection_info().schema_name();
        let schema_exists_result = connection
            .query_raw(
                "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)",
                &[schema_name.into()],
            )
            .await?;

        if let Some(true) = schema_exists_result
            .get(0)
            .and_then(|row| row.at(0).and_then(|value| value.as_bool()))
        {
            return Ok(());
        }

        tracing::debug!(
            "Detected that the `{schema_name}` schema does not exist on the target database. Attempting to create it.",
            schema_name = schema_name,
        );

        connection
            .raw_cmd(&format!("CREATE SCHEMA \"{}\"", schema_name))
            .await?;

        Ok(())
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        strip_schema_param_from_url(&mut url);
        let conn = create_postgres_admin_conn(url.clone()).await?;
        let schema = self.0.schema();
        let db_name = self.0.dbname();

        let query = format!("CREATE DATABASE \"{}\"", db_name);
        conn.raw_cmd(&query).await.ok();

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let conn = connect(&url.to_string()).await?;

        let drop_and_recreate_schema = format!(
            "DROP SCHEMA IF EXISTS \"{schema}\" CASCADE;\nCREATE SCHEMA \"{schema}\";",
            schema = schema
        );
        conn.raw_cmd(&drop_and_recreate_schema).await?;

        Ok(())
    }

    async fn reset(&self, connection: &Connection) -> ConnectorResult<()> {
        let schema_name = connection.connection_info().schema_name();

        connection
            .raw_cmd(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
            .await?;

        connection
            .raw_cmd(&format!("CREATE SCHEMA \"{}\"", schema_name))
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self, migrations, connection))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
    ) -> ConnectorResult<SqlSchema> {
        let database_name = format!("prisma_migrations_shadow_database_{}", uuid::Uuid::new_v4());
        let drop_database = format!("DROP DATABASE IF EXISTS \"{}\"", database_name);
        let create_database = format!("CREATE DATABASE \"{}\"", database_name);
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", self.schema_name());

        connection.raw_cmd(&drop_database).await?;
        connection.raw_cmd(&create_database).await?;

        let mut temporary_database_url = self.0.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {}", temporary_database_url);

        let sql_schema = {
            let temporary_database = crate::connect(&temporary_database_url).await?;

            temporary_database.raw_cmd(&create_schema).await?;

            for migration in migrations {
                let script = migration.read_migration_script()?;

                tracing::debug!(
                    "Applying migration `{}` to temporary database.",
                    migration.migration_name()
                );

                temporary_database
                    .raw_cmd(&script)
                    .await
                    .map_err(ConnectorError::from)
                    .map_err(|connector_error| {
                        connector_error.into_migration_failed(migration.migration_name().to_owned())
                    })?;
            }

            // the connection to the temporary database is dropped at the end of
            // the block.
            self.describe_schema(&temporary_database).await?
        };

        connection.raw_cmd(&drop_database).await?;

        Ok(sql_schema)
    }
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<Connection> {
    let candidate_default_databases = &["postgres", "template1"];

    let mut conn = None;

    for database_name in candidate_default_databases {
        url.set_path(&format!("/{}", database_name));
        match connect(url.as_str()).await {
            // If the database does not exist, try the next one.
            Err(err) => match &err.kind {
                migration_connector::ErrorKind::DatabaseDoesNotExist { .. } => (),
                _other_outcome => {
                    conn = Some(Err(err));
                    break;
                }
            },
            // If the outcome is anything else, use this.
            other_outcome => {
                conn = Some(other_outcome);
                break;
            }
        }
    }

    let conn = conn.ok_or_else(|| {
        ConnectorError::user_facing_error(migration_engine::DatabaseCreationFailed { database_error: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned() })
    })??;

    Ok(conn)
}
