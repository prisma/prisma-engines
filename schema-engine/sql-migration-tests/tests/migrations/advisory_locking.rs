use sql_migration_tests::multi_engine_test_api::*;
use std::sync::Arc;
use test_macros::test_connector;

#[test_connector(exclude(CockroachDb))]
fn advisory_locking_works(mut api: TestApi) {
    let mut first_me = api.new_engine();
    let migrations_directory = Arc::new(api.create_migrations_directory());

    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            inBox Boolean
        }
    "#,
    );

    let output = first_me
        .create_migration("01initial", &dm, &migrations_directory)
        .draft(true)
        .send_sync();

    let migration_name = output.output.generated_migration_name;

    let mut second_me = api.new_engine();
    let mut third_me = api.new_engine();

    let (result_1, result_2, result_3) = tok(async {
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
