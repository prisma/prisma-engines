use super::assertions::SchemaAssertion;
use super::{
    misc_helpers::{mysql_migration_connector, postgres_migration_connector, sqlite_migration_connector, test_api},
    InferAndApplyOutput,
};
use crate::{
    api::{GenericApi, MigrationApi},
    commands::{ApplyMigrationInput, MigrationStepsResultOutput, UnapplyMigrationInput, UnapplyMigrationOutput},
};
use migration_connector::{MigrationPersistence, MigrationStep};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::*;

mod apply;
mod infer;
mod infer_apply;

pub use apply::Apply;
pub use infer::Infer;
pub use infer_apply::InferApply;

/// A handle to all the context needed for end-to-end testing of the migration engine across
/// connectors.
pub struct TestApi {
    /// More precise than SqlFamily.
    connector_name: &'static str,
    sql_family: SqlFamily,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
    api: MigrationApi<sql_migration_connector::SqlMigrationConnector, sql_migration_connector::SqlMigration>,
    connection_info: Option<ConnectionInfo>,
}

impl TestApi {
    pub fn connector_name(&self) -> &str {
        self.connector_name
    }

    pub fn schema_name(&self) -> &str {
        self.connection_info.as_ref().unwrap().schema_name()
    }

    pub fn database(&self) -> &Arc<dyn Queryable + Send + Sync + 'static> {
        &self.database
    }

    pub fn is_sqlite(&self) -> bool {
        self.sql_family == SqlFamily::Sqlite
    }

    pub fn is_mysql(&self) -> bool {
        self.sql_family == SqlFamily::Mysql
    }

    pub fn migration_persistence(&self) -> Arc<dyn MigrationPersistence> {
        self.api.migration_persistence()
    }

    pub fn connection_info(&self) -> Option<&ConnectionInfo> {
        self.connection_info.as_ref()
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name(&self, table_name: &str) -> quaint::ast::Table {
        match self.connection_info.as_ref().map(|ci| ci.schema_name()) {
            Some(schema_name) => (schema_name.to_owned(), table_name.to_owned()).into(),
            None => table_name.to_owned().into(),
        }
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

    pub fn apply<'a>(&'a self) -> Apply<'a> {
        Apply {
            api: &self.api,
            migration_id: None,
            steps: None,
            force: None,
        }
    }

    pub async fn apply_migration_with(
        &self,
        input: &ApplyMigrationInput,
    ) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        Ok(self.api.apply_migration(&input).await?)
    }

    pub async fn unapply_migration(&self) -> UnapplyOutput {
        let input = UnapplyMigrationInput {};
        let output = self.api.unapply_migration(&input).await.unwrap();
        let sql_schema = self.describe_database().await.unwrap();

        UnapplyOutput { sql_schema, output }
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.connection_info().unwrap().schema_name().to_owned(),
            inspector: self.describer(),
            database: Arc::clone(&self.database),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
            },
        }
    }

    fn describer(&self) -> Box<dyn SqlSchemaDescriberBackend> {
        match self.api.connector_type() {
            "postgresql" => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::clone(
                &self.database,
            ))),
            "sqlite" => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::clone(
                &self.database,
            ))),
            "mysql" => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::clone(
                &self.database,
            ))),
            _ => unimplemented!(),
        }
    }

    pub async fn describe_database(&self) -> Result<SqlSchema, anyhow::Error> {
        let mut result = self
            .describer()
            .describe(self.connection_info().unwrap().schema_name())
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
}

pub async fn mysql_8_test_api(db_name: &str) -> TestApi {
    let url = mysql_8_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();
    let connector = mysql_migration_connector(&url).await;

    TestApi {
        connector_name: "mysql_8",
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Mysql,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Mysql,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Mysql,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Postgres,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Postgres,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Postgres,
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
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Postgres,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn sqlite_test_api(db_name: &str) -> TestApi {
    let connection_info = ConnectionInfo::from_url(&sqlite_test_url(db_name)).unwrap();
    let connector = sqlite_migration_connector(db_name).await;

    TestApi {
        connector_name: "sqlite",
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Sqlite,
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

#[derive(Debug)]
pub struct UnapplyOutput {
    pub sql_schema: SqlSchema,
    pub output: UnapplyMigrationOutput,
}
