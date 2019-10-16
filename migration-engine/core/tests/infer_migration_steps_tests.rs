#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;
use datamodel::dml::*;
use migration_connector::MigrationStep;
use migration_core::commands::*;
use pretty_assertions::{assert_eq, assert_ne};
use sql_migration_connector::{PrettySqlMigrationStep, SqlFamily};
use test_harness::*;

#[test]
fn assume_to_be_applied_must_work() {
    test_each_connector(|test_setup, api| {
        let dm0 = r#"
            model Blog {
                id Int @id
            }
        "#;

        infer_and_apply_with_migration_id(test_setup, api, &dm0, "mig0000");

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
        let steps1 = run_infer_command(api, input1).0.datamodel_steps;
        assert_eq!(steps1, &[create_field_step("Blog", "field1", ScalarType::String)]);

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
        let steps2 = run_infer_command(api, input2).0.datamodel_steps;
        assert_eq!(steps2, &[create_field_step("Blog", "field2", ScalarType::String)]);
    });
}

#[test]
fn special_handling_of_watch_migrations() {
    test_each_connector(|test_setup, api| {
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

        infer_and_apply_with_migration_id(test_setup, api, &dm, "mig00");

        let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

        infer_and_apply_with_migration_id(test_setup, api, &dm, "watch01");

        let dm = r#"
            model Blog {
                id Int @id
                field1 String
                field2 String
            }
        "#;

        infer_and_apply_with_migration_id(test_setup, api, &dm, "watch02");

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

        let steps = run_infer_command(api, input).0.datamodel_steps;

        assert_eq!(
            steps,
            &[
                create_field_step("Blog", "field1", ScalarType::String),
                create_field_step("Blog", "field2", ScalarType::String),
                create_field_step("Blog", "field3", ScalarType::Int),
            ]
        );
    });
}

/// When we transition out of watch mode and `lift save` the migrations to commit the changes to
/// the migration folder, we want the database steps to be returned so they can be documented in
/// the migration README, even though they are already applied and will not be reapplied.
///
/// Relevant issue: https://github.com/prisma/lift/issues/167
#[test]
fn watch_migrations_must_be_returned_when_transitioning_out_of_watch_mode() {
    test_each_connector(|test_setup, api| {
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

        let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

        infer_and_apply_with_migration_id(test_setup, api, &dm, "mig00");

        let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

        let output = infer_and_apply_with_migration_id(test_setup, api, &dm, "watch01").migration_output;
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

        let output = infer_and_apply_with_migration_id(test_setup, api, &dm, "watch02").migration_output;
        applied_database_steps
            .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

        // We added one field/column twice, and two models, so we should have four database steps.
        assert_eq!(
            applied_database_steps.len(),
            if test_setup.sql_family == SqlFamily::Sqlite {
                16
            } else {
                4
            }
        );

        let input = InferMigrationStepsInput {
            migration_id: "mig02".to_string(),
            assume_to_be_applied: vec![],
            datamodel: dm.to_string(),
        };

        let output = run_infer_command(api, input);
        let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.0.database_steps).unwrap();

        assert_eq!(
            returned_steps,
            applied_database_steps,
            "The database migration steps returned by the InferMigrationSteps command should represent all the steps since the last non-watch migration."
        );
    });
}

#[test]
fn watch_migrations_must_be_returned_in_addition_to_regular_inferred_steps_when_transitioning_out_of_watch_mode() {
    test_each_connector(|test_setup, api| {
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;

        let mut applied_database_steps: Vec<PrettySqlMigrationStep> = Vec::new();

        infer_and_apply_with_migration_id(test_setup, api, &dm, "mig00");

        let dm = r#"
            model Blog {
                id Int @id
                field1 String
            }
        "#;

        let output = infer_and_apply_with_migration_id(test_setup, api, &dm, "watch01").migration_output;
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

        let output = infer_and_apply_with_migration_id(test_setup, api, &dm, "watch02").migration_output;
        applied_database_steps
            .extend(serde_json::from_value::<Vec<PrettySqlMigrationStep>>(output.database_steps).unwrap());

        // We added one field/column twice, and two models, so we should have four database steps.
        assert_eq!(
            applied_database_steps.len(),
            if test_setup.sql_family == SqlFamily::Sqlite {
                16
            } else {
                4
            }
        );

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
                name String?
            }
        "#;

        let input = InferMigrationStepsInput {
            migration_id: "mig02".to_string(),
            assume_to_be_applied: vec![],
            datamodel: dm.to_string(),
        };

        let output = run_infer_command(api, input);
        let returned_steps: Vec<PrettySqlMigrationStep> = serde_json::from_value(output.0.database_steps).unwrap();

        assert_eq!(
            returned_steps.len(),
            applied_database_steps.len() + 1,
            "The step for the change made after leaving watch mode should be included in the inferred steps."
        );
    });
}
