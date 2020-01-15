#![allow(non_snake_case)]

use super::test_harness::*;
use crate::commands::AppliedMigration;
use pretty_assertions::assert_eq;

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

    assert_eq!(
        steps2,
        &[expected_steps_1, create_field_step("Blog", "field2", "String")]
    );

    Ok(())
}
#[test_each_connector]
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

    assert_eq!(response.unwrap_err().to_string(), "Failure during a migration command: Connector error. (error: Input is invalid. Migration mig01 is already applied.)");

    Ok(())
}
