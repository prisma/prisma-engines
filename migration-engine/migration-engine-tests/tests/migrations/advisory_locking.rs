use migration_core::{
    commands::{ApplyMigrationsInput, CreateMigrationInput},
    GenericApi,
};
use tempfile::TempDir;
use test_macros::test_each_connector;
use test_setup::{connectors::Tags, TestApiArgs};

type TestResult = Result<(), anyhow::Error>;

struct TestApi {
    args: TestApiArgs,
    source: String,
    url: String,
}

impl TestApi {
    async fn new(args: TestApiArgs) -> Self {
        let connection_string = (args.url_fn)(args.test_function_name);
        let source = args.datasource_block(&connection_string);

        TestApi {
            args,
            url: connection_string,
            source,
        }
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
    }

    async fn new_engine(&self) -> anyhow::Result<Box<dyn GenericApi>> {
        Ok(migration_core::migration_api(&self.source).await?)
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        if self.args.connector_tags.contains(Tags::Postgres) {
            test_setup::create_postgres_database(&self.url.parse()?).await.unwrap();
        } else if self.args.connector_tags.contains(Tags::Mysql) {
            test_setup::create_mysql_database(&self.url.parse()?).await.unwrap();
        } else if self.args.connector_tags.contains(Tags::Mssql) {
            test_setup::create_mssql_database(&self.url).await.unwrap();
        }

        Ok(())
    }
}

#[test_each_connector]
async fn advisory_locking_works(api: &TestApi) -> TestResult {
    api.initialize().await?;

    let first_me = api.new_engine().await?;
    let migrations_directory = api.create_migrations_directory()?;
    let p = migrations_directory.path().to_string_lossy().into_owned();

    let dm = r#"
        model Cat {
            id Int @id
            inBox Boolean
        }
    "#;

    let output = first_me
        .create_migration(&CreateMigrationInput {
            migrations_directory_path: p.clone(),
            prisma_schema: dm.into(),
            migration_name: "01initial".into(),
            draft: true,
        })
        .await?;

    let migration_name = output.generated_migration_name.expect("generated no migration");

    let second_me = api.new_engine().await?;
    let third_me = api.new_engine().await?;

    let input_1 = ApplyMigrationsInput {
        migrations_directory_path: p.clone(),
    };

    let input_2 = ApplyMigrationsInput {
        migrations_directory_path: p.clone(),
    };

    let input_3 = ApplyMigrationsInput {
        migrations_directory_path: p,
    };

    let (result_1, result_2, result_3) = tokio::join!(
        // We move the engines into the async block so they get dropped when they
        // are done with the request, releasing the lock as a consequence.
        async move { second_me.apply_migrations(&input_1).await },
        async move { first_me.apply_migrations(&input_2).await },
        async move { third_me.apply_migrations(&input_3).await },
    );

    let results = [&result_1, &result_2, &result_3];

    let applied_results_count = results
        .iter()
        .filter(|result| {
            let applied_migration_names = &result.as_ref().unwrap().applied_migration_names;

            applied_migration_names.len() == 1 && &applied_migration_names[0] == &migration_name
        })
        .count();

    assert_eq!(applied_results_count, 1);

    let empty_results_count = results
        .iter()
        .filter(|result| result.as_ref().unwrap().applied_migration_names.is_empty())
        .count();

    assert_eq!(empty_results_count, 2);

    Ok(())
}
