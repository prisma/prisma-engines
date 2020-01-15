mod apply;
mod infer;
mod infer_apply;
mod unapply_migration;

pub(crate) use apply::Apply;
pub(crate) use infer::Infer;
pub(crate) use infer_apply::InferApply;
pub(crate) use unapply_migration::UnapplyMigration;

use super::assertions::SchemaAssertion;
use super::{
    misc_helpers::{mysql_migration_connector, postgres_migration_connector, sqlite_migration_connector, test_api},
    InferAndApplyOutput,
};
use crate::{
    api::{GenericApi, MigrationApi},
    commands::ApplyMigrationInput,
};
use migration_connector::{MigrationPersistence, MigrationStep};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::*;

/// A handle to all the context needed for end-to-end testing of the migration engine across
/// connectors.
pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
    api: MigrationApi<sql_migration_connector::SqlMigrationConnector, sql_migration_connector::SqlMigration>,
    connection_info: ConnectionInfo,
}

impl TestApi {
    pub fn connector_name(&self) -> &str {
        self.connector_name
    }

    pub fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }

    pub fn database(&self) -> &Arc<dyn Queryable + Send + Sync + 'static> {
        &self.database
    }

    pub fn is_sqlite(&self) -> bool {
        self.sql_family() == SqlFamily::Sqlite
    }

    pub fn is_mysql(&self) -> bool {
        self.sql_family() == SqlFamily::Mysql
    }

    pub fn migration_persistence<'a>(&'a self) -> Box<dyn MigrationPersistence + 'a> {
        self.api.migration_persistence()
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name(&self, table_name: &str) -> quaint::ast::Table {
        (self.schema_name().to_owned(), table_name.to_owned()).into()
    }

    pub async fn apply_migration(&self, steps: Vec<MigrationStep>, migration_id: &str) -> InferAndApplyOutput {
        let input = ApplyMigrationInput {
            migration_id: migration_id.into(),
            steps,
            force: None,
        };

        let migration_output = self.api.apply_migration(&input).await.expect("ApplyMigration failed");

        assert!(
            migration_output.general_errors.is_empty(),
            format!(
                "ApplyMigration returned unexpected errors: {:?}",
                migration_output.general_errors
            )
        );

        InferAndApplyOutput {
            sql_schema: self.describe_database().await.unwrap(),
            migration_output,
        }
    }

    pub fn infer_apply<'a>(&'a self, schema: &'a str) -> InferApply<'a> {
        InferApply {
            api: &self.api,
            force: None,
            migration_id: None,
            schema,
        }
    }

    pub async fn infer_and_apply(&self, schema: &str) -> InferAndApplyOutput {
        let migration_output = self.infer_apply(schema).send().await.unwrap();

        InferAndApplyOutput {
            migration_output,
            sql_schema: self.describe_database().await.unwrap(),
        }
    }

    pub async fn execute_command<'a, C>(&self, input: &'a C::Input) -> Result<C::Output, user_facing_errors::Error>
    where
        C: crate::commands::MigrationCommand,
    {
        self.api
            .handle_command::<C>(input)
            .await
            .map_err(|err| self.api.render_error(err))
    }

    pub fn infer<'a>(&'a self, dm: impl Into<String>) -> Infer<'a> {
        Infer {
            datamodel: dm.into(),
            api: &self.api,
            assume_to_be_applied: None,
            migration_id: None,
        }
    }

    pub(crate) fn apply<'a>(&'a self) -> Apply<'a> {
        Apply {
            api: &self.api,
            migration_id: None,
            steps: None,
            force: None,
        }
    }

    pub(crate) fn unapply_migration<'a>(&'a self) -> UnapplyMigration<'a> {
        UnapplyMigration {
            api: &self.api,
            force: None,
        }
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
            inspector: self.describer(),
            database: Arc::clone(&self.database),
            sql_variant: match self.sql_family() {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
            },
        }
    }

    fn describer(&self) -> Box<dyn SqlSchemaDescriberBackend> {
        let db = Arc::clone(&self.database);
        match self.api.connector_type() {
            "postgresql" => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(db)),
            "sqlite" => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
            "mysql" => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
            _ => unimplemented!(),
        }
    }

    pub async fn describe_database(&self) -> Result<SqlSchema, anyhow::Error> {
        let mut result = self
            .describer()
            .describe(self.schema_name())
            .await
            .expect("Description failed");

        // the presence of the _Migration table makes assertions harder. Therefore remove it from the result.
        result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();

        Ok(result)
    }

    pub async fn assert_schema(&self) -> Result<SchemaAssertion, anyhow::Error> {
        let schema = self.describe_database().await?;

        Ok(SchemaAssertion(schema))
    }

    pub async fn dump_table(&self, table_name: &str) -> Result<quaint::prelude::ResultSet, quaint::error::Error> {
        let select_star =
            quaint::ast::Select::from_table(self.render_table_name(table_name)).value(quaint::ast::asterisk());

        self.database.query(select_star.into()).await
    }
}

pub async fn mysql_8_test_api(db_name: &str) -> TestApi {
    let url = mysql_8_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_8",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn mysql_test_api(db_name: &str) -> TestApi {
    let url = mysql_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn mysql_mariadb_test_api(db_name: &str) -> TestApi {
    let url = mariadb_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_mariadb",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn postgres9_test_api(db_name: &str) -> TestApi {
    let url = postgres_9_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres9",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn postgres_test_api(db_name: &str) -> TestApi {
    let url = postgres_10_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn postgres11_test_api(db_name: &str) -> TestApi {
    let url = postgres_11_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres11",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn postgres12_test_api(db_name: &str) -> TestApi {
    let url = postgres_12_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres12",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn sqlite_test_api(db_name: &str) -> TestApi {
    let connection_info = ConnectionInfo::from_url(&sqlite_test_url(db_name)).unwrap();
    let connector = sqlite_migration_connector(db_name).await;

    TestApi {
        connector_name: "sqlite",
        connection_info,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub struct BarrelMigrationExecutor {
    inspector: Box<dyn SqlSchemaDescriberBackend>,
    database: Arc<dyn Queryable + Send + Sync>,
    sql_variant: barrel::backend::SqlVariant,
    schema_name: String,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, mut migration_fn: F) -> SqlSchema
    where
        F: FnMut(&mut barrel::Migration) -> (),
    {
        use barrel::Migration;

        let mut migration = Migration::new().schema(&self.schema_name);
        migration_fn(&mut migration);
        let full_sql = migration.make_from(self.sql_variant);
        run_full_sql(&self.database, &full_sql).await;
        let mut result = self
            .inspector
            .describe(&self.schema_name)
            .await
            .expect("Description failed");

        // The presence of the _Migration table makes assertions harder. Therefore remove it.
        result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();
        result
    }
}

async fn run_full_sql(database: &Arc<dyn Queryable + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";").filter(|sql| !sql.is_empty()) {
        database.query_raw(&sql, &[]).await.unwrap();
    }
}
