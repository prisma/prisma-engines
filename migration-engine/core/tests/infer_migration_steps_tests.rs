#![allow(non_snake_case)]

mod test_harness;

use migration_core::commands::*;
use pretty_assertions::assert_eq;
use sql_migration_connector::PrettySqlMigrationStep;
use test_harness::*;

#[test_each_connector(log="error,[ApplyMigration{migration_id=mig0000}]=debug")]
async fn assume_to_be_applied_must_work(api: &TestApi) {
    let dm0 = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_and_apply_with_migration_id(&dm0, "mig0000").await;

    let dm1 = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;
    let input1 = InferMigrationStepsInput {
        migration_id: "mig0001".to_string(),
        assume_to_be_applied: Vec::new(),
        datamodel: dm1.to_string(),
    };
    let steps1 = api.run_infer_command(input1).await.0.datamodel_steps;
    let expected_steps_1 = create_field_step("Blog", "field1", "String");
    assert_eq!(steps1, &[expected_steps_1.clone()]);

    let dm2 = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }
        "#;
    let input2 = InferMigrationStepsInput {
        migration_id: "mig0002".to_string(),
        assume_to_be_applied: steps1,
        datamodel: dm2.to_string(),
    };
    let steps2 = api.run_infer_command(input2).await.0.datamodel_steps;

    // We are exiting watch mode, so the returned steps go back to the last non-watch migration.
    assert_eq!(
        steps2,
        &[expected_steps_1, create_field_step("Blog", "field2", "String")]
    );
}

#[test_each_connector]
async fn special_handling_of_watch_migrations(api: &TestApi) {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_and_apply_with_migration_id(&dm, "mig00").await;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    api.infer_and_apply_with_migration_id(&dm, "watch01").await;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }
        "#;

    api.infer_and_apply_with_migration_id(&dm, "watch02").await;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
                field3 Int
            }
        "#;

    let input = InferMigrationStepsInput {
        migration_id: "mig02".to_string(),
        assume_to_be_applied: Vec::new(),
        datamodel: dm.to_string(),
    };

    let steps = api.run_infer_command(input).await.0.datamodel_steps;

    assert_eq!(
        steps,
        &[
            create_field_step("Blog", "field1", "String"),
            create_field_step("Blog", "field2", "String"),
            create_field_step("Blog", "field3", "Int"),
        ]
    );
}

/// When we transition out of watch mode and `lift save` the migrations to commit the changes to
/// the migration folder, we want the database steps to be returned so they can be documented in
/// the migration README, even though they are already applied and will not be reapplied.
///
/// Relevant issue: https://github.com/prisma/lift/issues/167
#[test_each_connector]
async fn watch_migrations_must_be_returned_when_transitioning_out_of_watch_mode(api: &TestApi) {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    api.infer_and_apply_with_migration_id(&dm, "mig00").await;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

    let output = api
        .infer_and_apply_with_migration_id(&dm, "watch01")
        .await
        .migration_output;
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

    let output = api
        .infer_and_apply_with_migration_id(&dm, "watch02")
        .await
        .migration_output;
    applied_database_steps
        .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

    // We added one field/column twice, and two models, so we should have four database steps.
    assert_eq!(applied_database_steps.len(), if api.is_sqlite() { 16 } else { 4 });

    let input = InferMigrationStepsInput {
        migration_id: "mig02".to_string(),
        assume_to_be_applied: vec![],
        datamodel: dm.to_string(),
    };

    let output = api.run_infer_command(input).await;
    let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.0.database_steps).unwrap();

    let expected_steps_count = if api.is_sqlite() { 9 } else { 3 }; // one AlterTable, two CreateTables

    assert_eq!(returned_steps.len(), expected_steps_count);
}

#[test_each_connector]
async fn watch_migrations_must_be_returned_in_addition_to_regular_inferred_steps_when_transitioning_out_of_watch_mode(
    api: &TestApi,
) {
    let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

    let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

    api.infer_and_apply_with_migration_id(&dm, "mig00").await;

    let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

    let output = api
        .infer_and_apply_with_migration_id(&dm, "watch01")
        .await
        .migration_output;
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

    let output = api
        .infer_and_apply_with_migration_id(&dm, "watch02")
        .await
        .migration_output;
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

    let input = InferMigrationStepsInput {
        migration_id: "mig02".to_string(),
        assume_to_be_applied: vec![],
        datamodel: dm.to_string(),
    };

    let output = api.run_infer_command(input).await;
    let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.0.database_steps).unwrap();

    let expected_steps_count = if api.is_sqlite() {
        10
    } else {
        4 // three CreateModels, one AlterTable
    };

    assert_eq!(returned_steps.len(), expected_steps_count);
}
