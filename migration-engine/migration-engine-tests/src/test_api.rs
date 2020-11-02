mod apply;
mod apply_migrations;
mod calculate_database_steps;
mod create_migration;
mod diagnose_migration_history;
mod evaluate_data_loss;
mod infer;
mod infer_apply;
mod reset;
mod schema_push;
mod unapply_migration;

pub use apply::Apply;
pub use apply_migrations::ApplyMigrations;
pub use calculate_database_steps::CalculateDatabaseSteps;
pub use create_migration::CreateMigration;
pub use diagnose_migration_history::DiagnoseMigrationHistory;
pub use evaluate_data_loss::EvaluateDataLoss;
pub use infer::Infer;
pub use infer_apply::InferApply;
pub use reset::Reset;
pub use schema_push::SchemaPush;
pub use unapply_migration::UnapplyMigration;

use crate::AssertionResult;

use super::assertions::SchemaAssertion;
use super::{
    misc_helpers::{mysql_migration_connector, postgres_migration_connector, sqlite_migration_connector, test_api},
    sql::barrel_migration_executor::BarrelMigrationExecutor,
    InferAndApplyOutput,
};
use crate::connectors::Tags;
use enumflags2::BitFlags;
use migration_connector::{
    ImperativeMigrationsPersistence, MigrationConnector, MigrationPersistence, MigrationRecord, MigrationStep,
};
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::ApplyMigrationInput,
};
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_migration_connector::{SqlMigration, SqlMigrationConnector, MIGRATION_TABLE_NAME};
use sql_schema_describer::*;
use tempfile::TempDir;
use test_setup::*;

/// A handle to all the context needed for end-to-end testing of the migration engine across
/// connectors.
pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    database: Quaint,
    api: MigrationApi<SqlMigrationConnector, SqlMigration>,
    tags: BitFlags<Tags>,
}

impl TestApi {
    pub fn connector_name(&self) -> &str {
        self.connector_name
    }

    pub fn schema_name(&self) -> &str {
        self.connection_info().schema_name()
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub fn is_sqlite(&self) -> bool {
        self.tags.contains(Tags::Sqlite)
    }

    pub fn is_mysql(&self) -> bool {
        self.tags.contains(Tags::Mysql)
    }

    pub fn is_mysql_8(&self) -> bool {
        self.connector_name == "mysql_8"
    }

    pub fn is_mariadb(&self) -> bool {
        self.connector_name == "mysql_mariadb"
    }

    pub fn migration_persistence(&self) -> &dyn MigrationPersistence {
        self.api.connector().migration_persistence()
    }

    pub fn imperative_migration_persistence<'a>(&'a self) -> &(dyn ImperativeMigrationsPersistence + 'a) {
        self.api.connector()
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.database.connection_info()
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }

    pub fn datasource(&self) -> String {
        match self.sql_family() {
            SqlFamily::Mysql => mysql_test_config("unreachable"),
            SqlFamily::Postgres => postgres_12_test_config("unreachable"),
            SqlFamily::Sqlite => sqlite_test_config("unreachable"),
            SqlFamily::Mssql => mssql_2019_test_config("unreachable"),
        }
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name<'a>(&'a self, table_name: &'a str) -> quaint::ast::Table<'a> {
        if self.is_sqlite() {
            table_name.into()
        } else {
            (self.connection_info().schema_name(), table_name).into()
        }
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
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

    pub fn apply_migrations<'a>(&'a self, migrations_directory: &'a TempDir) -> ApplyMigrations<'a> {
        ApplyMigrations::new(&self.api, migrations_directory)
    }

    /// Convenient builder and assertions for the CreateMigration command.
    pub fn create_migration<'a>(
        &'a self,
        name: &'a str,
        prisma_schema: &'a str,
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new(&self.api, name, prisma_schema, migrations_directory)
    }

    /// Builder and assertions to call the DiagnoseMigrationHistory command.
    pub fn diagnose_migration_history<'a>(&'a self, migrations_directory: &'a TempDir) -> DiagnoseMigrationHistory<'a> {
        DiagnoseMigrationHistory::new(&self.api, migrations_directory)
    }

    pub fn infer_apply<'a>(&'a self, schema: &'a str) -> InferApply<'a> {
        InferApply::new(&self.api, schema)
    }

    pub async fn infer_and_apply_forcefully(&self, schema: &str) -> InferAndApplyOutput {
        let migration_output = self
            .infer_apply(schema)
            .force(Some(true))
            .send()
            .await
            .unwrap()
            .into_inner();

        InferAndApplyOutput {
            migration_output,
            sql_schema: self.describe_database().await.unwrap(),
        }
    }

    pub async fn infer_and_apply(&self, schema: &str) -> InferAndApplyOutput {
        let migration_output = self.infer_apply(schema).send().await.unwrap().into_inner();

        InferAndApplyOutput {
            migration_output,
            sql_schema: self.describe_database().await.unwrap(),
        }
    }

    pub fn infer(&self, dm: impl Into<String>) -> Infer<'_> {
        Infer::new(&self.api, dm)
    }

