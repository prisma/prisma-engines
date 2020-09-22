use super::SqlFlavour;
use crate::{catch, connect, database_info::DatabaseInfo, SqlError, SqlResult};
use futures::TryFutureExt;
use migration_connector::{ConnectorError, ConnectorResult, ErrorKind, MigrationDirectory};
use quaint::{connector::PostgresUrl, prelude::ConnectionInfo, prelude::Queryable, prelude::SqlFamily, single::Quaint};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{collections::HashMap, sync::Arc};
use url::Url;

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

        let (conn, _) = create_postgres_admin_conn(url.clone()).await?;

        let query = format!("CREATE DATABASE \"{}\"", db_name);

        let mut database_already_exists_error = None;

        match conn.raw_cmd(&query).map_err(SqlError::from).await {
            Ok(_) => (),
            Err(err @ SqlError::DatabaseAlreadyExists { .. }) => database_already_exists_error = Some(err),
            Err(err @ SqlError::UniqueConstraintViolation { .. }) => database_already_exists_error = Some(err),
            Err(err) => return Err(SqlError::from(err).into_connector_error(conn.connection_info())),
        };

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let (conn, _) = connect(&url.to_string()).await?;

        let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name());

        catch(
            conn.connection_info(),
            conn.raw_cmd(&schema_sql).map_err(SqlError::from),
        )
        .await?;

        if let Some(err) = database_already_exists_error {
            return Err(err.into_connector_error(conn.connection_info()));
        }

        Ok(db_name.to_owned())
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::postgres::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn ensure_connection_validity(&self, connection: &Quaint) -> ConnectorResult<()> {
        let schema_exists_result = catch(
            connection.connection_info(),
            connection
                .query_raw(
                    "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)",
                    &[connection.connection_info().schema_name().into()],
                )
                .map_err(SqlError::from),
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
            schema_name = connection.connection_info().schema_name(),
        );

        catch(
            connection.connection_info(),
            connection
                .raw_cmd(&format!(
                    "CREATE SCHEMA \"{}\"",
                    connection.connection_info().schema_name()
                ))
                .map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS _prisma_migrations (
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

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        strip_schema_param_from_url(&mut url);
        let (conn, _) = create_postgres_admin_conn(url.clone()).await?;
        let schema = self.0.schema();
        let db_name = self.0.dbname();

        let query = format!("CREATE DATABASE \"{}\"", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from))
            .await
            .ok();

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let (conn, _) = connect(&url.to_string()).await?;

        let drop_and_recreate_schema = format!(
            "DROP SCHEMA IF EXISTS \"{schema}\" CASCADE;\nCREATE SCHEMA \"{schema}\";",
            schema = schema
        );
        catch(
            conn.connection_info(),
            conn.raw_cmd(&drop_and_recreate_schema).map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn reset(&self, connection: &dyn Queryable, connection_info: &ConnectionInfo) -> ConnectorResult<()> {
        catch(
            connection_info,
            connection
                .raw_cmd(&format!("DROP SCHEMA \"{}\" CASCADE", connection_info.schema_name()))
                .map_err(SqlError::from),
        )
        .await?;

        catch(
            connection_info,
            connection
                .raw_cmd(&format!("CREATE SCHEMA \"{}\"", connection_info.schema_name()))
                .map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Postgres
    }

    #[tracing::instrument(skip(self, migrations, connection, connection_info))]
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<SqlSchema> {
        let database_name = format!("prisma_migrations_shadow_database_{}", uuid::Uuid::new_v4());
        let drop_database = format!("DROP DATABASE IF EXISTS \"{}\"", database_name);
        let create_database = format!("CREATE DATABASE \"{}\"", database_name);
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", self.schema_name());

        catch(
            connection_info,
            connection.raw_cmd(&drop_database).map_err(SqlError::from),
        )
        .await?;
        catch(
            connection_info,
            connection.raw_cmd(&create_database).map_err(SqlError::from),
        )
        .await?;

        let mut temporary_database_url = self.0.url().clone();
        temporary_database_url.set_path(&format!("/{}", database_name));
        let temporary_database_url = temporary_database_url.to_string();

        tracing::debug!("Connecting to temporary database at {}", temporary_database_url);

        let quaint = catch(
            connection_info,
            Quaint::new(&temporary_database_url).map_err(SqlError::from),
        )
        .await?;

        catch(
            quaint.connection_info(),
            quaint.raw_cmd(&create_schema).map_err(SqlError::from),
        )
        .await?;

        for migration in migrations {
            let script = migration
                .read_migration_script()
                .expect("failed to read migration script");

            tracing::debug!(
                "Applying migration `{}` to temporary database.",
                migration.migration_name()
            );

            catch(
                quaint.connection_info(),
                quaint.raw_cmd(&script).map_err(SqlError::from),
            )
            .await
            .map_err(|connector_error| connector_error.into_migration_failed(migration.migration_name().to_owned()))?
        }

        let sql_schema = catch(
            &quaint.connection_info().clone(),
            self.describe_schema(self.schema_name(), Arc::new(quaint)),
        )
        .await?;

        catch(
            connection_info,
            connection.raw_cmd(&drop_database).map_err(SqlError::from),
        )
        .await?;

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
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Quaint, DatabaseInfo)> {
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

    let conn = conn
        .ok_or_else(|| {
            ConnectorError::from_kind(ErrorKind::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            })
        })??;

    Ok(conn)
}
