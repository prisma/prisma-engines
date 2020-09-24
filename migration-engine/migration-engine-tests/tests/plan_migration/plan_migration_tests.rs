use crate::*;
use migration_core::commands::PlanMigrationOutput;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn plan_migration_with_an_up_to_date_database_returns_no_step(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm, &directory).send().await?;
    api.apply_migrations(&directory).send().await?;

    let output = api.plan_migration(&directory, dm).send().await?.into_output();
    let expected_output = PlanMigrationOutput {
        migration_steps: vec![],
        warnings: vec![],
        unexecutable_steps: vec![],
    };

    assert_eq!(output, expected_output);

    Ok(())
}

#[test_each_connector]
async fn plan_migration_with_up_to_date_db_and_pending_changes_returns_steps(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &directory).send().await?;
    api.apply_migrations(&directory).send().await?;

    let dm2 = r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings(&[])?
        .assert_unexecutable(&[])?
        .assert_steps_count(1)?;

    Ok(())
}

#[test_each_connector]
async fn plan_migration_with_not_up_to_date_db_and_pending_changes_returns_the_right_steps(
    api: &TestApi,
) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings(&[])?
        .assert_unexecutable(&[])?
        .assert_steps_count(1)?;

    Ok(())
}

#[test_each_connector(capabilities("enums"), log = "debug,sql_schema_describer=info")]
async fn plan_migration_with_past_unapplied_migrations_with_destructive_changes_does_not_warn_for_these(
    api: &TestApi,
) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
            PLAYFUL
        }
    "#;

    let directory = api.create_migrations_directory()?;
    api.create_migration("1-initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id Int @id
            name String
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings(&[
            if api.is_mysql() {
                "The migration will remove the values [PLAYFUL] on the enum `Cat_mood`. If these variants are still used in the database, the migration will fail."
            } else {
                "The migration will remove the values [PLAYFUL] on the enum `CatMood`. If these variants are still used in the database, the migration will fail." }

        .into()])?;

    api.create_migration("2-remove-value", dm2, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id Int @id
            name String
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
        }

        model Dog {
            id Int @id
            name String
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings(&[])?
        .assert_unexecutable(&[])?
        .assert_steps_count(1)?;

    Ok(())
}

// TODO: reenable MySQL when https://github.com/prisma/quaint/issues/187 is fixed.
#[test_each_connector(ignore("mysql"), log = "debug,sql_schema_describer=info")]
async fn plan_migration_returns_warnings_for_the_local_database_for_the_next_migration(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#;

    let directory = api.create_migrations_directory()?;
    api.create_migration("1-initial", dm1, &directory).send().await?;
    api.apply_migrations(&directory).send().await?;

    api.insert("Cat")
        .value("id", 1)
        .value("name", "Felix")
        .result_raw()
        .await?;

    api.insert("Dog")
        .value("id", 1)
        .value("name", "Norbert")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Dog {
            id Int @id
            name String
            fluffiness Float
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings(&["You are about to drop the `Cat` table, which is not empty (1 rows).".into()])?
        .assert_unexecutable(&[
            "Added the required column `fluffiness` to the `Dog` table without a default value. There are 1 rows in this table, it is not possible to execute this migration.".into()
        ])?
        .assert_steps_count(2)?;

    Ok(())
}

// TODO: reenable MySQL when https://github.com/prisma/quaint/issues/187 is fixed.
#[test_each_connector(capabilities("enums"), ignore("mysql"), log = "debug,sql_schema_describer=info")]
async fn plan_migration_maps_warnings_to_the_right_steps(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#;

    let directory = api.create_migrations_directory()?;
    api.create_migration("1-initial", dm1, &directory).send().await?;
    api.apply_migrations(&directory).send().await?;

    api.insert("Cat")
        .value("id", 1)
        .value("name", "Felix")
        .result_raw()
        .await?;

    api.insert("Dog")
        .value("id", 1)
        .value("name", "Norbert")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Hyena {
            id Int @id
            name String
        }

        model Cat {
            id Int @id
        }

        model Dog {
            id Int @id
            name String
            isGoodDog BetterBoolean
        }

        enum BetterBoolean {
            YES
        }
    "#;

    api.plan_migration(&directory, dm2)
        .send()
        .await?
        .assert_warnings_with_indices(&[("You are about to drop the column `name` on the `Cat` table, which still contains 1 non-null values.".into(), 1)])?
        .assert_unexecutables_with_indices(&[
            ("Added the required column `isGoodDog` to the `Dog` table without a default value. There are 1 rows in this table, it is not possible to execute this migration.".into(), 2)
        ])?;

    Ok(())
}
