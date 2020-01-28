use migration_engine_tests::*;
use quaint::ast as quaint_ast;

#[test_each_connector]
async fn unapply_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.infer_apply(&dm1).send().await?;

    let result1 = api
        .assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_has_column("field"))?
        .into_schema();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    api.infer_apply(&dm2).send().await?;

    let result2 = api
        .assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("field"))?
        .into_schema();

    api.unapply_migration().send().await?;
    api.assert_schema().await?.assert_equals(&result1)?;

    // reapply the migration again
    api.infer_apply(&dm2).send().await?;
    api.assert_schema().await?.assert_equals(&result2).map(drop)
}

#[test_each_connector]
async fn destructive_change_checks_run_on_unapply_migration(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            field String
        }
    "#;

    api.infer_apply(dm1).send().await?;

    // Insert data.
    let query = quaint_ast::Insert::single_into(api.render_table_name("Test"))
        .value("id", "the-id")
        .value("field", "meow");

    api.database().query(query.into()).await?;

    let output = api.unapply_migration().force(Some(false)).send().await?;

    assert!(!output.warnings.is_empty());

    // Since the force flag wasn't passed, the table should still be there.
    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_has_column("id"))?;

    let rows = api.dump_table("Test").await?;

    assert_eq!(rows.len(), 1);

    Ok(())
}
