use sql_migration_tests::test_api::*;

#[test_connector]
fn reset_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm).send();

    api.assert_schema().assert_tables_count(1);

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync(None);

    api.assert_schema().assert_tables_count(0);

    api.schema_push_w_datasource(dm).send();

    api.assert_schema().assert_tables_count(1);
}

#[test_connector]
fn reset_then_apply_with_migrations_directory_works(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();
    api.create_migration("0-init", &dm, &dir).send_sync();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync(None);

    api.assert_schema().assert_tables_count(0);

    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");
}

#[test_connector]
fn reset_then_diagnostics_with_migrations_directory_works(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let dir = api.create_migrations_directory();
    api.create_migration("0-init", &dm, &dir).send_sync();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync(None);

    api.assert_schema().assert_tables_count(0);

    api.diagnose_migration_history(&dir).send_sync();
    api.evaluate_data_loss(&dir, dm).send();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), namespaces("felines", "rodents"))]
fn multi_schema_reset(mut api: TestApi) {
    let prisma_schema = format! {
        r#"
            {}

            generator js {{
                provider = "prisma-client-js"
                previewFeatures = []
            }}

            model Manul {{
                id Int @id
                @@schema("felines")
            }}

            model Capybara {{
                id Int @id
                @@schema("rodents")
            }}
        "#, api.datasource_block_with(&[("schemas", r#"["felines", "rodents"]"#)])
    };

    let migrations_dir = api.create_migrations_directory();
    api.create_migration("0-init", &prisma_schema, &migrations_dir)
        .send_sync();
    api.apply_migrations(&migrations_dir).send_sync();
    api.raw_cmd("CREATE TABLE randomTable (id INTEGER PRIMARY KEY);");

    let all_namespaces = Namespaces::from_vec(&mut vec!["felines".into(), "rodents".into(), api.schema_name()]);
    let namespaces_in_psl = Namespaces::from_vec(&mut vec!["felines".into(), "rodents".into()]);

    api.assert_schema_with_namespaces(all_namespaces.clone())
        .assert_has_table("randomtable")
        .assert_has_table("_prisma_migrations")
        .assert_has_table("Manul")
        .assert_has_table("Capybara");

    api.reset().send_sync(namespaces_in_psl);

    api.assert_schema_with_namespaces(all_namespaces)
        .assert_has_table("randomtable") // we do not want to wipe the schema from search_path
        .assert_has_no_table("_prisma_migrations")
        .assert_has_no_table("Manul")
        .assert_has_no_table("Capybara");

    // Check that we can migrate from there.
    api.schema_push(&prisma_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
}
