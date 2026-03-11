mod connector;
mod destructive_change_checker;
mod renderer;
mod schema_calculator;
mod schema_differ;

use crate::{SqlConnector, sql_destructive_change_checker, sql_renderer::SqlRenderer};
use connection_string::JdbcString;
#[cfg(feature = "mssql-native")]
use connector::{Connection, generic_apply_migration_script, shadow_db};
#[cfg(not(feature = "mssql-native"))]
use connector::{Connection, generic_apply_migration_script, shadow_db};
use destructive_change_checker::MssqlDestructiveChangeCheckerFlavour;
use indoc::formatdoc;
use quaint::{
    connector::{DEFAULT_MSSQL_SCHEMA, MssqlUrl},
    prelude::Table,
};
use renderer::MssqlRenderer;
use schema_calculator::MssqlSchemaCalculatorFlavour;
use schema_connector::{
    BoxFuture, ConnectorError, ConnectorParams, ConnectorResult, Namespaces, SchemaFilter,
    migrations_directory::Migrations,
};
use schema_differ::MssqlSchemaDifferFlavour;
use sql_schema_describer::SqlSchema;
use std::{future, str::FromStr};

use super::{SqlDialect, UsingExternalShadowDb};

const DEFAULT_SCHEMA_NAME: &str = "dbo";

type State = super::State<Params, Connection>;

struct Params {
    connector_params: ConnectorParams,
    url: MssqlUrl,
}

impl Params {
    fn new(connector_params: ConnectorParams) -> ConnectorResult<Self> {
        if let Some(shadow_db_url) = &connector_params.shadow_database_connection_string {
            super::validate_connection_infos_do_not_match(&connector_params.connection_string, shadow_db_url)?;
        }

        let url = MssqlUrl::new(&connector_params.connection_string).map_err(ConnectorError::url_parse_error)?;
        Ok(Self { connector_params, url })
    }

    fn is_running_on_azure_sql(&self) -> bool {
        self.url.host().contains(".database.windows.net")
    }
}

#[derive(Debug)]
pub struct MssqlDialect {
    schema_name: String,
}

impl MssqlDialect {
    fn new(schema_name: String) -> Self {
        Self { schema_name }
    }

    fn schema_name(&self) -> &str {
        &self.schema_name
    }
}

impl Default for MssqlDialect {
    fn default() -> Self {
        Self::new(DEFAULT_SCHEMA_NAME.to_string())
    }
}

impl SqlDialect for MssqlDialect {
    fn renderer(&self) -> Box<dyn SqlRenderer> {
        Box::new(MssqlRenderer::new(self.schema_name().to_owned()))
    }

    fn schema_differ(&self) -> Box<dyn crate::sql_schema_differ::SqlSchemaDifferFlavour> {
        Box::new(MssqlSchemaDifferFlavour)
    }

    fn schema_calculator(&self) -> Box<dyn crate::sql_schema_calculator::SqlSchemaCalculatorFlavour> {
        Box::new(MssqlSchemaCalculatorFlavour)
    }

    fn destructive_change_checker(&self) -> Box<dyn sql_destructive_change_checker::DestructiveChangeCheckerFlavour> {
        Box::new(MssqlDestructiveChangeCheckerFlavour::new(self.schema_name().to_owned()))
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::MSSQL
    }

    fn migrations_table(&self) -> Table<'static> {
        (self.schema_name().to_owned(), crate::MIGRATIONS_TABLE_NAME.to_owned()).into()
    }

    fn empty_database_schema(&self) -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.set_connector_data(Box::<sql_schema_describer::mssql::MssqlSchemaExt>::default());
        schema
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(DEFAULT_MSSQL_SCHEMA)
    }

    #[cfg(feature = "mssql-native")]
    fn connect_to_shadow_db(
        &self,
        url: String,
        preview_features: psl::PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>> {
        let params = ConnectorParams::new(url, preview_features, None);
        Box::pin(async move { Ok(Box::new(MssqlConnector::new_with_params(params)?) as Box<dyn SqlConnector>) })
    }

    #[cfg(not(feature = "mssql-native"))]
    fn connect_to_shadow_db(
        &self,
        factory: Arc<dyn ExternalConnectorFactory>,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>> {
        todo!("MSSQL Wasm shadow database not supported yet")
    }
}

pub(crate) struct MssqlConnector {
    state: State,
}

impl std::fmt::Debug for MssqlConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MssqlFlavour").field("url", &"<REDACTED>").finish()
    }
}