    pub fn apply(&self) -> Apply<'_> {
        Apply::new(&self.api)
    }

    pub fn unapply_migration(&self) -> UnapplyMigration<'_> {
        UnapplyMigration {
            api: &self.api,
            force: None,
        }
    }

    pub fn evaluate_data_loss<'a>(
        &'a self,
        migrations_directory: &'a TempDir,
        prisma_schema: impl Into<String>,
    ) -> EvaluateDataLoss<'a> {
        EvaluateDataLoss::new(&self.api, migrations_directory, prisma_schema.into())
    }

    pub fn reset(&self) -> Reset<'_> {
        Reset::new(&self.api)
    }

    pub fn schema_push(&self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new(&self.api, dm.into())
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor<'_> {
        BarrelMigrationExecutor {
            api: self,
            sql_variant: match self.sql_family() {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
                SqlFamily::Mssql => barrel::SqlVariant::Mssql,
            },
        }
    }

    pub async fn describe_database(&self) -> Result<SqlSchema, anyhow::Error> {
        let mut result = self.api.connector().describe_schema().await?;

        // the presence of the _Migration table makes assertions harder. Therefore remove it from the result.
        result.tables = result
            .tables
            .into_iter()
            .filter(|t| t.name != MIGRATION_TABLE_NAME)
            .collect();

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

    pub fn insert<'a>(&'a self, table_name: &'a str) -> SingleRowInsert<'a> {
        SingleRowInsert {
            insert: quaint::ast::Insert::single_into(self.render_table_name(table_name)),
            api: self,
        }
    }

    pub fn select<'a>(&'a self, table_name: &'a str) -> TestApiSelect<'_> {
        TestApiSelect {
            select: quaint::ast::Select::from_table(self.render_table_name(table_name)),
            api: self,
        }
    }

    pub fn calculate_database_steps(&self) -> CalculateDatabaseSteps<'_> {
        CalculateDatabaseSteps::new(&self.api)
    }
}

pub struct SingleRowInsert<'a> {
    insert: quaint::ast::SingleRowInsert<'a>,
    api: &'a TestApi,
}

impl<'a> SingleRowInsert<'a> {
    pub fn value(mut self, name: &'a str, value: impl Into<quaint::ast::Expression<'a>>) -> Self {
        self.insert = self.insert.value(name, value);

        self
    }

    pub async fn result_raw(self) -> Result<quaint::connector::ResultSet, anyhow::Error> {
        Ok(self.api.database().query(self.insert.into()).await?)
    }
}

pub struct TestApiSelect<'a> {
    select: quaint::ast::Select<'a>,
    api: &'a TestApi,
}

impl<'a> TestApiSelect<'a> {
    pub fn column(mut self, name: &'a str) -> Self {
        self.select = self.select.column(name);

        self
    }

    /// This is deprecated. Used row assertions instead with the ResultSetExt trait.
    pub async fn send_debug(self) -> Result<Vec<Vec<String>>, anyhow::Error> {
        let rows = self.send().await?;

        let rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(|col| format!("{:?}", col)).collect())
            .collect();

        Ok(rows)
    }

    pub async fn send(self) -> anyhow::Result<quaint::prelude::ResultSet> {
        Ok(self.api.database().query(self.select.into()).await?)
    }
}

pub async fn mysql_8_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_8_url(db_name);
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_8",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn mysql_5_6_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_5_6_url(db_name);
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_5_6",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn mysql_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_url(db_name);
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn mysql_mariadb_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mariadb_url(db_name);
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_mariadb",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn postgres9_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_9_url(db_name);
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres9",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn postgres_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_10_url(db_name);
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn postgres11_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_11_url(db_name);
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres11",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn postgres12_test_api(args: TestAPIArgs) -> TestApi {
    let url = postgres_12_url(args.test_function_name);
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres12",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn postgres13_test_api(args: TestAPIArgs) -> TestApi {
    let url = postgres_13_url(args.test_function_name);
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres13",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub async fn sqlite_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let connector = sqlite_migration_connector(db_name).await;

    TestApi {
        connector_name: "sqlite",
        database: connector.quaint().clone(),
        api: test_api(connector).await,
        tags: args.test_tag,
    }
}

pub trait MigrationsAssertions: Sized {
    fn assert_checksum(self, expected: &str) -> AssertionResult<Self>;
    fn assert_migration_name(self, expected: &str) -> AssertionResult<Self>;
    fn assert_logs(self, expected: &str) -> AssertionResult<Self>;
    fn assert_applied_steps_count(self, count: u32) -> AssertionResult<Self>;
    fn assert_success(self) -> AssertionResult<Self>;
}

impl MigrationsAssertions for MigrationRecord {
    fn assert_checksum(self, expected: &str) -> AssertionResult<Self> {
        assert_eq!(self.checksum, expected);

        Ok(self)
    }

    fn assert_migration_name(self, expected: &str) -> AssertionResult<Self> {
        assert_eq!(&self.migration_name[15..], expected);

        Ok(self)
    }

    fn assert_logs(self, expected: &str) -> AssertionResult<Self> {
        assert_eq!(self.logs, expected);

        Ok(self)
    }

    fn assert_applied_steps_count(self, count: u32) -> AssertionResult<Self> {
        assert_eq!(self.applied_steps_count, count);

        Ok(self)
    }

    fn assert_success(self) -> AssertionResult<Self> {
        assert!(self.finished_at.is_some());

        Ok(self)
    }
}
