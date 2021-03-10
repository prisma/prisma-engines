use crate::{
    connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, SqlFlavour, SqlMigrationConnector,
};
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{ConnectorError, ConnectorResult, MigrationDirectory, MigrationFeature};
use quaint::{connector::PostgresUrl, error::ErrorKind as QuaintKind};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::collections::HashMap;
use url::Url;
use user_facing_errors::{
    common::DatabaseDoesNotExist, introspection_engine::DatabaseSchemaInconsistent, migration_engine, KnownError,
    UserFacingError,
};

const ADVISORY_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug)]
pub(crate) struct PostgresFlavour {
    pub(crate) url: PostgresUrl,
    features: BitFlags<MigrationFeature>,
}

impl PostgresFlavour {
    pub fn new(url: PostgresUrl, features: BitFlags<MigrationFeature>) -> Self {
        Self { url, features }
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.url.schema()
    }

    async fn shadow_database_connection(
        &self,
        main_connection: &Connection,
        connector: &SqlMigrationConnector,
        temporary_database_name: Option<String>,
    ) -> ConnectorResult<Connection> {
        if let Some(shadow_database_connection_string) = &connector.shadow_database_connection_string {
            let conn = crate::connect(shadow_database_connection_string).await?;
            let shadow_conninfo = conn.connection_info();
            let main_conninfo = main_connection.connection_info();

            if shadow_conninfo.host() == main_conninfo.host() && shadow_conninfo.dbname() == main_conninfo.dbname() {
                return Err(ConnectorError::from_message("The shadow database you configured appears to be the same as as the main database. Please specify another shadow database.".into()));
            }

            tracing::info!(
                "Connecting to user-provided shadow database at {}",
                shadow_database_connection_string
            );

            if self.reset(&conn).await.is_err() {
                connector.best_effort_reset(&conn).await?;
            }

            return Ok(conn);
        }

        let database_name = temporary_database_name.unwrap();
        let create_database = format!("CREATE DATABASE \"{}\"", database_name);
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", self.schema_name());

        main_connection
            .raw_cmd(&create_database)
            .await
            .map_err(ConnectorError::from)
            .map_err(|err| err.into_shadow_db_creation_error())?;

        let mut temporary_database_url = self.url.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {}", temporary_database_url);

        let temporary_database_conn = crate::connect(&temporary_database_url).await?;

        temporary_database_conn.raw_cmd(&create_schema).await?;

        Ok(temporary_database_conn)
    }
}

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()> {
        // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS

        tokio::time::timeout(
            ADVISORY_LOCK_TIMEOUT,
            connection.raw_cmd("SELECT pg_advisory_lock(72707369)"),
        )
        .await
        .map_err(|_elapsed| {
            ConnectorError::user_facing_error(user_facing_errors::common::DatabaseTimeout {
                database_host: connection.connection_info().host().to_owned(),
                database_port: connection
                    .connection_info()
                    .port()
                    .map(|port| port.to_string())
                    .unwrap_or_else(|| "<unknown>".into()),
            })
        })??;

        Ok(())
    }

    #[tracing::instrument(skip(database_str))]
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = self.url.dbname();

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

    async fn create_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        let sql = indoc! {r#"
            CREATE TABLE _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             TIMESTAMPTZ,
                migration_name          VARCHAR(255) NOT NULL,
                logs                    TEXT,
                rolled_back_at          TIMESTAMPTZ,
                started_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
                applied_steps_count     INTEGER NOT NULL DEFAULT 0
            );
        "#};

        Ok(connection.raw_cmd(sql).await?)
    }

    async fn describe_schema<'a>(&'a self, connection: &Connection) -> ConnectorResult<SqlSchema> {
        sql_schema_describer::postgres::SqlSchemaDescriber::new(connection.quaint().clone())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, connection.connection_info())
                }
                e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    let err = KnownError::new(DatabaseSchemaInconsistent {
                        explanation: format!("{}", e),
                    });

                    ConnectorError::from(err)
                }
            })
    }

    async fn drop_database(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = url.path().trim_start_matches('/').to_owned();
        assert!(!db_name.is_empty(), "Database name should not be empty.");

        strip_schema_param_from_url(&mut url);
        let conn = create_postgres_admin_conn(url.clone()).await?;

        conn.raw_cmd(&format!("DROP DATABASE \"{}\"", db_name)).await?;

        Ok(())
    }

    async fn drop_migrations_table(&self, connection: &Connection) -> ConnectorResult<()> {
        connection.raw_cmd("DROP TABLE _prisma_migrations").await?;

        Ok(())
    }

    #[tracing::instrument]
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
        let schema = self.url.schema();
        let db_name = self.url.dbname();

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

    #[tracing::instrument(skip(self, migrations, connection, connector))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
        connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema> {
        let temporary_database_name = connector.temporary_database_name();

        // We go through the whole process without early return, then clean up
        // the temporary database, and only then return the result. This avoids
        // leaving shadow databases behind in case of e.g. faulty migrations.

        let sql_schema_result = (|| {
            async {
                let temporary_database = self
                    .shadow_database_connection(connection, connector, temporary_database_name.clone())
                    .await?;

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
                            connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                        })?;
                }

                // the connection to the temporary database is dropped at the end of
                // the block.
                self.describe_schema(&temporary_database).await
            }
        })()
        .await;

        if let Some(temporary_database_name) = temporary_database_name {
            let drop_database = format!("DROP DATABASE IF EXISTS \"{}\"", temporary_database_name);
            connection.raw_cmd(&drop_database).await?;
        }

        sql_schema_result
    }

    fn features(&self) -> BitFlags<MigrationFeature> {
        self.features
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
    // "postgres" is the default database on most postgres installations,
    // "template1" is guaranteed to exist, and "defaultdb" is the only working
    // option on DigitalOcean managed postgres databases.
    const CANDIDATE_DEFAULT_DATABASES: &[&str] = &["postgres", "template1", "defaultdb"];

    let mut conn = None;

    for database_name in CANDIDATE_DEFAULT_DATABASES {
        url.set_path(&format!("/{}", database_name));
        match connect(url.as_str()).await {
            // If the database does not exist, try the next one.
            Err(err) => match &err.error_code() {
                Some(DatabaseDoesNotExist::ERROR_CODE) => (),
                _ => {
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
