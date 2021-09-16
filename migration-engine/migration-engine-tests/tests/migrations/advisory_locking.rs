use std::sync::Arc;

use migration_core::commands::CreateMigrationInput;
use migration_engine_tests::multi_engine_test_api::*;
use test_macros::test_connector;

#[test_connector]
fn advisory_locking_works(api: TestApi) {
    let first_me = api.new_engine();
    let migrations_directory = Arc::new(api.create_migrations_directory());

    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            inBox Boolean
        }
    "#,
    );

    let output = api
        .block_on(first_me.generic_api().create_migration(&CreateMigrationInput {
            migrations_directory_path: migrations_directory.path().to_string_lossy().into(),
            prisma_schema: dm,
            migration_name: "01initial".into(),
            draft: true,
        }))
        .unwrap();

    let migration_name = output.generated_migration_name.expect("generated no migration");

    let second_me = api.new_engine();
    let third_me = api.new_engine();

    let (result_1, result_2, result_3) = api.block_on(async {
        let migrations_directory_2 = migrations_directory.clone();
        let migrations_directory_3 = migrations_directory.clone();
        tokio::join!(
            // We move the engines into the async block so they get dropped when they
            // are done with the request, releasing the lock as a consequence.
            async move {
                second_me
                    .apply_migrations(&migrations_directory)
                    .send()
                    .await
                    .unwrap()
                    .into_output()
            },
            async move {
                first_me
                    .apply_migrations(&migrations_directory_2)
                    .send()
                    .await
                    .unwrap()
                    .into_output()
            },
            async move {
                third_me
                    .apply_migrations(&migrations_directory_3)
                    .send()
                    .await
                    .unwrap()
                    .into_output()
            },
        )
    });

    let results = &[&result_1, &result_2, &result_3];

    let applied_results_count = results
        .iter()
        .filter(|result| {
            let applied_migration_names = &result.applied_migration_names;

            applied_migration_names.len() == 1 && applied_migration_names[0] == migration_name
        })
        .count();

    assert_eq!(applied_results_count, 1);

    let empty_results_count = results
        .iter()
        .filter(|result| result.applied_migration_names.is_empty())
        .count();

    assert_eq!(empty_results_count, 2);
}
