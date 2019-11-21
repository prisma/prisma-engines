use super::{
    command_helpers::{run_infer_command, InferOutput},
    misc_helpers::{
        mysql_migration_connector, postgres_migration_connector, 
        sqlite_migration_connector, test_api,
    },
    InferAndApplyOutput
};
use migration_connector::{MigrationPersistence, MigrationStep};
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::{ApplyMigrationInput, InferMigrationStepsInput, UnapplyMigrationInput, UnapplyMigrationOutput},
};
use quaint::prelude::{ConnectionInfo, SqlFamily, Queryable};
use sql_schema_describer::*;
use std::sync::Arc;
use test_setup::*;

/// A handle to all the context needed for end-to-end testing of the migration engine across
/// connectors.
pub struct TestApi {
    sql_family: SqlFamily,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
    api: MigrationApi<sql_migration_connector::SqlMigrationConnector, sql_migration_connector::SqlMigration>,
    connection_info: Option<ConnectionInfo>,
}

impl TestApi {
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

    pub async fn apply_migration(&self, steps: Vec<MigrationStep>, migration_id: &str) -> InferAndApplyOutput {
        let input = ApplyMigrationInput {
            migration_id: migration_id.to_string(),
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
            sql_schema: self.introspect_database().await,
            migration_output,
        }
    }

    pub async fn infer_and_apply(&self, datamodel: &str) -> InferAndApplyOutput {
        let migration_id = "the-migration-id";

        self.infer_and_apply_with_migration_id(datamodel, migration_id).await
    }

    pub async fn infer_and_apply_with_migration_id(&self, datamodel: &str, migration_id: &str) -> InferAndApplyOutput {
        let input = InferMigrationStepsInput {
            migration_id: migration_id.to_string(),
            datamodel: datamodel.to_string(),
            assume_to_be_applied: Vec::new(),
        };

        let steps = self.run_infer_command(input).await.0.datamodel_steps;

        self.apply_migration(steps, migration_id).await
    }

    pub async fn execute_command<'a, C>(&self, input: &'a C::Input) -> Result<C::Output, user_facing_errors::Error>
    where
        C: migration_core::commands::MigrationCommand,
    {
        self.api
            .handle_command::<C>(input)
            .await
            .map_err(|err| self.api.render_error(err))
    }

    pub async fn run_infer_command(&self, input: InferMigrationStepsInput) -> InferOutput {
        run_infer_command(&self.api, input).await
    }

    pub async fn unapply_migration(&self) -> UnapplyOutput {
        let input = UnapplyMigrationInput {};
        let output = self.api.unapply_migration(&input).await.unwrap();

        let sql_schema = self.introspect_database().await;

        UnapplyOutput { sql_schema, output }
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.connection_info().unwrap().schema_name().to_owned(),
            inspector: self.inspector(),
            database: Arc::clone(&self.database),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
            },
        }
    }

    fn inspector(&self) -> Box<dyn SqlSchemaDescriberBackend> {
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

    async fn introspect_database(&self) -> SqlSchema {
        let mut result = self
            .inspector()
            .describe(self.connection_info().unwrap().schema_name())
            .await
            .expect("Introspection failed");

        // the presence of the _Migration table makes assertions harder. Therefore remove it from the result.
        result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();

        result
    }
}

pub async fn mysql_8_test_api() -> TestApi {
    let connection_info = ConnectionInfo::from_url(&mysql_8_url()).unwrap();
    let connector = mysql_migration_connector(&mysql_8_url()).await;

    TestApi {
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Mysql,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn mysql_test_api() -> TestApi {
    let connection_info = ConnectionInfo::from_url(&mysql_url()).unwrap();
    let connector = mysql_migration_connector(&mysql_url()).await;

    TestApi {
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Mysql,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn postgres_test_api() -> TestApi {
    let connection_info = ConnectionInfo::from_url(&postgres_url()).unwrap();
    let connector = postgres_migration_connector(&postgres_url()).await;

    TestApi {
        connection_info: Some(connection_info),
        sql_family: SqlFamily::Postgres,
        database: Arc::clone(&connector.database),
        api: test_api(connector).await,
    }
}

pub async fn sqlite_test_api() -> TestApi {
    let connection_info = ConnectionInfo::from_url(&sqlite_test_url()).unwrap();
    let connector = sqlite_migration_connector().await;

    TestApi {
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
            .expect("Introspection failed");

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
