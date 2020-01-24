use crate::commands::AppliedMigration;
use crate::tests::test_harness::sql::*;

#[test_each_connector]
async fn calculate_database_steps_with_infer_after_an_apply_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        type CUID = String @id @default(cuid())

        model User {
            id CUID
        }
    "#;

    let output = api
        .infer(dm1)
        .assume_to_be_applied(Some(Vec::new()))
        .migration_id(Some("mig02"))
        .send()
        .await?;

    let steps = output.datamodel_steps;

    api.infer_apply(dm1).send_assert().await?.assert_green()?;

    let dm2 = r#"
        type CUID = String @id @default(cuid())

        model User {
            id CUID
            name String
        }

        model Cat {
            id CUID
            age Int
        }
    "#;

    let output = api
        .infer(dm2)
        .assume_to_be_applied(Some(Vec::new()))
        .migration_id(Some("mig02"))
        .send()
        .await?;

    let new_steps = output.datamodel_steps.clone();

    let result = api
        .calculate_database_steps()
        .assume_to_be_applied(Some(steps))
        .steps_to_apply(Some(new_steps.clone()))
        .send()
        .await?;

    assert_eq!(result.datamodel_steps, new_steps);

    Ok(())
}

#[test_each_connector]
async fn calculate_database_steps_with_already_applied_steps_does_not_crash(api: &TestApi) -> TestResult {
    let first_migration_id = "first-migration";

    // Apply a first migration
    let output = {
        let dm1 = r#"
            type CUID = String @id @default(cuid())
    
            model User {
                id CUID
            }
        "#;

        api.infer_apply(dm1)
            .migration_id(Some(first_migration_id))
            .send_assert()
            .await?
            .assert_green()?
            .into_inner()
    };

    // Try calculating a second migration with bad assumeAppliedMigration
    {
        let dm2 = r#"
            type CUID = String @id @default(cuid())

            model User {
                id CUID
                name String @default("maggie smith")
            }

            model Cat {
                id CUID
                age Int
            }
        "#;

        let inferred_steps = api.infer(dm2).send().await?;

        let result = api
            .calculate_database_steps()
            .steps_to_apply(Some(inferred_steps.datamodel_steps))
            .assume_applied_migrations(Some(vec![AppliedMigration {
                datamodel_steps: output.datamodel_steps,
                migration_id: first_migration_id.to_owned(),
            }]))
            .send()
            .await;

        assert_eq!(result.unwrap_err().to_string(), "Failure during a migration command: Connector error. (error: Input is invalid. Migration first-migration is already applied.)");
    }

    Ok(())
}
