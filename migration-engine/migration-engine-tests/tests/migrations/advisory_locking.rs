use migration_core::commands::{ApplyMigrationsInput, CreateMigrationInput};
use migration_engine_tests::{multi_engine_test_api::*, TestResult};
use test_macros::test_each_connector;

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
        .generic_api()
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
        async move { second_me.generic_api().apply_migrations(&input_1).await },
        async move { first_me.generic_api().apply_migrations(&input_2).await },
        async move { third_me.generic_api().apply_migrations(&input_3).await },
    );

    let results = [&result_1, &result_2, &result_3];

    let applied_results_count = results
        .iter()
        .filter(|result| {
            let applied_migration_names = &result.as_ref().unwrap().applied_migration_names;

            applied_migration_names.len() == 1 && applied_migration_names[0] == migration_name
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