impl MssqlConnector {
    pub fn new_with_params(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(Self {
            state: State::WithParams(Params::new(params)?),
        })
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.state
            .params()
            .map(|p| p.url.schema())
            .unwrap_or(DEFAULT_SCHEMA_NAME)
    }

    /// Get the url as a JDBC string, extract the database name, and re-encode the string.
    fn master_url(input: &str) -> ConnectorResult<(String, String)> {
        let url = MssqlUrl::new(input).map_err(ConnectorError::url_parse_error)?;
        let db_name = url.dbname().into_owned();

        let mut conn = JdbcString::from_str(&format!("jdbc:{input}"))
            .map_err(|e| ConnectorError::from_source(e, "JDBC string parse error"))?;
        conn.properties_mut().remove("database");

        Ok((db_name, conn.to_string()))
    }
}

impl SqlConnector for MssqlConnector {
    fn dialect(&self) -> Box<dyn SqlDialect> {
        Box::new(MssqlDialect::new(self.schema_name().to_owned()))
    }

    fn shadow_db_url(&self) -> Option<&str> {
        self.state
            .params()?
            .connector_params
            .shadow_database_connection_string
            .as_deref()
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        // see
        // https://docs.microsoft.com/en-us/sql/relational-databases/system-stored-procedures/sp-getapplock-transact-sql?view=sql-server-ver15
        // We don't set an explicit timeout because we want to respect the
        // server-set default.
        Box::pin(
            self.raw_cmd("sp_getapplock @Resource = 'prisma_migrate', @LockMode = 'Exclusive', @LockOwner = 'Session'"),
        )
    }

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |_, connection| {
            generic_apply_migration_script(migration_name, script, connection)
        })
    }

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>> {
        with_connection(&mut self.state, |params, connection| async move {
            connection.describe_schema(params, namespaces).await
        })
    }

    fn connector_type(&self) -> &'static str {
        "mssql"
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let connection_string = &params.connector_params.connection_string;
            let (db_name, master_uri) = Self::master_url(connection_string)?;
            let mut master_conn = Connection::new(&master_uri).await?;

            let query = format!("CREATE DATABASE [{db_name}]");
            master_conn
                .raw_cmd(
                    &query,
                    &Params {
                        url: MssqlUrl::new(&master_uri).unwrap(),
                        connector_params: ConnectorParams::new(master_uri, Default::default(), None),
                    },
                )
                .await?;

            let mut conn = Connection::new(&params.connector_params.connection_string).await?;

            // dbo is created automatically
            if params.url.schema() != DEFAULT_SCHEMA_NAME {
                let query = format!("CREATE SCHEMA {}", params.url.schema());
                conn.raw_cmd(&query, params).await?;
            }

            Ok(db_name)
        })
    }

    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let sql = formatdoc! { r#"
            CREATE TABLE [{}].[{}] (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIMEOFFSET,
                migration_name          NVARCHAR(250) NOT NULL,
                logs                    NVARCHAR(MAX) NULL,
                rolled_back_at          DATETIMEOFFSET,
                started_at              DATETIMEOFFSET NOT NULL DEFAULT CURRENT_TIMESTAMP,
                applied_steps_count     INT NOT NULL DEFAULT 0
            );
        "#, self.schema_name(), crate::MIGRATIONS_TABLE_NAME};

        Box::pin(async move { self.raw_cmd(&sql).await })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async {
            let params = self.state.get_unwrapped_params();
            let (db_name, master_uri) = Self::master_url(&params.connector_params.connection_string)?;
            assert!(db_name != "master", "Cannot drop the `master` database.");
            let mut conn = Connection::new(&master_uri.to_string()).await?;

            let query = format!("DROP DATABASE IF EXISTS [{db_name}]");
            conn.raw_cmd(
                &query,
                &Params {
                    connector_params: ConnectorParams::new(master_uri.clone(), Default::default(), None),
                    url: MssqlUrl::new(&master_uri).unwrap(),
                },
            )
            .await?;

            Ok(())
        })
    }

    fn table_names(
        &mut self,
        namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>> {
        Box::pin(async move {
            let search_path = self.schema_name().to_string();

            let mut namespaces: Vec<_> = namespaces.map(|ns| ns.into_iter().collect()).unwrap_or_default();
            namespaces.push(search_path);

            let select = r#"
                SELECT
                    tbl.name AS table_name,
                    SCHEMA_NAME(tbl.schema_id) AS namespace
                FROM sys.tables tbl
                WHERE tbl.is_ms_shipped = 0 AND tbl.type = 'U'
                ORDER BY tbl.name;
            "#;

            let rows = self.query_raw(select, &[]).await?;

            let table_names: Vec<String> = rows
                .into_iter()
                .flat_map(|row| {
                    let ns = row.get("namespace").and_then(|s| s.to_string());
                    let table_name = row.get("table_name").and_then(|s| s.to_string());

                    ns.and_then(|ns| table_name.map(|table_name| (ns, table_name)))
                })
                .filter(|(ns, table_name)| {
                    namespaces.contains(ns)
                        && !self.dialect().schema_differ().contains_table(
                            &filters.external_tables,
                            Some(ns),
                            table_name,
                        )
                })
                .map(|(_, table_name)| table_name)
                .collect();

            Ok(table_names)
        })
    }

    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        let sql = format!("DROP TABLE [{}].[{}]", self.schema_name(), crate::MIGRATIONS_TABLE_NAME);
        Box::pin(async move { self.raw_cmd(&sql).await })
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, move |params, conn| async move {
            conn.query(query, params).await
        })
    }

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>> {
        with_connection(&mut self.state, move |conn_params, conn| async move {
            conn.query_raw(sql, params, conn_params).await
        })
    }

    #[tracing::instrument(skip(self))]
    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        with_connection(&mut self.state, move |params, connection| async move {
            let ns_vec = Namespaces::to_vec(namespaces, params.url.schema().to_string());
            for schema_name in &ns_vec {
                let drop_procedures = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'DROP PROCEDURE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
                    FROM sys.objects
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}' AND type = 'P'

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                let drop_shared_defaults = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'DROP DEFAULT [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(object_id) + ']'
                    FROM sys.objects
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}' AND type = 'D' AND parent_object_id = 0

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                let drop_views = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'DROP VIEW [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
                    FROM sys.views
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}'

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                let drop_fks = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'ALTER TABLE [' + SCHEMA_NAME(schema_id) + '].[' + OBJECT_NAME(parent_object_id) + '] DROP CONSTRAINT [' + name + ']'
                    FROM sys.foreign_keys
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}'

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                let drop_tables = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'DROP TABLE [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
                    FROM sys.tables
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}'

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                let drop_types = format!(
                    r#"
                    DECLARE @stmt NVARCHAR(max)
                    DECLARE @n CHAR(1)

                    SET @n = CHAR(10)

                    SELECT @stmt = ISNULL(@stmt + @n, '') +
                        'DROP TYPE [' + SCHEMA_NAME(schema_id) + '].[' + name + ']'
                    FROM sys.types
                    WHERE SCHEMA_NAME(schema_id) = '{schema_name}'
                    AND is_user_defined = 1

                    EXEC SP_EXECUTESQL @stmt
                    "#
                );

                connection.raw_cmd(&drop_procedures, params).await?;
                connection.raw_cmd(&drop_views, params).await?;
                connection.raw_cmd(&drop_shared_defaults, params).await?;
                connection.raw_cmd(&drop_fks, params).await?;
                connection.raw_cmd(&drop_tables, params).await?;
                connection.raw_cmd(&drop_types, params).await?;
            }

            // We need to drop namespaces after we've dropped everything else.
            for schema_name in ns_vec {
                let drop_namespace = format!("DROP SCHEMA IF EXISTS [{schema_name}]");
                connection.raw_cmd(&drop_namespace, params).await?;
            }

            Ok(())
        })
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.raw_cmd("SELECT 1")
    }

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        with_connection(&mut self.state, move |params, conn| conn.raw_cmd(sql, params))
    }

    fn set_preview_features(&mut self, preview_features: enumflags2::BitFlags<psl::PreviewFeature>) {
        match &mut self.state {
            super::State::Initial => {
                if !preview_features.is_empty() {
                    tracing::warn!("set_preview_feature on Initial state has no effect ({preview_features}).");
                }
            }
            super::State::WithParams(params) | super::State::Connected(params, _) => {
                params.connector_params.preview_features = preview_features
            }
        }
    }

    fn preview_features(&self) -> psl::PreviewFeatures {
        self.state
            .params()
            .map(|p| p.connector_params.preview_features)
            .unwrap_or_default()
    }

    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        external_shadow_db: UsingExternalShadowDb,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        match external_shadow_db {
            UsingExternalShadowDb::Yes => Box::pin(async move {
                self.ensure_connection_validity().await?;
                tracing::info!("Connected to an external shadow database.");

                if self.reset(namespaces.clone()).await.is_err() {
                    crate::best_effort_reset(self, namespaces.clone(), filter).await?;
                }

                shadow_db::sql_schema_from_migrations_history(migrations, self, namespaces).await
            }),

            // If we're not using an external shadow database, one must be created manually.
            UsingExternalShadowDb::No => {
                with_connection(&mut self.state, move |params, main_connection| async move {
                    let shadow_database_name = crate::new_shadow_database_name();
                    // See https://github.com/prisma/prisma/issues/6371 for the rationale on
                    // this conditional.
                    if params.is_running_on_azure_sql() {
                        return Err(ConnectorError::user_facing(
                            user_facing_errors::schema_engine::AzureMssqlShadowDb,
                        ));
                    }

                    let create_database = format!("CREATE DATABASE [{shadow_database_name}]");

                    main_connection
                        .raw_cmd(&create_database, params)
                        .await
                        .map_err(|err| err.into_shadow_db_creation_error())?;

                    let connection_string = format!("jdbc:{}", params.connector_params.connection_string);
                    let mut jdbc_string: JdbcString = connection_string.parse().unwrap();
                    jdbc_string
                        .properties_mut()
                        .insert("database".into(), shadow_database_name.to_owned());
                    let host = jdbc_string.server_name();

                    tracing::debug!("Connecting to shadow database at {}", host.unwrap_or("localhost"));

                    let connector_params =
                        ConnectorParams::new(jdbc_string.to_string(), params.connector_params.preview_features, None);
                    let mut shadow_database = MssqlConnector::new_with_params(connector_params.clone())?;

                    if let Some(schema) = jdbc_string.properties().get("schema")
                        && schema != DEFAULT_SCHEMA_NAME
                    {
                        shadow_database
                            .raw_cmd(&format!("CREATE SCHEMA [{schema}]"))
                            .await
                            .map_err(|err| err.into_shadow_db_creation_error())?;
                    }

                    // We go through the whole process without early return, then clean up
                    // the shadow database, and only then return the result. This avoids
                    // leaving shadow databases behind in case of e.g. faulty
                    // migrations.
                    let ret =
                        shadow_db::sql_schema_from_migrations_history(migrations, &mut shadow_database, namespaces)
                            .await;

                    // Drop the shadow database before cleaning up from the main connection.
                    drop(shadow_database);

                    clean_up_shadow_database(&shadow_database_name, main_connection, params).await?;
                    ret
                })
            }
        }
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        with_connection(&mut self.state, |params, connection| async {
            connection.version(params).await
        })
    }

    fn search_path(&self) -> &str {
        self.schema_name()
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(self.schema_name())
    }

    fn describe_query<'a>(
        &'a mut self,
        _sql: &str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>> {
        unimplemented!("SQL Server does not support describe_query yet.")
    }

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        // Nothing to on dispose, the connection is disposed in Drop
        Box::pin(async move { Ok(()) })
    }
}

