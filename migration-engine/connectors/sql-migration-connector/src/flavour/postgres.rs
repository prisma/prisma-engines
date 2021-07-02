use crate::{
    connect, connection_wrapper::Connection, error::quaint_error_to_connector_error, sql_renderer::IteratorJoin,
    SqlFlavour, SqlMigrationConnector,
};
use datamodel::common::preview_features::PreviewFeature;
use enumflags2::BitFlags;
use indoc::indoc;
use migration_connector::{migrations_directory::MigrationDirectory, ConnectorError, ConnectorResult};
use quaint::{
    connector::{tokio_postgres::error::ErrorPosition, PostgresUrl},
    error::ErrorKind as QuaintKind,
};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use std::collections::HashMap;
use url::Url;
use user_facing_errors::{
    common::{DatabaseAccessDenied, DatabaseDoesNotExist},
    introspection_engine::DatabaseSchemaInconsistent,
    migration_engine::{self, ApplyMigrationError},
    KnownError, UserFacingError,
};

const ADVISORY_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub(crate) struct PostgresFlavour {
    url: PostgresUrl,
    preview_features: BitFlags<PreviewFeature>,
}

impl std::fmt::Debug for PostgresFlavour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresFlavour").field("url", &"<REDACTED>").finish()
    }
}

impl PostgresFlavour {
    pub fn new(url: PostgresUrl, preview_features: BitFlags<PreviewFeature>) -> Self {
        Self { url, preview_features }
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.url.schema()
    }

