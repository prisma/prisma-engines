use migration_core::{
    commands::{ApplyMigrationsInput, CreateMigrationInput},
    GenericApi,
};
use tempfile::TempDir;
use test_macros::test_each_connector;
use test_setup::{connectors::Tags, TestAPIArgs};

type TestResult = Result<(), anyhow::Error>;

struct TestApi {
    args: TestAPIArgs,
    source: String,
    url: String,
}

impl TestApi {
    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
    }

    async fn new_engine(&self) -> anyhow::Result<Box<dyn GenericApi>> {
        Ok(migration_core::migration_api(&self.source).await?)
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        if self.args.test_tag.contains(Tags::Postgres) {
            test_setup::create_postgres_database(&self.url.parse()?).await.unwrap();
        } else if self.args.test_tag.contains(Tags::Mysql) {
            test_setup::create_mysql_database(&self.url.parse()?).await.unwrap();
        } else if self.args.test_tag.contains(Tags::Mssql) {
            test_setup::create_mssql_database(&self.url).await.unwrap();
        }

        Ok(())
    }
}

async fn mysql_8_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mysql_8_test_config(&args.test_function_name),
        url: test_setup::mysql_8_url(&args.test_function_name),
        args,
    }
}

async fn mysql_5_6_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mysql_5_6_test_config(&args.test_function_name),
        url: test_setup::mysql_5_6_url(&args.test_function_name),
        args,
    }
}

async fn mysql_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mysql_test_config(&args.test_function_name),
        url: test_setup::mysql_url(&args.test_function_name),
        args,
    }
}

async fn mysql_mariadb_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mariadb_test_config(&args.test_function_name),
        url: test_setup::mariadb_url(&args.test_function_name),
        args,
    }
}

async fn sqlite_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::sqlite_test_config(&args.test_function_name),
        url: test_setup::sqlite_test_file(&args.test_function_name),
        args,
    }
}

async fn mssql_2019_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mssql_2019_test_config(&args.test_function_name),
        url: test_setup::mssql_2019_url(&args.test_function_name),
        args,
    }
}

async fn mssql_2017_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::mssql_2017_test_config(&args.test_function_name),
        url: test_setup::mssql_2017_url(&args.test_function_name),
        args,
    }
}

async fn postgres9_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::postgres_9_test_config(&args.test_function_name),
        url: test_setup::postgres_9_url(&args.test_function_name),
        args,
    }
}

async fn postgres_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::postgres_10_test_config(&args.test_function_name),
        url: test_setup::postgres_10_url(&args.test_function_name),
        args,
    }
}

async fn postgres11_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::postgres_11_test_config(&args.test_function_name),
        url: test_setup::postgres_11_url(&args.test_function_name),
        args,
    }
}

async fn postgres12_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::postgres_12_test_config(&args.test_function_name),
        url: test_setup::postgres_12_url(&args.test_function_name),
        args,
    }
}

async fn postgres13_test_api(args: TestAPIArgs) -> TestApi {
    TestApi {
        source: test_setup::postgres_13_test_config(&args.test_function_name),
        url: test_setup::postgres_13_url(&args.test_function_name),
        args,
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
