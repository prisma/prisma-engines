use sql_migration_tests::test_api::*;

#[test_connector]
fn evaluate_data_loss_with_an_up_to_date_database_returns_no_step(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();

    api.create_migration("initial", &dm, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    let output = api.evaluate_data_loss(&directory, dm).send().into_output();
    let expected = expect_test::expect![[r#"
        EvaluateDataLossOutput {
            migration_steps: 0,
            unexecutable_steps: [],
            warnings: [],
        }
    "#]];

    expected.assert_debug_eq(&output);
}

#[test_connector]
fn evaluate_data_loss_with_up_to_date_db_and_pending_changes_returns_steps(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#,
    );

    api.evaluate_data_loss(&directory, dm2)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_steps_count(1);
}

#[test_connector]
fn evaluate_data_loss_with_not_up_to_date_db_and_pending_changes_returns_the_right_steps(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#,
    );

    api.evaluate_data_loss(&directory, dm2)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_steps_count(1);
}

#[test_connector(capabilities(Enums))]
fn evaluate_data_loss_with_past_unapplied_migrations_with_destructive_changes_does_not_warn_for_these(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
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
    "#,
    );

    let directory = api.create_migrations_directory();
    api.create_migration("1-initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
        }
    "#,
    );

    let is_mysql = api.is_mysql();
    api.evaluate_data_loss(&directory, dm2.clone())
        .send()
        .assert_warnings(&[if is_mysql {
        "The values [PLAYFUL] on the enum `Cat_mood` will be removed. If these variants are still used in the database, this will fail."
    } else {
        "The values [PLAYFUL] on the enum `CatMood` will be removed. If these variants are still used in the database, this will fail."
    }
    .into()]);

    api.create_migration("2-remove-value", &dm2, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
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
    "#,
    );

    api.evaluate_data_loss(&directory, dm2)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_steps_count(1);
}

#[test_connector]
fn evaluate_data_loss_returns_warnings_for_the_local_database_for_the_next_migration(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();
    api.create_migration("1-initial", &dm1, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    api.insert("Cat").value("id", 1).value("name", "Felix").result_raw();
    api.insert("Dog").value("id", 1).value("name", "Norbert").result_raw();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Dog {
            id Int @id
            name String
            fluffiness Float
        }
    "#,
    );

    let warn = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Cat")
    );

    api.evaluate_data_loss(&directory, dm2)
        .send()
        .assert_warnings(&[warn.into()])
        .assert_unexecutable(&[
            "Added the required column `fluffiness` to the `Dog` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".into()
        ])
        .assert_steps_count(2);
}

#[test_connector(capabilities(Enums))]
fn evaluate_data_loss_maps_warnings_to_the_right_steps(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();
    api.create_migration("1-initial", &dm1, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    api.insert("Cat").value("id", 1).value("name", "Felix").result_raw();
    api.insert("Dog").value("id", 1).value("name", "Norbert").result_raw();

    let dm2 = api.datamodel_with_provider(
        r#"
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
    "#,
    );

    let warn = format!(
        "You are about to drop the column `name` on the `{}` table, which still contains 1 non-null values.",
        api.normalize_identifier("Cat")
    );

    let is_postgres = api.is_postgres();

    #[allow(clippy::bool_to_int_with_if)]
    api.evaluate_data_loss(&directory, dm2)
        .send()
        .assert_warnings_with_indices(&[(warn.into(), if is_postgres { 1 } else { 0 })])
        .assert_unexecutables_with_indices(&[
            ("Added the required column `isGoodDog` to the `Dog` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".into(), if is_postgres { 2 } else { 1 }),
        ]);
}

#[test_connector(capabilities(Enums))]
fn evaluate_data_loss_multi_file_maps_warnings_to_the_right_steps(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }

        model Dog {
            id Int @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();
    api.create_migration("1-initial", &dm1, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    api.insert("Cat").value("id", 1).value("name", "Felix").result_raw();
    api.insert("Dog").value("id", 1).value("name", "Norbert").result_raw();

    let schema_a = api.datamodel_with_provider(
        r#"
        model Hyena {
            id Int @id
            name String
        }

        model Cat {
            id Int @id
        }
    "#,
    );

    let schema_b = indoc::indoc! {r#"
        model Dog {
            id Int @id
            name String
            isGoodDog BetterBoolean
        }

        enum BetterBoolean {
            YES
        }
    "#};

    let warn = format!(
        "You are about to drop the column `name` on the `{}` table, which still contains 1 non-null values.",
        api.normalize_identifier("Cat")
    );

    let is_postgres = api.is_postgres();

    #[allow(clippy::bool_to_int_with_if)]
    api.evaluate_data_loss_multi_file(&directory, &[("schema_a", &schema_a), ("schema_b", schema_b)])
        .send()
        .assert_warnings_with_indices(&[(warn.into(), if is_postgres { 1 } else { 0 })])
        .assert_unexecutables_with_indices(&[
            ("Added the required column `isGoodDog` to the `Dog` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".into(), if is_postgres { 2 } else { 1 }),
        ]);
}