    async fn shadow_database_connection(
        &self,
        main_connection: &Connection,
        connector: &SqlMigrationConnector,
        shadow_database_name: Option<String>,
    ) -> ConnectorResult<Connection> {
        if let Some(shadow_database_connection_string) = &connector.shadow_database_connection_string {
            let conn = crate::connect(shadow_database_connection_string).await?;
            let shadow_conninfo = conn.connection_info();
            let main_conninfo = main_connection.connection_info();

            super::validate_connection_infos_do_not_match((&shadow_conninfo, &main_conninfo))?;

            tracing::info!(
                "Connecting to user-provided shadow database at {}.{:?}",
                shadow_conninfo.host(),
                shadow_conninfo.dbname()
            );

            if self.reset(&conn).await.is_err() {
                connector.best_effort_reset(&conn).await?;
            }

            return Ok(conn);
        }

        let database_name = shadow_database_name.unwrap();
        let create_database = format!("CREATE DATABASE \"{}\"", database_name);
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", self.schema_name());

        main_connection
            .raw_cmd(&create_database)
            .await
            .map_err(ConnectorError::from)
            .map_err(|err| err.into_shadow_db_creation_error())?;

        let mut shadow_database_url = self.url.url().clone();
        shadow_database_url.set_path(&format!("/{}", database_name));
        let host = shadow_database_url.host();
        let shadow_database_url = shadow_database_url.to_string();

        tracing::debug!("Connecting to shadow database at {:?}/{}", host, database_name);

        let shadow_database_conn = crate::connect(&shadow_database_url).await?;

        shadow_database_conn.raw_cmd(&create_schema).await?;

        Ok(shadow_database_conn)
    }
}

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()> {
        // https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS

        // 72707369 is a unique number we chose to identify Migrate. It does not
        // have any meaning, but it should not be used by any other tool.
        tokio::time::timeout(
            ADVISORY_LOCK_TIMEOUT,
            connection.raw_cmd("SELECT pg_advisory_lock(72707369)"),
        )
        .await
        .map_err(|_elapsed| {
            ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
                database_host: connection.connection_info().host().to_owned(),
                database_port: connection
                    .connection_info()
                    .port()
                    .map(|port| port.to_string())
                    .unwrap_or_else(|| "<unknown>".into()),
                context: format!(
                    "Timed out trying to acquire a postgres advisory lock (SELECT pg_advisory_lock(72707369)). Elapsed: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.", ADVISORY_LOCK_TIMEOUT.as_millis()
                ),
            })
        })??;

        Ok(())
    }

    async fn run_query_script(&self, sql: &str, connection: &Connection) -> ConnectorResult<()> {
        Ok(connection.raw_cmd(sql).await?)
    }

    async fn apply_migration_script(
        &self,
        migration_name: &str,
        script: &str,
        conn: &Connection,
    ) -> ConnectorResult<()> {
        let (client, _url) = conn.unwrap_postgres();
        let inner_client = client.client();

        match inner_client.simple_query(script).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let (database_error_code, database_error): (Option<&str>, _) = if let Some(db_error) = err.as_db_error()
                {
                    let position = if let Some(ErrorPosition::Original(position)) = db_error.position() {
                        let mut previous_lines = [""; 5];
                        let mut byte_index = 0;
                        let mut error_position = String::new();

                        for (line_idx, line) in script.lines().enumerate() {
                            // Line numbers start at 1, not 0.
                            let line_number = line_idx + 1;
                            byte_index += line.len() + 1; // + 1 for the \n character.

                            if *position as usize <= byte_index {
                                let numbered_lines = previous_lines
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(idx, line)| {
                                        line_number
                                            .checked_sub(previous_lines.len() - idx)
                                            .map(|idx| (idx, line))
                                    })
                                    .map(|(idx, line)| {
                                        format!(
                                            "\x1b[1m{:>3}\x1b[0m{}{}",
                                            idx,
                                            if line.is_empty() { "" } else { " " },
                                            line
                                        )
                                    })
                                    .join("\n");

                                error_position = format!(
                                    "\n\nPosition:\n{}\n\x1b[1m{:>3}\x1b[1;31m {}\x1b[0m",
                                    numbered_lines, line_number, line
                                );
                                break;
                            } else {
                                previous_lines = [
                                    previous_lines[1],
                                    previous_lines[2],
                                    previous_lines[3],
                                    previous_lines[4],
                                    line,
                                ];
                            }
                        }

                        error_position
                    } else {
                        String::new()
                    };

                    let database_error = format!("{}{}\n\n{:?}", db_error, position, db_error);

                    (Some(db_error.code().code()), database_error)
                } else {
                    (err.code().map(|c| c.code()), err.to_string())
                };

                Err(ConnectorError::user_facing(ApplyMigrationError {
                    migration_name: migration_name.to_owned(),
                    database_error_code: database_error_code.unwrap_or("none").to_owned(),
                    database_error,
                }))
            }
        }
    }

    #[tracing::instrument(skip(database_str))]
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;
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
        sql_schema_describer::postgres::SqlSchemaDescriber::new(connection.queryable(), Default::default())
            .describe(connection.connection_info().schema_name())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    quaint_error_to_connector_error(err, &connection.connection_info())
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
        let mut url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;
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
        let schema_name = connection.schema_name();
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
        let mut url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;

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
        let schema_name = connection.schema_name();

        connection
            .raw_cmd(&format!("DROP SCHEMA \"{}\" CASCADE", schema_name))
            .await?;

        connection
            .raw_cmd(&format!("CREATE SCHEMA \"{}\"", schema_name))
            .await?;

        Ok(())
    }

    fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }

    #[tracing::instrument(skip(self, migrations, connection, connector))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
        connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema> {
        let shadow_database_name = connector.shadow_database_name();

        // We go through the whole process without early return, then clean up
        // the shadow database, and only then return the result. This avoids
        // leaving shadow databases behind in case of e.g. faulty migrations.

        let sql_schema_result = (|| {
            async {
                let shadow_database = self
                    .shadow_database_connection(connection, connector, shadow_database_name.clone())
                    .await?;

                for migration in migrations {
                    let script = migration.read_migration_script()?;

                    tracing::debug!(
                        "Applying migration `{}` to shadow database.",
                        migration.migration_name()
                    );

                    shadow_database
                        .raw_cmd(&script)
                        .await
                        .map_err(ConnectorError::from)
                        .map_err(|connector_error| {
                            connector_error.into_migration_does_not_apply_cleanly(migration.migration_name().to_owned())
                        })?;
                }

                // The connection to the shadow database is dropped at the end of
                // the block.
                self.describe_schema(&shadow_database).await
            }
        })()
        .await;

        if let Some(shadow_database_name) = shadow_database_name {
            let drop_database = format!("DROP DATABASE IF EXISTS \"{}\"", shadow_database_name);
            connection.raw_cmd(&drop_database).await?;
        }

        sql_schema_result
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
                Some(DatabaseAccessDenied::ERROR_CODE) => (),
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
        ConnectorError::user_facing(migration_engine::DatabaseCreationFailed { database_error: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned() })
    })??;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "postgresql://myname:mypassword@myserver:8765/mydbname";

        let flavour = PostgresFlavour::new(PostgresUrl::new(url.parse().unwrap()).unwrap(), BitFlags::empty());
        let debugged = format!("{:?}", flavour);

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }
}
