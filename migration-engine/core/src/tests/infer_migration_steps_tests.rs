#![allow(non_snake_case)]

use super::test_harness::*;
use pretty_assertions::assert_eq;
use sql_migration_connector::PrettySqlMigrationStep;

#[test_each_connector]
async fn assume_to_be_applied_must_work(api: &TestApi) -> TestResult {
    let dm0 = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_apply(&dm0)
        .migration_id(Some("mig0000"))
        .send()
        .await
        .unwrap();

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
        .assume_to_be_applied(Some(steps1))
        .send()
        .await?
        .datamodel_steps;

    // We are exiting watch mode, so the returned steps go back to the last non-watch migration.
    assert_eq!(
        steps2,
        &[expected_steps_1, create_field_step("Blog", "field2", "String")]
    );

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
        .await?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    api.infer_apply(&dm)
        .migration_id(Some("watch01".to_owned()))
        .send()
        .await?;

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

    let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

    let output = api
        .infer_apply(&dm)
        .migration_id(Some("watch01".to_owned()))
        .send()
        .await?;

    applied_database_steps
        .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

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

    let output = api.infer_apply(&dm).migration_id(Some("watch02")).send().await?;

    applied_database_steps
        .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

    // We added one field/column twice, and two models, so we should have four database steps.
    assert_eq!(applied_database_steps.len(), if api.is_sqlite() { 16 } else { 4 });

    let output = api.infer(dm).migration_id(Some("mig02")).send().await?;
    let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.database_steps).unwrap();

    let expected_steps_count = if api.is_sqlite() { 9 } else { 3 }; // one AlterTable, two CreateTables

    assert_eq!(returned_steps.len(), expected_steps_count);

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

    let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

    api.infer_apply(&dm).migration_id(Some("mig00")).send().await?;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    let output = api.infer_apply(&dm).migration_id(Some("watch01")).send().await?;

    applied_database_steps
        .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

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

    let output = api.infer_apply(&dm).migration_id(Some("watch02")).send().await?;
    applied_database_steps
        .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

    // We added one field/column twice, and two models, so we should have four database steps.
    assert_eq!(applied_database_steps.len(), if api.is_sqlite() { 16 } else { 4 });

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
    let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.database_steps).unwrap();

    let expected_steps_count = if api.is_sqlite() {
        10
    } else {
        4 // three CreateModels, one AlterTable
    };

    assert_eq!(returned_steps.len(), expected_steps_count);

    Ok(())
}
