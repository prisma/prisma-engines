#![allow(non_snake_case)]

use super::test_harness::*;

#[test_each_connector]
async fn applying_an_already_applied_migration_must_return_an_error(api: &TestApi) -> TestResult {
    let steps = vec![
        create_model_step("Test"),
        create_field_step("Test", "id", "Int"),
        create_id_directive_step("Test", "id"),
    ];

    let migration_id = "duplicate-migration";

    let cmd = api
        .apply()
        .migration_id(Some(migration_id.to_owned()))
        .steps(Some(steps))
        .force(Some(true));

    cmd.clone().send().await?;

    assert_eq!(
        cmd.send()
            .await
            .map_err(|err| err.to_string())
            .unwrap_err(),
        "Failure during a migration command: Error in command input. (error: Invariant violation: the migration with id `duplicate-migration` has already been applied.)",
    );

    Ok(())
}