fn with_connection<'a, O, F, C>(state: &'a mut State, f: C) -> BoxFuture<'a, ConnectorResult<O>>
where
    O: 'a,
    F: future::Future<Output = ConnectorResult<O>> + Send + 'a,
    C: (FnOnce(&'a mut Params, &'a mut Connection) -> F) + Send + 'a,
{
    match state {
        super::State::Initial => panic!("logic error: Initial"),
        super::State::Connected(p, c) => Box::pin(f(p, c)),
        state @ super::State::WithParams(_) => Box::pin(async move {
            state
                .try_connect(|params| Box::pin(Connection::new(&params.connector_params.connection_string)))
                .await?;
            with_connection(state, f).await
        }),
    }
}

/// Call this on the _main_ database when you are done with a shadow database.
async fn clean_up_shadow_database(
    database_name: &str,
    connection: &mut Connection,
    params: &Params,
) -> ConnectorResult<()> {
    let drop_database = format!("DROP DATABASE [{database_name}]");
    connection.raw_cmd(&drop_database, params).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_impl_does_not_leak_connection_info() {
        let url = "sqlserver://myserver:8765;database=master;schema=mydbname;user=SA;password=<mypassword>;trustServerCertificate=true;socket_timeout=60;isolationLevel=READ UNCOMMITTED";

        let params = ConnectorParams::new(url.to_owned(), Default::default(), None);
        let flavour = MssqlConnector::new_with_params(params).unwrap();
        let debugged = format!("{flavour:?}");

        let words = &["myname", "mypassword", "myserver", "8765", "mydbname"];

        for word in words {
            assert!(!debugged.contains(word));
        }
    }
}
