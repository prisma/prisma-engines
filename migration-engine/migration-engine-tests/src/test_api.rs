mod apply;
mod calculate_database_steps;
mod infer;
mod infer_apply;
mod schema_push;
mod unapply_migration;

pub use apply::Apply;
pub use calculate_database_steps::CalculateDatabaseSteps;
pub use infer::Infer;
pub use infer_apply::InferApply;
pub use schema_push::SchemaPush;
pub use unapply_migration::UnapplyMigration;

use super::assertions::SchemaAssertion;
use super::{
    misc_helpers::{mysql_migration_connector, postgres_migration_connector, sqlite_migration_connector, test_api},
    sql::barrel_migration_executor::BarrelMigrationExecutor,
    InferAndApplyOutput,
};
use migration_connector::{ImperativeMigrationsPersistence, MigrationPersistence, MigrationStep};
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::ApplyMigrationInput,
};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_migration_connector::{sql_migration::SqlMigration, SqlMigrationConnector, MIGRATION_TABLE_NAME};
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::*;

/// A handle to all the context needed for end-to-end testing of the migration engine across
/// connectors.
pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
    api: MigrationApi<SqlMigrationConnector, SqlMigration>,
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

    pub fn is_mysql_8(&self) -> bool {
        self.connector_name == "mysql_8"
    }

    pub fn is_mariadb(&self) -> bool {
        self.connector_name == "mysql_mariadb"
    }

    pub fn migration_persistence<'a>(&'a self) -> Box<dyn MigrationPersistence + 'a> {
        self.api.migration_persistence()
    }

    pub fn imperative_migration_persistence<'a>(&'a self) -> &(dyn ImperativeMigrationsPersistence + 'a) {
        self.api.connector()
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }

    pub fn datasource(&self) -> String {
        match self.sql_family() {
            SqlFamily::Mysql => mysql_test_config("unreachable"),
            SqlFamily::Postgres => postgres_12_test_config("unreachable"),
            SqlFamily::Sqlite => sqlite_test_config("unreachable"),
            SqlFamily::Mssql => todo!("Greetings from Redmond"),
        }
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name<'a>(&self, table_name: &'a str) -> quaint::ast::Table<'a> {
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

    pub fn infer<'a>(&'a self, dm: impl Into<String>) -> Infer<'a> {
        Infer::new(&self.api, dm)
    }

    pub fn apply<'a>(&'a self) -> Apply<'a> {
        Apply::new(&self.api)
    }

    pub fn unapply_migration<'a>(&'a self) -> UnapplyMigration<'a> {
        UnapplyMigration {
            api: &self.api,
            force: None,
        }
    }

    pub fn schema_push<'a>(&'a self, dm: impl Into<String>) -> SchemaPush<'a> {
        SchemaPush::new(&self.api, dm.into())
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor<'_> {
        BarrelMigrationExecutor {
            api: self,
            sql_variant: match self.sql_family() {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
                SqlFamily::Mssql => todo!("Greetings from Redmond"),
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

    pub fn calculate_database_steps<'a>(&'a self) -> CalculateDatabaseSteps<'a> {
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

pub async fn mysql_5_6_test_api(db_name: &str) -> TestApi {
    let url = mysql_5_6_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_5_6",
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

pub async fn postgres13_test_api(db_name: &str) -> TestApi {
    let url = postgres_13_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = postgres_migration_connector(&url).await;

    TestApi {
        connector_name: "postgres13",
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
