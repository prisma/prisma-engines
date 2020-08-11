use migration_connector::PrettyDatabaseMigrationStep;
use migration_core::commands::AppliedMigration;
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn assume_to_be_applied_must_work(api: &TestApi) -> TestResult {
    let dm0 = r#"
        model Blog {
            id Int @id
        }
    "#;

    api.infer_apply(&dm0).migration_id(Some("mig0000")).send().await?;

    let dm1 = r#"
        model Blog {
            id Int @id
            field1 String
        }
    "#;

    let steps1 = api
        .infer(dm1)
        .migration_id(Some("mig0001"))
        .send()
        .await?
        .datamodel_steps;
    let expected_steps_1 = create_field_step("Blog", "field1", "String");
    assert_eq!(steps1, &[expected_steps_1.clone()]);

    let dm2 = r#"
        model Blog {
            id Int @id
            field1 String
            field2 String
        }
    "#;

    let steps2 = api
        .infer(dm2)
        .migration_id(Some("mig0002"))
        .assume_to_be_applied(Some(steps1.clone()))
        .send()
        .await?
        .datamodel_steps;

    assert_eq!(steps2, &[create_field_step("Blog", "field2", "String")]);

    api.infer(dm2)
        .migration_id(Some("mig0003"))
        .assume_to_be_applied(Some(steps1.into_iter().chain(steps2.into_iter()).collect()))
        .send()
        .await?;

    Ok(())
}

#[test_each_connector(log = "debug")]
async fn infer_migration_steps_validates_that_already_applied_migrations_are_not_in_assume_to_be_applied(
    api: &TestApi,
) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    let mig_1_id = "mig01";

    let response = api.infer(dm1).migration_id(Some(mig_1_id)).send().await?;
    let steps = response.datamodel_steps;

    api.apply()
        .steps(Some(steps.clone()))
        .migration_id(Some(mig_1_id))
        .send()
        .await?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int
        }
    "#;

    let response = api
        .infer(dm2)
        .migration_id(Some("mig02"))
        .assume_applied_migrations(Some(vec![AppliedMigration {
            datamodel_steps: steps,
            migration_id: mig_1_id.to_owned(),
        }]))
        .send()
        .await;

    assert!(response.unwrap_err().to_string().starts_with("Failure during a migration command: Connector error. (error: Input is invalid. Migration mig01 is already applied."));

    Ok(())
}

#[test_each_connector]
async fn special_handling_of_watch_migrations(api: &TestApi) -> TestResult {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_apply(&dm)
        .migration_id(Some("mig00".to_owned()))
        .send()
        .await?
        .assert_green()?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    api.infer_apply(&dm)
        .migration_id(Some("watch01".to_owned()))
        .send()
        .await?
        .assert_green()?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }
        "#;

    api.infer_apply(&dm)
        .migration_id(Some("watch02".to_owned()))
        .send()
        .await?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
                field3 Int
            }
        "#;

    let steps = api.infer(dm).migration_id(Some("mig02")).send().await?.datamodel_steps;

    assert_eq!(
        steps,
        &[
            create_field_step("Blog", "field1", "String"),
            create_field_step("Blog", "field2", "String"),
            create_field_step("Blog", "field3", "Int"),
        ]
    );

    Ok(())
}

/// When we transition out of watch mode and `lift save` the migrations to commit the changes to
/// the migration folder, we want the database steps to be returned so they can be documented in
/// the migration README, even though they are already applied and will not be reapplied.
///
/// Relevant issue: https://github.com/prisma/lift/issues/167
#[test_each_connector]
async fn watch_migrations_must_be_returned_when_transitioning_out_of_watch_mode(api: &TestApi) -> TestResult {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_apply(&dm).migration_id(Some("mig00")).send().await?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    let mut applied_database_steps: Vec<PrettyDatabaseMigrationStep> = Vec::new();

    let output = api
        .infer_apply(&dm)
        .migration_id(Some("watch01"))
        .send()
        .await?
        .into_inner();

    applied_database_steps.extend(output.database_steps);

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }

            model User {
                id Int @id
            }

            model Category {
                id Int @id
            }
        "#;

    let output = api
        .infer_apply(&dm)
        .migration_id(Some("watch02"))
        .send()
        .await?
        .into_inner();

    applied_database_steps.extend_from_slice(&output.database_steps);

    // applied_database_steps.extend(output.database_steps.iter().map(|s| s.clone()));

    // We added one field/column twice, and two models, so we should have four database steps.
    assert_eq!(applied_database_steps.len(), 4);

    let output = api.infer(dm).migration_id(Some("mig02")).send().await?;
    let returned_steps: Vec<PrettyDatabaseMigrationStep> = output.database_steps;

    // one AlterTable, two CreateTables
    assert_eq!(returned_steps.len(), 3);

    Ok(())
}

#[test_each_connector]
async fn watch_migrations_must_be_returned_in_addition_to_regular_inferred_steps_when_transitioning_out_of_watch_mode(
    api: &TestApi,
) -> TestResult {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    let mut applied_database_steps: Vec<PrettyDatabaseMigrationStep> = Vec::new();

    api.infer_apply(&dm)
        .migration_id(Some("mig00"))
        .send()
        .await?
        .assert_green()?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    let output = api
        .infer_apply(&dm)
        .migration_id(Some("watch01"))
        .send()
        .await?
        .into_inner();

    applied_database_steps.extend_from_slice(&output.database_steps);

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }

            model User {
                id Int @id
            }

            model Category {
                id Int @id
            }
        "#;

    let output = api
        .infer_apply(&dm)
        .migration_id(Some("watch02"))
        .send()
        .await?
        .into_inner();
    applied_database_steps.extend_from_slice(&output.database_steps);

    // We added one field/column twice, and two models, so we should have four database steps.
    assert_eq!(applied_database_steps.len(), 4);

    let dm: &'static str = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }

            model User {
                id Int @id
            }

            model Category {
                id Int @id
            }

            model Comment {
                id Int @id
            }
        "#;

    let output = api.infer(dm).migration_id(Some("mig02")).send().await?;
    let returned_steps: Vec<PrettyDatabaseMigrationStep> = output.database_steps;

    let expected_steps_count = 4; // three CreateModels, one AlterTable

    assert_eq!(returned_steps.len(), expected_steps_count);

    Ok(())
}
