use quaint::prelude::Queryable;
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
                    .map(|r| r.into_output())
            },
            async move {
                first_me
                    .apply_migrations(&migrations_directory_2)
                    .send()
                    .await
                    .map(|r| r.into_output())
            },
            async move {
                third_me
                    .apply_migrations(&migrations_directory_3)
                    .send()
                    .await
                    .map(|r| r.into_output())
            },
        )
    });

    let results = [&result_1, &result_2, &result_3];

    let applied_results_count = results
        .iter()
        .filter(|result| match result {
            Ok(out) => {
                out.applied_migration_names.len() == 1 && out.applied_migration_names[0] == migration_name
            }
            Err(_) => false,
        })
        .count();

    assert_eq!(applied_results_count, 1);

    // The other two engines either succeeded with nothing to apply (lock acquired
    // after the first engine was done) or, on Postgres, failed fast with an
    // advisory-lock contention error (`pg_try_advisory_lock` returned false).
    let other_results = results
        .iter()
        .filter(|result| match result {
            Ok(out) => {
                !(out.applied_migration_names.len() == 1 && out.applied_migration_names[0] == migration_name)
            }
            Err(_) => true,
        })
        .collect::<Vec<_>>();

    assert_eq!(other_results.len(), 2);

    for result in other_results {
        match result {
            Ok(out) => assert!(out.applied_migration_names.is_empty()),
            Err(err) => assert!(
                err.to_string().contains("advisory lock"),
                "unexpected error: {err}"
            ),
        }
    }
}

// Regression test for https://github.com/prisma/prisma-engines/issues/5755.
// We hold a session-level advisory lock on `72707369` from an external
// connection, then try to apply a migration. The schema engine must fail fast
// with an advisory-lock error instead of blocking (and potentially deadlocking
// against concurrent operations such as `CREATE INDEX CONCURRENTLY`).
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_advisory_lock_contention_fails_fast(mut api: TestApi) {
    let mut me = api.new_engine();
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            inBox Boolean
        }
    "#,
    );

    me.create_migration("01initial", &dm, &migrations_directory)
        .draft(true)
        .send_sync();

    // Acquire a dedicated connection (separate from the engine's connection)
    // and grab the same advisory lock the schema engine uses.
    let lock_holder = tok(quaint::single::Quaint::new(api.connection_string())).unwrap();
    tok(lock_holder.raw_cmd("SELECT pg_advisory_lock(72707369)")).unwrap();

    let err = tok(async { me.apply_migrations(&migrations_directory).send().await }).unwrap_err();

    // Release the lock so the test database teardown is not blocked.
    tok(lock_holder.raw_cmd("SELECT pg_advisory_unlock(72707369)")).unwrap();

    let msg = err.to_string();
    assert!(
        msg.contains("pg_try_advisory_lock") && msg.contains("Another instance"),
        "unexpected error: {msg}"
    );
}
